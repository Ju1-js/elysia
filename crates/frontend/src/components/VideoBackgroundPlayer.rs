use anyhow::{Context, Result};
use dioxus::prelude::*;
use freya::prelude::*;
use skia_safe::{
    images, AlphaType, Canvas, ColorType, Data, FilterMode, ImageInfo, MipmapMode, SamplingOptions,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use backend::settings::GlobalSettings;
use ffmpeg_next as ffmpeg;

#[derive(Clone)]
struct VideoFrame {
    pixels: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

#[derive(Clone)]
struct VideoPlayerState {
    shared_frame: Arc<Mutex<Option<VideoFrame>>>,
    cancel_token: CancellationToken,
}

fn generate_cache_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("video_{:x}.webm", hasher.finish())
}

async fn get_cached_or_download_video(url: &str, cache_dir: &PathBuf) -> Result<PathBuf> {
    let videos_cache_dir = cache_dir.join("videos");
    tokio::fs::create_dir_all(&videos_cache_dir)
        .await
        .context("Failed to create videos cache directory")?;

    let cache_path = videos_cache_dir.join(generate_cache_filename(url));

    if cache_path.exists() {
        return Ok(cache_path);
    }

    let temp_path = cache_path.with_extension("tmp");
    let response = reqwest::get(url)
        .await
        .context("Failed to fetch video URL")?;

    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .context("Failed to create temp file")?;
    
    let mut stream = response.bytes_stream();
    
    use tokio_stream::StreamExt;
    use tokio::io::AsyncWriteExt;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read chunk")?;
        file.write_all(&chunk)
            .await
            .context("Failed to write chunk")?;
    }
    
    file.flush().await.context("Failed to flush file")?;
    drop(file);

    tokio::fs::rename(&temp_path, &cache_path)
        .await
        .context("Failed to rename video file to cache")?;

    Ok(cache_path)
}

fn extract_frame_pixels(frame: &ffmpeg::util::frame::Video) -> VideoFrame {
    let width = frame.width();
    let height = frame.height();
    let stride = frame.stride(0) as usize;
    let frame_data = frame.data(0);

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for row in 0..height as usize {
        let row_start = row * stride;
        let row_end = row_start + (width as usize * 4);
        pixels.extend_from_slice(&frame_data[row_start..row_end]);
    }

    VideoFrame {
        pixels: Arc::new(pixels),
        width,
        height,
    }
}

fn get_valid_frame_rate(
    stream_fps: ffmpeg::util::rational::Rational,
    decoder_fps: Option<ffmpeg::util::rational::Rational>,
) -> ffmpeg::util::rational::Rational {
    let is_valid = |fps: &ffmpeg::util::rational::Rational| fps.0 > 0 && fps.1 > 0;
    
    [Some(stream_fps), decoder_fps]
        .into_iter()
        .flatten()
        .find(is_valid)
        .unwrap_or_else(|| (30, 1).into())
}

