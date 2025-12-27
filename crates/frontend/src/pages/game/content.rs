use std::sync::{Arc, RwLock};
use freya::prelude::*;
use reqwest::Url;

use crate::{
    components::{DownloadControl, MyNewsWidget, ComponentState},
    context::Context,
};
use backend::{
    settings::GlobalSettings,
    game_providers::hoyoplay::{get_video_url, get_theme_url},
    progress::ProgressTracker,
};
use crate::layout::GamePageState;

use super::components::{BackgroundLayers, TopRightButtons, BottomRightButtons, CrossfadeState};
use super::game_settings::GameSettingsModal;
use super::handlers;
use super::helpers;

#[component]
pub fn GameContent(
    selected_game_id: Signal<Option<String>>,
) -> Element {
    let ctx = use_context::<Context>();
    let settings = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    let mut page_state = use_context::<Signal<GamePageState>>();
    
    let video_fade = use_animation(move |_| {
        AnimNum::new(0.0, 1.0).time(700).ease(Ease::InOut).function(Function::Cubic)
    });
    
    let mut video_ready = use_signal(|| false);
    let mut video_loaded = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    
    let settings_scale_anim = use_animation(move |_| {
        AnimNum::new(0.92, 1.0).time(200).ease(Ease::Out).function(Function::Cubic)
    });
    
    let mut bg_prev = use_signal(|| Option::<Url>::None);
    let mut bg_curr = use_signal(|| Option::<Url>::None);
    let bg_fade = use_animation(move |_| {
        AnimNum::new(0.0, 1.0).time(700).ease(Ease::InOut).function(Function::Cubic)
    });
    
    let mut theme_prev = use_signal(|| Option::<String>::None);
    let mut theme_curr = use_signal(|| Option::<String>::None);
    let theme_fade = use_animation(move |_| {
        AnimNum::new(0.0, 1.0).time(700).ease(Ease::InOut).function(Function::Cubic)
    });
    
    let mut news_prev_game = use_signal(|| Option::<String>::None);
    let mut news_curr_game = use_signal(|| Option::<String>::None);
    let mut news_initial_load = use_signal(|| true);
    let news_fade = use_animation(move |_| {
        AnimNum::new(0.0, 1.0).time(700).ease(Ease::InOut).function(Function::Cubic)
    });
    
    let game_id = selected_game_id.read();
    let Some(ref game_id_str) = *game_id else {
        return rsx! { 
            rect { 
                width: "fill", 
                height: "fill",
                main_align: "center",
                cross_align: "center",
                label { 
                    font_size: "20",
                    color: "rgb(150, 150, 150)",
                    "No game selected" 
                }
            } 
        };
    };

    let Some(game_data) = ctx.api_games.iter().find(|g| &g.id == game_id_str).cloned() else {
        return rsx! { 
            rect { 
                width: "fill",
                height: "fill",
                main_align: "center",
                cross_align: "center",
                label { 
                    font_size: "20",
                    color: "rgb(200, 100, 100)",
                    "Game not found" 
                }
            }
        };
    };

    let Some(parsed_bg_url) = game_data.display.background.url.parse::<Url>().ok() else {
        return rsx! {
            rect {
                width: "fill",
                height: "fill",
                main_align: "center",
                cross_align: "center",
                label { 
                    font_size: "20",
                    color: "rgb(200, 100, 100)",
                    "Invalid background URL" 
                }
            }
        };
    };

    let (video_url, _theme_url) = ctx.api_game_basic_info
        .iter()
        .find(|info| info.game.id == game_data.id)
        .map(|info| {
            let video = get_video_url(&info.backgrounds);
            let theme = get_theme_url(&info.backgrounds);
            (video, theme)
        })
        .unwrap_or((None, None));

    let current_game_id = selected_game_id.read().clone();
    use_effect(use_reactive!(|current_game_id| {
        let stored_prev = page_state.peek().prev_game_id.clone();
        
        if stored_prev.as_ref() != current_game_id.as_ref() {
            page_state.write().prev_game_id = current_game_id.clone();
            *video_ready.write() = false;
            *video_loaded.write() = false;
            *show_settings.write() = false;
        }
    }));
        
    let bg_url_str = parsed_bg_url.to_string();
    let game_id_for_bg = game_data.id.clone();
    let api_info_bg = ctx.api_game_basic_info.clone();
    use_effect(use_reactive!(|bg_url_str, game_id_for_bg| {
        let video_url = api_info_bg
            .iter()
            .find(|info| info.game.id == game_id_for_bg)
            .and_then(|info| get_video_url(&info.backgrounds));
        
        let curr = bg_curr.peek().as_ref().map(|u| u.to_string());

        if curr.is_none() {
            if let Ok(url) = bg_url_str.parse::<Url>() {
                bg_prev.set(Some(url.clone()));
                bg_curr.set(Some(url));
            }
            return;
        }

        if curr.as_ref() != Some(&bg_url_str) {
            let is_transitioning_to_video = video_url.is_some() && !video_loaded();
            
            if !is_transitioning_to_video {
                bg_prev.set(bg_curr.peek().clone());
                if let Ok(new_url) = bg_url_str.parse::<Url>() {
                    bg_curr.set(Some(new_url));
                }
                if video_url.is_none() {
                    bg_fade.start();
                }
            }
        }
    }));

    let game_id_for_theme = game_data.id.clone();
    let api_info_theme = ctx.api_game_basic_info.clone();
    use_effect(use_reactive!(|game_id_for_theme| {
        let theme_url_str = api_info_theme
            .iter()
            .find(|info| info.game.id == game_id_for_theme)
            .and_then(|info| get_theme_url(&info.backgrounds));
        
        let curr = theme_curr.peek().clone();
        
        if curr.is_none() && theme_prev.peek().is_none() {
            theme_prev.set(theme_url_str.clone());
            theme_curr.set(theme_url_str.clone());
            return;
        }
        
        if curr != theme_url_str {
            theme_prev.set(curr);
            theme_curr.set(theme_url_str.clone());
            theme_fade.start();
        }
    }));

    let current_game_id_for_news = game_data.id.clone();
    use_effect(use_reactive!(|current_game_id_for_news| {
        let curr = news_curr_game.peek().clone();
        
        if curr.is_none() && news_prev_game.peek().is_none() {
            news_curr_game.set(Some(current_game_id_for_news.clone()));
            news_fade.start();
            news_initial_load.set(false);
            return;
        }
        
        if curr.as_ref() != Some(&current_game_id_for_news) {
            news_prev_game.set(curr);
            news_curr_game.set(Some(current_game_id_for_news.clone()));
            news_fade.start();
        }
    }));

    if video_ready() {
        video_loaded.set(true);
        video_fade.start();
        video_ready.set(false);
    }

    let video_opacity = if video_loaded() {
        if video_fade.is_running() {
            video_fade.get().read().read() as f64
        } else {
            1.0
        }
    } else {
        0.0
    };

    let bg_fade_progress = if bg_fade.is_running() {
        bg_fade.get().read().read() as f64
    } else {
        1.0
    };
    
    let theme_fade_progress = if theme_fade.is_running() {
        theme_fade.get().read().read() as f64
    } else {
        1.0
    };
    
    let news_fade_progress = if news_fade.is_running() {
        news_fade.get().read().read() as f64
    } else {
        1.0
    };
    
    let prev_url = bg_prev.peek().clone().unwrap_or(parsed_bg_url.clone());
    let curr_url = bg_curr.peek().clone().unwrap_or(parsed_bg_url.clone());
    let prev_theme = theme_prev.peek().clone();
    let curr_theme = theme_curr.peek().clone();
    
    let is_installed = helpers::is_game_installed(&settings, &game_data.id, &game_data.biz);
    let system_status = use_context::<Signal<Option<backend::status::SystemStatus>>>();
    
    let status = system_status.read();
    let (runtime_installed, runtime_needs_update, tweaks_installed, tweaks_needs_update) = if let Some(ref status) = *status {
        (
            status.runtime_ready(),
            status.runtime_needs_update(),
            status.tweaks_ready(),
            status.tweaks_need_update(),
        )
    } else {
        (false, false, false, false)
    };
    
    let runtime_active = use_signal(|| false);
    let runtime_ready = use_signal(|| runtime_installed);
    let runtime_needs_update_signal = use_signal(|| runtime_needs_update);
    let tweaks_active = use_signal(|| false);
    let tweaks_ready = use_signal(|| tweaks_installed);
    let tweaks_needs_update_signal = use_signal(|| tweaks_needs_update);
    let is_downloading = use_signal(|| false); 

    let progress_tracker = use_signal(|| ProgressTracker::new());
    let progress_tracker_instance = progress_tracker();

    let (game_progress_key, get_progress) = helpers::create_progress_getter(
        settings,
        game_data.id.clone(),
        game_data.biz.clone()
    );

    let get_runtime_progress = handlers::create_runtime_progress_getter(progress_tracker_instance.clone());
    let get_tweaks_progress = handlers::create_tweaks_progress_getter(progress_tracker_instance.clone());

    let on_setup_runtime = handlers::create_runtime_setup_handler(
        settings,
        runtime_active,
        runtime_ready,
        runtime_needs_update_signal,
        progress_tracker_instance.clone(),
        system_status,
    );

    let on_setup_tweaks = handlers::create_tweaks_setup_handler(
        settings,
        game_data.id.clone(),
        tweaks_active,
        tweaks_ready,
        tweaks_needs_update_signal,
        progress_tracker_instance.clone(),
        system_status,
    );

    let on_download_game = helpers::create_game_download_handler(
        settings,
        game_data.id.clone(),
        game_data.biz.clone(),
        is_downloading,
    );

    let settings_scale = if show_settings() {
        if settings_scale_anim.is_running() {
            settings_scale_anim.get().read().read() as f64
        } else {
            1.0
        }
    } else {
        0.92
    };

    rsx! {
        rect {
            key: "game-page-{game_id_str}",
            width: "fill",
            height: "fill",

            BackgroundLayers {
                crossfade: CrossfadeState {
                    prev_url,
                    curr_url,
                    fade_progress: bg_fade_progress,
                },
                video_url,
                video_opacity,
                theme_url: curr_theme,
                prev_theme_url: prev_theme,
                theme_fade_progress,
                static_bg_url: parsed_bg_url,
                on_video_ready: move |_| video_ready.set(true),
            }

            rect {
                key: "ui-layer-{game_id_str}",
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                layer: "-1",
                
                if !*show_settings.read() {
                    TopRightButtons {}
                    
                    BottomRightButtons {
                        on_settings: move |_| {
                            show_settings.set(true);
                            settings_scale_anim.start();
                        },
                    }
                }
                
                if !*show_settings.read() {
                    rect {
                        position: "absolute",
                        position_top: "0",
                        position_left: "92",
                        width: "550",
                        height: "100%",
                        direction: "vertical",
                        main_align: "end",
                        cross_align: "start",
                        padding: "60",
                        spacing: "16",
                        
                        rect {
                            key: "news-widget-container",
                            width: "400",
                            height: "240",
                            
                            if let Some(prev_game) = news_prev_game.peek().clone() {
                                rect {
                                    key: "news-widget-prev-{prev_game}",
                                    position: "absolute",
                                    width: "400",
                                    height: "240",
                                    opacity: "{1.0 - news_fade_progress}",
                                    offset_x: "{-50.0 * news_fade_progress}",
                                    MyNewsWidget {
                                        game_id: prev_game.clone(),
                                    }
                                }
                            }
                            
                            if let Some(curr_game) = news_curr_game.peek().clone() {
                                rect {
                                    key: "news-widget-curr-{curr_game}",
                                    position: "absolute",
                                    width: "400",
                                    height: "240",
                                    opacity: "{news_fade_progress}",
                                    offset_x: "{50.0 - (50.0 * news_fade_progress)}",
                                    MyNewsWidget {
                                        game_id: curr_game.clone(),
                                    }
                                }
                            }
                        }
                        
                        rect {
                            key: "download-control-{game_data.id}",
                            DownloadControl {
                                game_name: game_data.display.name.clone(),
                                game_progress_key,
                                installed: is_installed,
                                runtime: ComponentState {
                                    ready: runtime_ready,
                                    needs_update: *runtime_needs_update_signal.read(),
                                    active: runtime_active,
                                },
                                tweaks: ComponentState {
                                    ready: tweaks_ready,
                                    needs_update: *tweaks_needs_update_signal.read(),
                                    active: tweaks_active,
                                },
                                download_active: is_downloading,
                                get_game_progress: get_progress,
                                get_runtime_progress,
                                get_tweaks_progress,
                                accent_color: "#ff9500".to_string(),
                                on_setup_runtime,
                                on_setup_tweaks,
                                on_download_game,
                            }
                        }
                    }
                }
            }
            
            if *show_settings.read() {
                rect {
                    key: "settings-modal-overlay",
                    position: "absolute",
                    position_top: "0",
                    position_left: "0",
                    width: "100%",
                    height: "100%",
                    layer: "1",
                    
                    GameSettingsModal {
                        on_close: move |_| show_settings.set(false),
                        game_name: game_data.display.name.clone(),
                        scale: settings_scale,
                    }
                }
            }
        }
    }
}
