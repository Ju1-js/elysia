use freya::prelude::*;
use reqwest::Url;
use std::time::{Duration, Instant};

use super::CrossfadeState;
use crate::components::{MyNetworkImage, VideoBackgroundPlayer};
use crate::pages::game::video_state::{VideoSlot, VideoState};
use crate::debug_info;

#[derive(Clone, Copy, PartialEq, Debug)]
enum TransitionType {
    ImageToImage,
    ImageToVideo,
    VideoToImage,
    VideoToVideo,
}

impl TransitionType {
    fn from_urls(prev: Option<&String>, curr: Option<&String>) -> Self {
        match (prev.is_some(), curr.is_some()) {
            (false, false) => Self::ImageToImage,
            (false, true) => Self::ImageToVideo,
            (true, false) => Self::VideoToImage,
            (true, true) => Self::VideoToVideo,
        }
    }
}

#[component]
pub fn BackgroundLayers(
    crossfade: CrossfadeState,
    video_url: Option<String>,
    video_opacity: f64,
    theme_url: Option<String>,
    prev_theme_url: Option<String>,
    theme_fade_progress: f64,
    static_bg_url: Url,
    on_video_ready: EventHandler<()>,
    game_biz: String,
) -> Element {
    let mut video_state = use_context::<Signal<VideoState>>();

    let fade = use_animation(move |_| {
        AnimNum::new(0.0, 1.0)
            .time(800)
            .ease(Ease::InOut)
            .function(Function::Cubic)
    });

    let mut transition = use_signal(|| TransitionType::ImageToImage);
    let mut prev_url = use_signal(|| None::<String>);
    let mut pending_url = use_signal(|| None::<Option<String>>);
    let mut last_change = use_signal(Instant::now);

    let cooldown = Duration::from_millis(300);

    // Clean up video state when this component is unmounted (when game changes)
    use_drop(move || {
        debug_info!("BackgroundLayers unmounting, resetting video state");
        video_state.write().reset();
    });

    use_effect(use_reactive!(|video_url| {
        let prev = prev_url.read().clone();

        if prev.as_ref() == video_url.as_ref() {
            return;
        }

        debug_info!("BackgroundLayers video URL change: {:?} -> {:?}", prev, video_url);

        let now = Instant::now();
        let elapsed = now.duration_since(*last_change.read());

        let mut vs = video_state.write();

        if elapsed < cooldown && vs.is_transitioning() {
            debug_info!("Video transition pending (in cooldown)");
            pending_url.set(Some(video_url.clone()));
            return;
        }

        last_change.set(now);

        if vs.is_transitioning() {
            debug_info!("Video transition pending (already transitioning)");
            pending_url.set(Some(video_url.clone()));
            return;
        }

        let trans_type = TransitionType::from_urls(prev.as_ref(), video_url.as_ref());
        debug_info!("Video transition type: {:?}", trans_type);
        transition.set(trans_type);

        match trans_type {
            TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                vs.load_next_video(video_url.clone());
            }
            TransitionType::VideoToImage => {
                if !fade.is_running() {
                    vs.start_transition();
                    fade.start();
                }
            }
            TransitionType::ImageToImage => {
                if video_url.is_some() {
                    vs.load_next_video(video_url.clone());
                }
            }
        }

        prev_url.set(video_url.clone());
    }));

    let vs = video_state.read();
    let inactive_ready = vs.inactive_ready();
    let is_transitioning = vs.is_transitioning();
    drop(vs);

    let trans_type = *transition.read();
    let is_animating = fade.is_running();

    use_effect(use_reactive!(|inactive_ready,
                              is_transitioning,
                              trans_type,
                              is_animating| {
        if inactive_ready && !is_transitioning {
            let mut vs = video_state.write();

            match trans_type {
                TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                    if !is_animating {
                        vs.start_transition();
                        fade.start();
                    }
                }
                TransitionType::ImageToImage => {
                    let has_primary = vs.primary_url().read().is_some();
                    let has_secondary = vs.secondary_url().read().is_some();

                    if has_primary || has_secondary {
                        if is_animating {
                            fade.start();
                        }

                        transition.set(TransitionType::ImageToVideo);
                        vs.start_transition();
                        fade.start();
                    }
                }
                TransitionType::VideoToImage => {}
            }
        }
    }));

    let is_running = fade.is_running();
    let progress = if is_running {
        f64::from(fade.get().read().read())
    } else {
        0.0
    };

    use_effect(use_reactive!(|is_running, progress| {
        if is_running {
            video_state.write().update_crossfade(progress);
        }
    }));

    use_effect(use_reactive!(|is_running| {
        if !is_running {
            let mut vs = video_state.write();

            if vs.is_transitioning() {
                let trans_type = *transition.read();

                match trans_type {
                    TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                        vs.swap_and_clear();
                        vs.finish_transition();
                    }
                    TransitionType::VideoToImage => {
                        vs.reset();
                    }
                    TransitionType::ImageToImage => {}
                }

                drop(vs);

                let pending = pending_url.read().clone();
                if let Some(pending_url_val) = pending {
                    let now = Instant::now();
                    let elapsed = now.duration_since(*last_change.read());

                    if elapsed >= cooldown {
                        pending_url.set(None);

                        let prev = prev_url.read().clone();
                        let trans_type =
                            TransitionType::from_urls(prev.as_ref(), pending_url_val.as_ref());
                        transition.set(trans_type);

                        let mut vs = video_state.write();

                        match trans_type {
                            TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                                vs.load_next_video(pending_url_val.clone());
                            }
                            TransitionType::VideoToImage => {
                                vs.start_transition();
                                fade.start();
                            }
                            TransitionType::ImageToImage => {
                                if pending_url_val.is_some() {
                                    vs.load_next_video(pending_url_val.clone());
                                }
                            }
                        }

                        prev_url.set(pending_url_val);
                        last_change.set(now);
                    }
                }
            }
        }
    }));

    let vs = video_state.read();
    let trans_type = *transition.read();
    let active_slot = *vs.active_slot.read();
    let has_active_video = vs.active_url().is_some() && vs.active_ready();
    let is_paused = vs.is_paused();

    let (primary_opacity, secondary_opacity, video_layer_opacity) = if is_running {
        let layer_opacity = match trans_type {
            TransitionType::ImageToVideo => progress,
            TransitionType::VideoToImage => 1.0 - progress,
            TransitionType::VideoToVideo => 1.0,
            TransitionType::ImageToImage => 0.0,
        };

        (
            *vs.primary_opacity().read(),
            *vs.secondary_opacity().read(),
            layer_opacity,
        )
    } else {
        let opacity = if has_active_video && !is_paused { 1.0 } else { 0.0 };

        let (primary_op, secondary_op) = match active_slot {
            VideoSlot::Primary => (1.0, 0.0),
            VideoSlot::Secondary => (0.0, 1.0),
        };

        (primary_op, secondary_op, opacity)
    };

    let primary_url = if is_paused { None } else { vs.primary_url().read().clone() };
    let secondary_url = if is_paused { None } else { vs.secondary_url().read().clone() };
    drop(vs);

    // Compute theme overlay styling based on game type
    let is_endfield = game_biz == "endfield";
    let (theme_width, theme_height, theme_main_align, theme_cross_align, theme_position_top, theme_position_left, theme_offset_base) = if is_endfield {
        ("46%", "46%", "center", "start", "125", "370", -280.0)
    } else {
        ("100%", "100%", "end", "end", "0", "25", -25.0)
    };

    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            cross_align: "start",
            main_align: "start",
            layer: "5",
            opacity: "{1.0 - video_layer_opacity}",
            MyNetworkImage {
                url: static_bg_url.clone(),
                sampling: "trilinear",
            }
        }

        rect {
            position: "absolute",
            position_top: "0",
            position_left: "20",
            width: "100%",
            height: "100%",
            cross_align: "end",
            main_align: "end",
            layer: "4",
            offset_x: "-20",
            opacity: "{1.0 - video_layer_opacity}",
            MyNetworkImage {
                url: static_bg_url,
                sampling: "trilinear",
            }
        }

        rect {
            position: "absolute",
            position_top: "0",
            position_left: "20",
            width: "100%",
            height: "100%",
            cross_align: "end",
            main_align: "end",
            layer: "3",
            opacity: "{(1.0 - crossfade.fade_progress) * (1.0 - video_layer_opacity)}",
            offset_x: "-20",
            MyNetworkImage {
                url: crossfade.prev_url,
                sampling: "trilinear",
            }
        }

        rect {
            position: "absolute",
            position_top: "0",
            position_left: "20",
            cross_align: "end",
            main_align: "end",
            width: "100%",
            height: "100%",
            layer: "3",
            opacity: "{crossfade.fade_progress * (1.0 - video_layer_opacity)}",
            offset_x: "{-20.0 * crossfade.fade_progress}",
            MyNetworkImage {
                url: crossfade.curr_url,
                sampling: "trilinear",
            }
        }

        if let Some(video_primary) = primary_url {
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "20",
                cross_align: "end",
                main_align: "end",
                width: "100%",
                height: "100%",
                layer: "2",
                opacity: "{primary_opacity * video_layer_opacity}",
                offset_x: "-20",
                VideoBackgroundPlayer {
                    key: "{video_primary}",
                    video_url: video_primary,
                    on_ready: move |()| {
                        let mut vs = video_state.write();
                        vs.mark_ready(VideoSlot::Primary);
                        if active_slot == VideoSlot::Primary {
                            on_video_ready.call(());
                        }
                    },
                }
            }
        }

        if let Some(video_secondary) = secondary_url {
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "20",
                cross_align: "end",
                main_align: "end",
                width: "100%",
                height: "100%",
                layer: "2",
                opacity: "{secondary_opacity * video_layer_opacity}",
                offset_x: "-20",
                VideoBackgroundPlayer {
                    key: "{video_secondary}",
                    video_url: video_secondary,
                    on_ready: move |()| {
                        let mut vs = video_state.write();
                        vs.mark_ready(VideoSlot::Secondary);
                        if active_slot == VideoSlot::Secondary {
                            on_video_ready.call(());
                        }
                    },
                }
            }
        }

        if let Some(prev_theme) = prev_theme_url {
            if let Ok(prev_theme_parsed) = prev_theme.parse::<Url>() {
                rect {
                    position: "absolute",
                    position_top: "{theme_position_top}",
                    position_left: "{theme_position_left}",
                    width: "{theme_width}",
                    height: "{theme_height}",
                    main_align: "{theme_main_align}",
                    cross_align: "{theme_cross_align}",
                    layer: "1",
                    offset_x: "{theme_offset_base - (30.0 * theme_fade_progress)}",
                    opacity: "{1.0 - theme_fade_progress}",
                    MyNetworkImage {
                        url: prev_theme_parsed,
                        sampling: "trilinear",
                    }
                }
            }
        }

        if let Some(theme) = theme_url {
            if let Ok(theme_parsed) = theme.parse::<Url>() {
                rect {
                    position: "absolute",
                    position_top: "{theme_position_top}",
                    position_left: "{theme_position_left}",
                    width: "{theme_width}",
                    height: "{theme_height}",
                    main_align: "{theme_main_align}",
                    cross_align: "{theme_cross_align}",
                    layer: "1",
                    offset_x: "{theme_offset_base + (30.0 * (1.0 - theme_fade_progress))}",
                    opacity: "{theme_fade_progress}",
                    MyNetworkImage {
                        url: theme_parsed,
                        sampling: "trilinear",
                    }
                }
            }
        }
    }
}