async fn decode_and_stream_video(
    url: String,
    shared_frame: Arc<Mutex<Option<VideoFrame>>>,
    cancel_token: CancellationToken,
    ready_callback: tokio::sync::oneshot::Sender<()>,
    cache_dir: PathBuf,
    frame_ready_tx: tokio::sync::mpsc::Sender<()>,
) -> Result<()> {
    if cancel_token.is_cancelled() {
        return Ok(());
    }

    let video_path = get_cached_or_download_video(&url, &cache_dir).await?;

    if cancel_token.is_cancelled() {
        return Ok(());
    }

    tokio::task::spawn_blocking(move || -> Result<()> {
        if cancel_token.is_cancelled() {
            return Ok(());
        }

        ffmpeg::init().context("Failed to initialize FFmpeg")?;
        ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Quiet);

        let mut input_context = ffmpeg::format::input(&video_path)
            .context("Failed to open video file")?;

        if cancel_token.is_cancelled() {
            return Ok(());
        }

        let video_stream = input_context
            .streams()
            .best(ffmpeg::media::Type::Video)
            .context("No video stream found")?;

        let stream_index = video_stream.index();
        let stream_fps = video_stream.avg_frame_rate();
        let codec_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .context("Failed to create codec context")?;
        let mut decoder = codec_context
            .decoder()
            .video()
            .context("Failed to create video decoder")?;

        if cancel_token.is_cancelled() {
            drop(decoder);
            drop(input_context);
            return Ok(());
        }

        let frame_rate = get_valid_frame_rate(stream_fps, decoder.frame_rate());
        let fps_value = frame_rate.0 as f64 / frame_rate.1 as f64;
        let frame_duration = Duration::from_secs_f64(1.0 / fps_value);

        if cancel_token.is_cancelled() {
            drop(decoder);
            drop(input_context);
            return Ok(());
        }

        let mut scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::LANCZOS,
        )
        .context("Failed to create scaler")?;

        let mut decoded_frame = ffmpeg::util::frame::Video::empty();
        let mut rgba_frame = ffmpeg::util::frame::Video::empty();

        let mut first_frame_sent = false;
        let mut last_frame_time = Instant::now();
        let mut ready_callback = Some(ready_callback);

        let result = (|| -> Result<()> {
            loop {
                if cancel_token.is_cancelled() {
                    return Ok(());
                }

                let mut reached_end = true;

                for (stream, packet) in input_context.packets() {
                    if cancel_token.is_cancelled() {
                        return Ok(());
                    }

                    if stream.index() == stream_index {
                        reached_end = false;
                        
                        if let Err(e) = decoder.send_packet(&packet) {
                            if cancel_token.is_cancelled() {
                                return Ok(());
                            }
                            return Err(e).context("Failed to send packet to decoder");
                        }

                        while decoder.receive_frame(&mut decoded_frame).is_ok() {
                            if cancel_token.is_cancelled() {
                                return Ok(());
                            }

                            scaler
                                .run(&decoded_frame, &mut rgba_frame)
                                .context("Failed to scale frame")?;

                            let video_frame = extract_frame_pixels(&rgba_frame);

                            if cancel_token.is_cancelled() {
                                return Ok(());
                            }

                            if let Ok(mut frame_guard) = shared_frame.try_lock() {
                                if !cancel_token.is_cancelled() {
                                    *frame_guard = Some(video_frame);
                                    let _ = frame_ready_tx.try_send(());
                                } else {
                                    return Ok(());
                                }
                            }

                            if !first_frame_sent {
                                if let Some(callback) = ready_callback.take() {
                                    let _ = callback.send(());
                                }
                                first_frame_sent = true;
                            }

                            let elapsed = last_frame_time.elapsed();
                            if elapsed < frame_duration {
                                let sleep_duration = frame_duration - elapsed;
                                let sleep_start = Instant::now();
                                
                                while sleep_start.elapsed() < sleep_duration {
                                    if cancel_token.is_cancelled() {
                                        return Ok(());
                                    }
                                    std::thread::sleep(Duration::from_millis(5));
                                }
                            }
                            last_frame_time = Instant::now();
                        }
                    }
                }

                if reached_end {
                    if cancel_token.is_cancelled() {
                        return Ok(());
                    }
                    
                    input_context
                        .seek(0, ..)
                        .context("Failed to seek to beginning for loop")?;
                }
            }
        })();

        if let Ok(mut guard) = shared_frame.lock() {
            *guard = None;
        }
        
        drop(rgba_frame);
        drop(decoded_frame);
        drop(scaler);
        drop(decoder);
        drop(input_context);

        result
    })
    .await
    .context("Video decode task panicked")??;

    Ok(())
}

