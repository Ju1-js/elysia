use freya::prelude::*;
use reqwest::Url;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::{
    components::{DownloadControl, MyNewsWidget},
    context::Context,
    theme,
    debug_info,
};

use crate::layout::GamePageState;
use crate::pages::game_settings_modal::GameSettingsModal;
use super::video_state::VideoState;
use backend::{
    components::tweaks::TweakManifest,
    game_providers::hoyoplay::{get_theme_url, get_video_url},
    progress::ProgressTracker,
    settings::GlobalSettings,
};

use super::animations::{create_fade_animation, create_scale_animation};
use super::components::{BackgroundLayers, BottomRightButtons, CrossfadeState, TopRightButtons};
use super::handlers;
use super::helpers;
use super::state::{GlobalGameStateSignal, RunnerType};

/// Updates component readiness state based on the current runner type.
///
/// This helper function consolidates the logic for checking which runtime components
/// are installed and updating the game state accordingly. It checks:
/// - For Wine runners: Wine and DXVK installation
/// - For Proton runners: Proton, UMU launcher, and Steam Runtime installation
/// - For all runners: Jadeite (tweaks) installation
///
/// # Arguments
/// * `game_state` - Mutable reference to the global game state signal
/// * `runner_type` - The type of runner (Wine or Proton) to check components for
/// * `game_id` - The game ID to associate tweaks readiness with
/// * `component_svc` - The component service for checking installation status
/// * `settings_guard` - Global settings containing installation paths
async fn update_component_readiness(
    game_state: &mut GlobalGameStateSignal,
    runner_type: &RunnerType,
    game_id: &str,
    component_svc: &crate::services::ComponentService,
    settings_guard: &GlobalSettings,
) {
    match runner_type {
        RunnerType::Wine => {
            let wine_installed = component_svc.is_installed(
                settings_guard,
                backend::components::ComponentType::Wine,
            ).await;
            let dxvk_installed = component_svc.is_installed(
                settings_guard,
                backend::components::ComponentType::Dxvk,
            ).await;
            game_state.write().set_wine_ready(wine_installed);
            game_state.write().set_dxvk_ready(dxvk_installed);
            game_state.write().set_proton_ready(false);
        }
        RunnerType::Proton => {
            // Proton requires Proton itself, UMU launcher, and Steam Runtime
            let proton_installed = component_svc.is_installed(
                settings_guard,
                backend::components::ComponentType::Proton,
            ).await;
            let umu_installed = component_svc.is_installed(
                settings_guard,
                backend::components::ComponentType::Umu,
            ).await;
            let steamrt_installed = component_svc.is_installed(
                settings_guard,
                backend::components::ComponentType::SteamRuntime,
            ).await;
            let all_ready = proton_installed && umu_installed && steamrt_installed;
            game_state.write().set_proton_ready(all_ready);
            game_state.write().set_wine_ready(false);
            game_state.write().set_dxvk_ready(false);
        }
    }

    // Check if Jadeite (tweaks) is installed
    let jadeite_installed = component_svc.is_installed(
        settings_guard,
        backend::components::ComponentType::Jadeite,
    ).await;
    game_state.write().set_tweaks_ready(game_id, jadeite_installed);
}

