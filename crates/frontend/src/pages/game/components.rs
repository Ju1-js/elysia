use freya::prelude::*;
use reqwest::Url;
use std::time::{Duration, Instant};

use crate::components::{MyButton, MyNetworkImage, VideoBackgroundPlayer};
use super::video_state::{VideoState, VideoSlot};

#[derive(Clone, PartialEq)]
pub struct CrossfadeState {
    pub prev_url: Url,
    pub curr_url: Url,
    pub fade_progress: f64,
}

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
    mut video_state: VideoState,
    on_video_ready: EventHandler<()>,
) -> Element {
    let fade = use_animation(move |_| {
        AnimNum::new(0.0, 1.0)
            .time(800)
            .ease(Ease::InOut)
            .function(Function::Cubic)
    });
    
    let mut transition = use_signal(|| TransitionType::ImageToImage);
    let mut prev_url = use_signal(|| None::<String>);
    let mut pending_url = use_signal(|| None::<Option<String>>);
    let mut last_change = use_signal(|| Instant::now());
    
    let cooldown = Duration::from_millis(300);
    
    use_effect(use_reactive!(|video_url| {
        let prev = prev_url.read().clone();
        
        if prev.as_ref() == video_url.as_ref() {
            return;
        }
        
        let now = Instant::now();
        let elapsed = now.duration_since(*last_change.read());
        
        if elapsed < cooldown && video_state.is_transitioning() {
            pending_url.set(Some(video_url.clone()));
            return;
        }
        
        last_change.set(now);
        
        if video_state.is_transitioning() {
            pending_url.set(Some(video_url.clone()));
            return;
        }
        
        let trans_type = TransitionType::from_urls(prev.as_ref(), video_url.as_ref());
        transition.set(trans_type);
        
        match trans_type {
            TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                if matches!(trans_type, TransitionType::VideoToVideo) {
                    let inactive = video_state.active_slot.read().other();
                    video_state.clear_slot(inactive);
                }
                video_state.load_next_video(video_url.clone());
            }
            TransitionType::VideoToImage => {
                if !fade.is_running() {
                    video_state.start_transition();
                    fade.start();
                }
            }
            TransitionType::ImageToImage => {
                if video_url.is_some() {
                    video_state.load_next_video(video_url.clone());
                }
            }
        }
        
        prev_url.set(video_url.clone());
    }));

    let inactive_ready = video_state.inactive_ready();
    let is_transitioning = video_state.is_transitioning();
    let trans_type = *transition.read();
    let is_animating = fade.is_running();
    
    use_effect(use_reactive!(|inactive_ready, is_transitioning, trans_type, is_animating| {
        if inactive_ready && !is_transitioning {
            match trans_type {
                TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                    if !is_animating {
                        video_state.start_transition();
                        fade.start();
                    }
                }
                TransitionType::ImageToImage => {
                    let has_primary = video_state.primary_url().read().is_some();
                    let has_secondary = video_state.secondary_url().read().is_some();
                    
                    if has_primary || has_secondary {
                        if is_animating {
                            fade.start();
                        }
                        
                        transition.set(TransitionType::ImageToVideo);
                        video_state.start_transition();
                        fade.start();
                    }
                }
                _ => {}
            }
        }
    }));
    
    let is_running = fade.is_running();
    let progress = if is_running {
        fade.get().read().read() as f64
    } else {
        0.0
    };

    use_effect(use_reactive!(|is_running, progress| {
        if is_running {
            video_state.update_crossfade(progress);
        }
    }));

    use_effect(use_reactive!(|is_running| {
        if !is_running && video_state.is_transitioning() {
            let trans_type = *transition.read();
            
            match trans_type {
                TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                    video_state.swap_and_clear();
                    video_state.finish_transition();
                }
                TransitionType::VideoToImage => {
                    video_state.reset();
                }
                _ => {}
            }

            let pending = pending_url.read().clone();
            if let Some(pending_url_val) = pending {
                let now = Instant::now();
                let elapsed = now.duration_since(*last_change.read());
                
                if elapsed >= cooldown {
                    pending_url.set(None);
                    
                    let prev = prev_url.read().clone();
                    let trans_type = TransitionType::from_urls(prev.as_ref(), pending_url_val.as_ref());
                    transition.set(trans_type);
                    
                    match trans_type {
                        TransitionType::ImageToVideo | TransitionType::VideoToVideo => {
                            video_state.load_next_video(pending_url_val.clone());
                        }
                        TransitionType::VideoToImage => {
                            video_state.start_transition();
                            fade.start();
                        }
                        TransitionType::ImageToImage => {
                            if pending_url_val.is_some() {
                                video_state.load_next_video(pending_url_val.clone());
                            }
                        }
                    }
                    
                    prev_url.set(pending_url_val);
                    last_change.set(now);
                }
            }
        }
    }));

    let trans_type = *transition.read();
    let active_slot = *video_state.active_slot.read();
    let has_active_video = video_state.active_url().is_some() && video_state.active_ready();

    let (primary_opacity, secondary_opacity, video_layer_opacity) = if is_running {
        let layer_opacity = match trans_type {
            TransitionType::ImageToVideo => progress,
            TransitionType::VideoToImage => 1.0 - progress,
            TransitionType::VideoToVideo => 1.0,
            TransitionType::ImageToImage => 0.0,
        };
        
        (
            *video_state.primary_opacity().read(),
            *video_state.secondary_opacity().read(),
            layer_opacity,
        )
    } else {
        let opacity = if has_active_video { 1.0 } else { 0.0 };
        
        let (primary_op, secondary_op) = match active_slot {
            VideoSlot::Primary => (1.0, 0.0),
            VideoSlot::Secondary => (0.0, 1.0),
        };
        
        (primary_op, secondary_op, opacity)
    };
    
    let primary_url = video_state.primary_url().read().clone();
    let secondary_url = video_state.secondary_url().read().clone();
    
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
                    on_ready: move |_| {
                        video_state.mark_ready(VideoSlot::Primary);
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
                    on_ready: move |_| {
                        video_state.mark_ready(VideoSlot::Secondary);
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
                    position_top: "0",
                    position_left: "25",
                    width: "100%",
                    height: "100%",
                    layer: "1",
                    offset_x: "{-25.0 - (30.0 * theme_fade_progress)}",
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
                    position_top: "0",
                    position_left: "25",
                    width: "100%",
                    height: "100%",
                    layer: "1",
                    offset_x: "{-25.0 + (30.0 * (1.0 - theme_fade_progress))}",
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

#[component]
pub fn TopRightButtons() -> Element {
    rsx! {
        rect {
            position: "absolute",
            position_top: "32",
            position_left: "0",
            width: "100%",
            height: "0",
            direction: "horizontal",
            main_align: "end",
            padding: "0 32",
            
            MyButton {
                onpress: move |_| println!("Meow clicked!"),
                label {
                    font_size: "16",
                    font_weight: "500",
                    color: "white",
                    "Meow 🐾"
                }
            }
        }
    }
}

#[component]
pub fn BottomRightButtons(
    on_settings: EventHandler<()>,
) -> Element {
    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            direction: "horizontal",
            main_align: "end",
            cross_align: "end",
            spacing: "12",
            padding: "32",
            
            MyButton {
                onpress: move |_| println!("Game tracker clicked!"),
                rect {
                    direction: "horizontal",
                    cross_align: "center",
                    spacing: "8",
                    svg {
                        width: "20",
                        height: "20",
                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10 10-4.5 10-10S17.5 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm.5-13H11v6l5.2 3.2.8-1.3-4.5-2.7V7z"/></svg>"#
                    }
                    label {
                        font_size: "16",
                        font_weight: "500",
                        color: "white",
                        "2h 34m"
                    }
                }
            }
            
            MyButton {
                onpress: move |_| on_settings.call(()),
                rect {
                    direction: "horizontal",
                    cross_align: "center",
                    spacing: "8",
                    svg {
                        width: "20",
                        height: "20",
                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z"/></svg>"#
                    }
                    label {
                        font_size: "16",
                        font_weight: "500",
                        color: "white",
                        "Game Settings"
                    }
                }
            }
        }
    }
}