#[component]
pub fn VideoBackgroundPlayer(video_url: String, on_ready: EventHandler<()>) -> Element {
    let settings_signal = use_context::<Signal<Arc<std::sync::RwLock<GlobalSettings>>>>();
    let platform = use_platform();

    let mut player_state = use_signal(|| None::<VideoPlayerState>);
    let mut last_url = use_signal(|| String::new());

    // Only recreate state when URL actually changes
    let current_url = video_url.clone();
    if *last_url.read() != current_url {
        // Cancel and cleanup old video
        if let Some(ref state) = *player_state.peek() {
            state.cancel_token.cancel();
            
            if let Ok(mut guard) = state.shared_frame.lock() {
                *guard = None;
            }
        }
        
        // Create new player state
        player_state.set(Some(VideoPlayerState {
            shared_frame: Arc::new(Mutex::new(None)),
            cancel_token: CancellationToken::new(),
        }));
        
        last_url.set(current_url.clone());
    }

    // Start video playback
    use_effect(move || {
        let state_option = player_state.read().clone();
        
        if let Some(state) = state_option {
            let url = video_url.clone();
            let shared_frame = state.shared_frame.clone();
            let cancel_token = state.cancel_token.clone();

            let cache_dir = settings_signal
                .read()
                .read()
                .ok()
                .map(|s| s.cache_directory.clone())
                .unwrap_or_else(|| PathBuf::from("/tmp"));

            let (frame_ready_tx, mut frame_ready_rx) = tokio::sync::mpsc::channel(2);

            spawn(async move {
                while frame_ready_rx.recv().await.is_some() {
                    platform.request_animation_frame();
                }
            });

            spawn(async move {
                let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
                let cancel_check = cancel_token.clone();
                let cancel_for_error = cancel_token.clone();

                spawn(async move {
                    if let Err(err) = decode_and_stream_video(
                        url.clone(),
                        shared_frame,
                        cancel_token,
                        ready_tx,
                        cache_dir,
                        frame_ready_tx,
                    )
                    .await
                    {
                        if !cancel_for_error.is_cancelled() {
                            eprintln!("[VIDEO] Playback error: {}", err);
                        }
                    }
                });

                if ready_rx.await.is_ok() && !cancel_check.is_cancelled() {
                    on_ready.call(());
                }
            });
        }
    });

    let (canvas_ref, canvas_size) = use_node_signal();

    use_drop(move || {
        if let Some(ref state) = *player_state.peek() {
            state.cancel_token.cancel();
            
            if let Ok(mut guard) = state.shared_frame.lock() {
                *guard = None;
            }
        }
    });

    let canvas = use_canvas_with_deps((&player_state, &canvas_size), move |_| {
        let state_option = player_state.read().clone();

        move |canvas_context| {
            canvas_context.canvas.save();

            let canvas_width = canvas_context.canvas.image_info().width() as f32;
            let canvas_height = canvas_context.canvas.image_info().height() as f32;

            if let Some(ref state) = state_option {
                if let Ok(frame_guard) = state.shared_frame.try_lock() {
                    if let Some(ref frame) = *frame_guard {
                        if !state.cancel_token.is_cancelled() {
                            render_video_frame(&canvas_context.canvas, frame, canvas_width, canvas_height);
                        } else {
                            render_placeholder(&canvas_context.canvas, canvas_width, canvas_height);
                        }
                    } else {
                        render_placeholder(&canvas_context.canvas, canvas_width, canvas_height);
                    }
                } else {
                    render_placeholder(&canvas_context.canvas, canvas_width, canvas_height);
                }
            } else {
                render_placeholder(&canvas_context.canvas, canvas_width, canvas_height);
            }

            canvas_context.canvas.restore();
        }
    });

    rsx! {
        rect {
            canvas_reference: canvas.attribute(),
            reference: canvas_ref,
            width: "100%",
            height: "100%",
        }
    }
}

fn render_video_frame(canvas: &Canvas, frame: &VideoFrame, width: f32, height: f32) {
    let image_info = ImageInfo::new(
        (frame.width as i32, frame.height as i32),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );

    let pixel_data = Data::new_copy(&frame.pixels);

    if let Some(image) = images::raster_from_data(&image_info, pixel_data, (frame.width * 4) as usize) {
        let destination_rect = skia_safe::Rect::from_xywh(0.0, 0.0, width, height);
        let sampling_options = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);

        canvas.draw_image_rect_with_sampling_options(
            image,
            None,
            destination_rect,
            sampling_options,
            &paint,
        );
    }
}

fn render_placeholder(canvas: &Canvas, width: f32, height: f32) {
    let mut paint = skia_safe::Paint::default();
    paint.set_color(skia_safe::Color::from_rgb(20, 20, 20));
    let rect = skia_safe::Rect::from_xywh(0.0, 0.0, width, height);
    canvas.draw_rect(rect, &paint);
}