/// Game content page displaying game information and actions
#[component]
pub fn GameContent(selected_game_id: Signal<Option<String>>) -> Element {
    let ctx = use_context::<Context>();
    let settings = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    let mut page_state = use_context::<Signal<GamePageState>>();
    let mut game_state = use_context::<GlobalGameStateSignal>();
    let video_state = use_context::<Signal<VideoState>>();
    let show_elysia_modal = use_context::<Signal<bool>>();
    let tweak_manifest = use_signal(TweakManifest::new);

    let video_fade = use_animation(create_fade_animation());

    let mut video_ready = use_signal(|| false);
    let mut video_loaded = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    let mut show_install_modal = use_signal(|| false);
    let mut show_import_modal = use_signal(|| false);

    let settings_scale_anim = use_animation(create_scale_animation());

    let mut bg_prev = use_signal(|| Option::<Url>::None);
    let mut bg_curr = use_signal(|| Option::<Url>::None);
    let bg_fade = use_animation(create_fade_animation());

    let mut theme_prev = use_signal(|| Option::<String>::None);
    let mut theme_curr = use_signal(|| Option::<String>::None);
    let theme_fade = use_animation(create_fade_animation());

    let mut news_prev_game = use_signal(|| Option::<String>::None);
    let mut news_curr_game = use_signal(|| Option::<String>::None);
    let mut news_initial_load = use_signal(|| true);
    let news_fade = use_animation(create_fade_animation());

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

    // Check if videos are disabled in settings
    let videos_disabled = settings.read().read().ok().is_some_and(|s| s.disable_videos);

    let (video_url, _theme_url) = ctx
        .api_game_basic_info
        .iter()
        .find(|info| info.game.id == game_data.id)
        .map_or((None, None), |info| {
            let video = if videos_disabled {
                None // Disable videos if setting is enabled
            } else {
                get_video_url(&info.backgrounds)
            };
            let theme = get_theme_url(&info.backgrounds);
            (video, theme)
        });

    let current_game_id = selected_game_id.read().clone();
    use_effect(use_reactive!(|current_game_id| {
        let stored_prev = page_state.peek().prev_game_id.clone();

        if stored_prev.as_ref() != current_game_id.as_ref() {
            debug_info!("Game switch detected: {:?} -> {:?}", stored_prev, current_game_id);
            
            page_state.write().prev_game_id.clone_from(&current_game_id);
            *video_ready.write() = false;
            *video_loaded.write() = false;
            *show_settings.write() = false;
        }
    }));

    let bg_url_str = parsed_bg_url.to_string();
    use_effect(use_reactive!(|bg_url_str| {
        let curr = bg_curr.peek().as_ref().map(std::string::ToString::to_string);

        if curr.as_ref() != Some(&bg_url_str) {
            debug_info!("Background URL updating to: {}", bg_url_str);
            
            if curr.is_some() {
                bg_prev.set(bg_curr.peek().clone());
            }
            
            if let Ok(new_url) = bg_url_str.parse::<Url>() {
                bg_curr.set(Some(new_url));
                bg_fade.start();
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
            f64::from(video_fade.get().read().read())
        } else {
            1.0
        }
    } else {
        0.0
    };

    let bg_fade_progress = if bg_fade.is_running() {
        f64::from(bg_fade.get().read().read())
    } else {
        1.0
    };

    let theme_fade_progress = if theme_fade.is_running() {
        f64::from(theme_fade.get().read().read())
    } else {
        1.0
    };

    let news_fade_progress = if news_fade.is_running() {
        f64::from(news_fade.get().read().read())
    } else {
        1.0
    };

    let prev_url = bg_prev.peek().clone().unwrap_or(parsed_bg_url.clone());
    let curr_url = bg_curr.peek().clone().unwrap_or(parsed_bg_url.clone());
    let prev_theme = theme_prev.peek().clone();
    let curr_theme = theme_curr.peek().clone();

    let system_status = use_context::<Signal<Option<backend::status::SystemStatus>>>();

    let progress_tracker = use_signal(ProgressTracker::new);
    let progress_tracker_instance = progress_tracker();

    let (game_progress_key, get_progress) =
        helpers::create_progress_getter(settings, game_data.id.clone(), game_data.biz.clone());

    let get_runtime_progress =
        handlers::create_runtime_progress_getter(progress_tracker_instance.clone());
    let get_tweaks_progress =
        handlers::create_tweaks_progress_getter(progress_tracker_instance.clone());

    let component_service = use_context::<Signal<Option<crate::services::ComponentService>>>();

    // Sync game state with current game's runner configuration when game changes or settings update
    let game_id_for_state = game_data.id.clone();
    let settings_watcher = settings;
    use_effect(use_reactive!(|game_id_for_state, settings_watcher| {
        let settings_arc = settings_watcher.read().clone();
        if let Ok(settings_guard) = settings_arc.read() {
            // Check component service availability
            let component_svc = component_service.read().clone();

            // Check if game is installed and update download state
            let is_game_installed = settings_guard.installed_games.contains_key(&game_id_for_state);
            game_state.write().set_download_installed(&game_id_for_state, is_game_installed);

            // Determine runner type from game settings, preferences, or defaults
            let runner_type = if let Some(game) = settings_guard.installed_games.get(&game_id_for_state) {
                // Game is installed - use its runner
                match &game.runner {
                    backend::runners::Runners::Wine(_) => RunnerType::Wine,
                    backend::runners::Runners::Native | backend::runners::Runners::Proton(_) => RunnerType::Proton,
                }
            } else if let Some(prefs) = settings_guard.game_preferences.get(&game_id_for_state) {
                // Game has preferences - use those
                match &prefs.runner {
                    backend::runners::Runners::Wine(_) => RunnerType::Wine,
                    backend::runners::Runners::Native | backend::runners::Runners::Proton(_) => RunnerType::Proton,
                }
            } else {
                // No game-specific settings - use defaults
                match &settings_guard.default_preferences.runner {
                    backend::runners::Runners::Wine(_) => RunnerType::Wine,
                    backend::runners::Runners::Native | backend::runners::Runners::Proton(_) => RunnerType::Proton,
                }
            };

            // Update component readiness based on runner type
            if let Some(ref svc) = component_svc {
                let svc_clone = svc.clone();
                let mut game_state_clone = game_state;
                let runner_type_clone = runner_type.clone();
                let game_id_clone = game_id_for_state.clone();
                let settings_clone = settings_guard.clone();
                spawn(async move {
                    update_component_readiness(
                        &mut game_state_clone,
                        &runner_type_clone,
                        &game_id_clone,
                        &svc_clone,
                        &settings_clone,
                    ).await;
                });
            }

            // Update the runner type in state
            game_state.write().set_runner_type(runner_type);
        }
    }));

    // Clone values needed by multiple memoized closures
    let progress_tracker_for_runtime = progress_tracker_instance.clone();
    let progress_tracker_for_tweaks = progress_tracker_instance.clone();
    let game_id_for_tweaks = game_data.id.clone();
    let game_id_for_download = game_data.id.clone();
    let game_biz_for_download = game_data.biz.clone();

    let on_setup_runtime = use_memo(move || {
        handlers::create_component_setup_handler(
            settings,
            progress_tracker_for_runtime.clone(),
            system_status,
            game_state,
            component_service,
        )
    });

    let on_setup_tweaks = use_memo(move || {
        handlers::create_tweaks_setup_handler(
            settings,
            game_id_for_tweaks.clone(),
            progress_tracker_for_tweaks.clone(),
            system_status,
            game_state,
            component_service,
        )
    });

    // Create game download handler
    let on_download_game = use_memo(use_reactive!(|game_id_for_download, game_biz_for_download| {
        helpers::create_game_download_handler(
            settings,
            game_id_for_download.clone(),
            game_biz_for_download.clone(),
            game_state,
            video_state,
        )
    }));

    // fixme: not only jadeite
    let game_needs_tweaks = tweak_manifest.read().needs_jadeite(&game_data.id);
    
    let tweaks_needs_update = system_status
        .read()
        .as_ref()
        .is_some_and(backend::status::SystemStatus::tweaks_need_update);

    let settings_scale = if show_settings() {
        if settings_scale_anim.is_running() {
            f64::from(settings_scale_anim.get().read().read())
        } else {
            1.0
        }
    } else {
        0.92
    };

    let game_id_for_import = game_data.id.clone();
    let game_biz_for_import = game_data.biz.clone();
    let _game_name_for_import = game_data.display.name.clone();

    rsx! {
        rect {
            key: "game-page-{game_id_str}",
            width: "fill",
            height: "fill",

            BackgroundLayers {
                key: "bg-layers-{game_id_str}",
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
                on_video_ready: move |()| video_ready.set(true),
            }

            rect {
                key: "ui-layer-{game_id_str}",
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                layer: "-1",

                if !*show_settings.read() && !*show_install_modal.read() && !*show_import_modal.read() && !*show_elysia_modal.read() {
                    TopRightButtons {}

                    BottomRightButtons {
                        game_id: game_data.id.clone(),
                        on_settings: move |()| {
                            show_settings.set(true);
                            settings_scale_anim.start();
                        },
                    }
                }

                if !*show_settings.read() && !*show_install_modal.read() && !*show_import_modal.read() && !*show_elysia_modal.read() {
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
                            width: "350",
                            height: "200",

                            if let Some(prev_game) = news_prev_game.peek().clone() {
                                rect {
                                    key: "news-widget-prev-{prev_game}",
                                    position: "absolute",
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
                                game_id: game_data.id.clone(),
                                game_name: game_data.display.name.clone(),
                                game_biz: game_data.biz.clone(),
                                game_progress_key,
                                get_game_progress: get_progress,
                                get_runtime_progress,
                                get_tweaks_progress,
                                accent_color: theme::ACCENT_PRIMARY.to_string(),
                                on_setup_runtime: *on_setup_runtime.read(),
                                on_setup_tweaks: *on_setup_tweaks.read(),
                                on_download_game: *on_download_game.read(),
                                on_show_install_modal: move |()| show_install_modal.set(true),
                                on_show_import_modal: move |()| show_import_modal.set(true),
                                game_state,
                                game_needs_tweaks,
                                tweaks_needs_update,
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
                        on_close: move |()| show_settings.set(false),
                        game_name: game_data.display.name.clone(),
                        scale: settings_scale,
                    }
                }
            }

            if *show_install_modal.read() {
                rect {
                    key: "install-modal-overlay",
                    position: "absolute",
                    position_top: "0",
                    position_left: "0",
                    width: "100%",
                    height: "100%",
                    layer: "1",

                    crate::components::download_control::InstallDirectoryModal {
                        on_close: move |()| show_install_modal.set(false),
                        on_confirm: move |path: Option<PathBuf>| {
                            if let Some(custom_path) = path {
                                // Update the games directory in settings
                                if let Ok(mut settings_guard) = settings.peek().write() {
                                    settings_guard.games_directory = custom_path;
                                    let _ = settings_guard.save();
                                }
                            }
                            // Close modal - user should click the button again to start download
                            show_install_modal.set(false);
                        },
                        settings,
                        game_name: game_data.display.name.clone(),
                        game_biz: game_data.biz.clone(),
                    }
                }
            }

            if *show_import_modal.read() {
                rect {
                    key: "import-modal-overlay",
                    position: "absolute",
                    position_top: "0",
                    position_left: "0",
                    width: "100%",
                    height: "100%",
                    layer: "1",

                    crate::components::download_control::ImportGameModal {
                        on_close: move |()| show_import_modal.set(false),
                        on_confirm: move |import_path: PathBuf| {
                            let game_id = game_id_for_import.clone();
                            let game_biz = game_biz_for_import.clone();
                            
                            // Clone for use in outer closure
                            let game_id_outer = game_id.clone();
                            let import_path_outer = import_path.clone();
                            
                            // Extract the Arc<RwLock<GlobalSettings>> from Signal before spawning
                            let settings_arc = settings.peek().clone();
                            
                            let mut game_state_clone = game_state;
                            let mut show_import_modal_clone = show_import_modal;
                            
                            spawn(async move {
                                // Use import_game_by_scan
                                let result = {
                                    if let Ok(mut settings_guard) = settings_arc.write() {
                                        let temp_dir = settings_guard.temp_directory.clone();
                                        let games_dir = settings_guard.games_directory.clone();
                                        
                                        backend::game_providers::installer::InstallerManager::import_game_by_scan(
                                            &mut settings_guard,
                                            &game_id,
                                            &game_biz,
                                            import_path.clone(),
                                            temp_dir,
                                            games_dir,
                                        )
                                    } else {
                                        Err("Failed to acquire settings lock".to_string())
                                    }
                                };
                                
                                match result {
                                    Ok(_) => {
                                        eprintln!("[GameContent] Successfully imported game from: {}", import_path_outer.display());
                                        game_state_clone.write().set_download_installed(&game_id_outer, true);
                                        show_import_modal_clone.set(false);
                                    }
                                    Err(e) => {
                                        eprintln!("[GameContent] Failed to import game: {e}");
                                    }
                                }
                            });
                        },
                        settings,
                        game_name: game_data.display.name.clone(),
                        game_biz: game_data.biz.clone(),
                    }
                }
            }
        }
    }
}
