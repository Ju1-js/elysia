use dioxus::prelude::*;
use freya::prelude::*;
use skia_safe::{images, ImageInfo, ColorType, AlphaType, Data, SamplingOptions, FilterMode, MipmapMode};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use ffmpeg_next as ffmpeg;

// State for holding the current video frame as raw pixels
#[derive(Clone)]
struct VideoFrame {
    pixels: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

#[component]
pub fn VideoBackgroundPlayer(video_url: String) -> Element {
    let platform = use_platform();
    let current_frame = use_signal(|| Arc::new(Mutex::new(None::<VideoFrame>)));
    let should_stop = use_signal(|| Arc::new(AtomicBool::new(false)));
    
    // Start video decoder in background - runs once per component instance
    use_effect(move || {
        let url = video_url.clone();
        let frame_store = current_frame.read().clone();
        let stop_flag = should_stop.read().clone();
        
        // Reset stop flag for new video
        stop_flag.store(false, Ordering::Relaxed);
        
        spawn(async move {
            if let Err(e) = stream_video_frames(url.clone(), frame_store, stop_flag).await {
                eprintln!("Video playback error: {}", e);
            }
        });
    });

    // Cleanup on unmount - this is the key to stopping old videos
    use_drop(move || {
        should_stop.read().store(true, Ordering::Relaxed);
    });
    
    let (reference, size) = use_node_signal();
    
    // Ticker to drive frame updates
    use_hook(|| {
        let mut ticker = platform.new_ticker();
        spawn(async move {
            loop {
                ticker.tick().await;
                platform.invalidate_drawing_area(size.peek().area);
                platform.request_animation_frame();
            }
        });
    });
    
    // Canvas rendering with high-quality scaling
    let canvas = use_canvas(move || {
        let frame_store = current_frame.read().clone();
        move |ctx| {
            ctx.canvas.save();
            
            let frame_guard = frame_store.lock().unwrap();
            
            if let Some(frame) = frame_guard.as_ref() {
                // Create Skia image from raw RGBA pixels
                let info = ImageInfo::new(
                    (frame.width as i32, frame.height as i32),
                    ColorType::RGBA8888,
                    AlphaType::Premul,
                    None,
                );
                
                let data = Data::new_copy(&frame.pixels);
                
                if let Some(image) = images::raster_from_data(
                    &info,
                    data,
                    (frame.width * 4) as usize,
                ) {
                    // Draw the image to fill the canvas area with high-quality scaling
                    let dest_rect = skia_safe::Rect::from_xywh(
                        ctx.area.min_x(),
                        ctx.area.min_y(),
                        ctx.area.width(),
                        ctx.area.height(),
                    );
                    
                    // Use high-quality sampling for better upscaling
                    let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
                    
                    let mut paint = skia_safe::Paint::default();
                    paint.set_anti_alias(true);
                    
                    ctx.canvas.draw_image_rect_with_sampling_options(
                        image,
                        None, // src rect (None = full image)
                        dest_rect,
                        sampling,
                        &paint,
                    );
                }
            } else {
                // Draw placeholder while loading
                let mut paint = skia_safe::Paint::default();
                paint.set_color(skia_safe::Color::from_rgb(20, 20, 20));
                ctx.canvas.draw_rect(
                    skia_safe::Rect::from_xywh(
                        ctx.area.min_x(),
                        ctx.area.min_y(),
                        ctx.area.width(),
                        ctx.area.height()
                    ),
                    &paint,
                );
            }
            
            ctx.canvas.restore();
        }
    });
    
    rsx! {
        rect {
            canvas_reference: canvas.attribute(),
            reference,
            width: "100%",
            height: "100%",
        }
    }
}

// Background task that decodes video and updates the frame store
async fn stream_video_frames(
    url: String,
    frame_store: Arc<Mutex<Option<VideoFrame>>>,
    should_stop: Arc<AtomicBool>,
) -> Result<(), String> {
    // Download video first
    let video_path = download_video(&url).await?;
    
    // FFmpeg decoding in blocking task
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // Suppress FFmpeg logs
        ffmpeg::init().map_err(|e| e.to_string())?;
        ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Quiet);
        
        let mut ictx = ffmpeg::format::input(&video_path).map_err(|e| e.to_string())?;
        
        // Find video stream (ignore audio)
        let video_stream = ictx.streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or("No video stream found")?;
        let video_stream_index = video_stream.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|e| e.to_string())?;
        let mut decoder = context_decoder.decoder().video().map_err(|e| e.to_string())?;
        
        // Use original resolution for better quality
        let target_width = decoder.width();
        let target_height = decoder.height();
        
        let mut scaler = ffmpeg::software::scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            target_width,
            target_height,
            ffmpeg::software::scaling::Flags::LANCZOS, // High-quality scaling
        ).map_err(|e| e.to_string())?;
        
        let mut decoded_frame = ffmpeg::util::frame::Video::empty();
        let mut rgb_frame = ffmpeg::util::frame::Video::empty();
        
        // Calculate frame delay for FPS
        let frame_rate = decoder.frame_rate().unwrap_or((30, 1).into());
        let frame_duration = std::time::Duration::from_secs_f64(
            frame_rate.1 as f64 / frame_rate.0 as f64
        );
        
        'outer: loop {
            // Check if we should stop
            if should_stop.load(Ordering::Relaxed) {
                break;
            }
            
            let mut packets_exhausted = true;
            
            for (stream, packet) in ictx.packets() {
                // Check stop flag during packet processing
                if should_stop.load(Ordering::Relaxed) {
                    break 'outer;
                }
                
                // Only process video packets, ignore audio
                if stream.index() == video_stream_index {
                    packets_exhausted = false;
                    decoder.send_packet(&packet).map_err(|e| e.to_string())?;
                    
                    while decoder.receive_frame(&mut decoded_frame).is_ok() {
                        // Check stop flag during frame decoding
                        if should_stop.load(Ordering::Relaxed) {
                            break 'outer;
                        }
                        
                        // Convert to RGBA
                        scaler.run(&decoded_frame, &mut rgb_frame).map_err(|e| e.to_string())?;
                        
                        // Copy pixel data
                        let width = rgb_frame.width();
                        let height = rgb_frame.height();
                        let stride = rgb_frame.stride(0);
                        let data = rgb_frame.data(0);
                        
                        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                        for y in 0..height {
                            let row_start = (y * stride as u32) as usize;
                            let row_end = row_start + (width * 4) as usize;
                            pixels.extend_from_slice(&data[row_start..row_end]);
                        }
                        
                        // Update frame store
                        *frame_store.lock().unwrap() = Some(VideoFrame {
                            pixels: Arc::new(pixels),
                            width,
                            height,
                        });
                        
                        // Frame pacing
                        std::thread::sleep(frame_duration);
                    }
                }
            }
            
            // Loop video - seek to beginning
            if packets_exhausted {
                ictx.seek(0, ..).map_err(|e| e.to_string())?;
            }
        }
        
        // Clean up downloaded file
        let _ = std::fs::remove_file(&video_path);
        
        Ok(())
    })
    .await
    .map_err(|e| format!("Join error: {}", e))??;
    
    Ok(())
}

async fn download_video(url: &str) -> Result<std::path::PathBuf, String> {
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    
    let path = std::path::PathBuf::from(format!(
        "/tmp/video_{}.webm",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs()
    ));
    
    tokio::fs::write(&path, bytes).await.map_err(|e| e.to_string())?;
    Ok(path)
}