#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod components;
mod context;
mod debug;
mod layout;
mod pages;
mod services;
mod theme;

use std::path::PathBuf;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use freya::prelude::*;
use reqwest::Url;

use crate::context::Context;
use crate::pages::state::GlobalGameState;
use backend::{
    components::tweaks::TweakManifest,
    game_providers::hoyoplay::{get_game_content, get_games, get_video_url},
    runners::Runners,
    settings::{GlobalSettings, InstalledGame},
    status::SystemStatus,
};

/// Application entry point
fn main() {
    launch_cfg(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_size(1280.0, 720.0)
                .with_decorations(true)
                .with_transparency(true)
                .with_title("Elysia")
                .with_window_attributes(|attributes| attributes.with_resizable(true)),
        ),
    );
}

/// Main application component that initializes settings, context, and global state
#[allow(clippy::too_many_lines)]
fn app() -> Element {
    let mut settings = use_signal(|| {
        Arc::new(RwLock::new(if let Ok(mut settings) = GlobalSettings::load() {
            settings.validate();
            settings
        } else {
            let mut settings = GlobalSettings::default();
            settings.validate();
            if let Err(err) = settings.save() {
                debug_error!("Failed to save settings: {}", err);
            }
            settings
        }))
    });

    use_hook(|| {
        let settings = settings.write();
        if let Ok(mut settings_data) = settings.clone().write() {
            // Use default preferences for runner and runtime components
            let runner = settings_data.default_preferences.runner.clone();
            let runtime_components = settings_data.default_preferences.runtime_components.clone();
            
            settings_data.installed_games.insert(
                "U5hbdsT9W7".to_string(),
                InstalledGame {
                    id: "U5hbdsT9W7".to_string(),
                    biz_name: "nap_global".to_string(),
                    executable_path: PathBuf::from("ZenlessZoneZero.exe"),
                    install_path: PathBuf::from("/mnt/SSD/Games/Zenless Zone Zero/"),
                    runner,
                    runtime_components,
                    ..Default::default()
                },
            );
        }
    });

    {
        to_owned![settings];
        use_drop(move || {
            if let Ok(settings_data) = settings().read()
                && let Err(err) = settings_data.save() {
                debug_error!("Failed to save settings: {}", err);
            }
        });
    }
    {
        to_owned![settings];
        use_context_provider(move || settings);
    }

    use_init_theme(|| DARK_THEME);

    let context = use_resource(move || async move {
        let settings_lock = settings.read();
        let settings_data = settings_lock.read().unwrap().clone();

        let mut api_games = get_games(&settings_data)
            .await
            .map_err(|err| err.clone())
            .map_or_else(|err| {
                debug_error!("Failed to load games from api: {err}");
                Vec::new()
            }, |response| response.games);

        let endfield_games = backend::game_providers::endfield::get_games()
            .await
            .map_or_else(|err| {
                debug_error!("Failed to load endfield games: {err}");
                Vec::new()
            }, |response| response.games);

        api_games.extend(endfield_games);

        let mut api_news = HashMap::new();

        for game in &api_games {
            let game_id = game.id.clone();
            let game_biz = game.biz.clone();

            let response = if game_biz == "endfield" {
                backend::game_providers::endfield::get_game_content(&game_id).await
            } else {
                get_game_content(&settings_data, &game_id).await
            };

            match response {
                Ok(content_response) => {
                    api_news.insert(game_id, content_response.content);
                }
                Err(err) => {
                    debug_error!("Failed to load game content: {err}");
                }
            }
        }

        let api_game_basic_info =
            backend::game_providers::hoyoplay::get_all_game_basic_info(&settings_data, None)
                .await
                .map_or_else(|err| {
                    debug_error!("Failed to load game basic info: {err}");
                    Vec::new()
                }, |info| info.game_info_list);

        Context {
            api_games,
            api_news,
            api_game_basic_info,
        }
    });

    use_context_provider(move || context);

    // Initialize ComponentService once and share across the entire app via context
    let mut component_service = use_signal(|| None::<services::ComponentService>);
    use_context_provider(|| component_service);

    use_effect(move || {
        if component_service.read().is_none() {
            spawn(async move {
                let service = services::ComponentService::new().await;
                component_service.set(Some(service));
            });
        }
    });

    // System status tracks runtime component readiness
    let mut system_status = use_signal(|| None::<SystemStatus>);
    use_context_provider(|| system_status);

    // GlobalGameState is the single source of truth for game state management
    let game_state = use_signal(GlobalGameState::new);
    use_context_provider(|| game_state);

    let init_settings = settings;
    let mut game_state_initialized = use_signal(|| false);

    use_effect(move || {
        if !game_state_initialized() {
            let settings_clone = init_settings.read().clone();
            let mut state_signal = game_state;

            spawn(async move {
                // Check system status
                let (sys_status_check_data, runner_type, configured_wine_version, configured_proton_version, configured_dxvk_version, components_dir) = {
                    let Ok(settings_guard) = settings_clone.read() else {
                        debug_error!("Failed to read settings for initialization");
                        return;
                    };
                    
                    // Clone all data we need before the await
                    let settings_data = settings_guard.clone();
                    
                    // Read the runner type and configured versions from default_preferences
                    let default_runner = &settings_guard.default_preferences.runner;
                    let (runner_type, configured_wine_version, configured_proton_version) = match default_runner {
                        Runners::Wine(wine) => (
                            crate::pages::game::state::RunnerType::Wine,
                            Some(wine.version.clone()),
                            None,
                        ),
                        Runners::Proton(proton) => (
                            crate::pages::game::state::RunnerType::Proton,
                            None,
                            Some(proton.version.clone()),
                        ),
                        Runners::Native => {
                            // Native shouldn't be in preferences, default to Proton
                            debug_error!("Native runner found in default_preferences, defaulting to Proton");
                            (crate::pages::game::state::RunnerType::Proton, None, None)
                        }
                    };
                    
                    // Get configured DXVK version
                    let configured_dxvk_version = settings_guard
                        .default_preferences
                        .runtime_components
                        .iter()
                        .find_map(|component| {
                            if let backend::settings::RuntimeComponents::Dxvk(version) = component {
                                Some(version.clone())
                            } else {
                                None
                            }
                        });
                    
                    // Clone components directory for checking if specific versions exist
                    let components_dir = settings_guard.components_directory.clone();
                    
                    (settings_data, runner_type, configured_wine_version, configured_proton_version, configured_dxvk_version, components_dir)
                };

                let sys_status = SystemStatus::check(&sys_status_check_data).await;
                system_status.set(Some(sys_status.clone()));

                // Set the runner type in GlobalGameState first
                state_signal.write().set_runner_type(runner_type.clone());

                // Check runtime components based on the selected runner
                let runtime_ready = match runner_type {
                    crate::pages::game::state::RunnerType::Wine => {
                        // For Wine, check if the configured Wine version is installed
                        // System Wine doesn't need to be "installed" via download
                        let wine_ready = if configured_wine_version.as_deref() == Some("system") {
                            true
                        } else if let Some(ref wine_ver) = configured_wine_version {
                            // Check if the specific configured Wine version directory exists
                            components_dir.join("wine").join(wine_ver).exists()
                        } else {
                            false
                        };
                        
                        // Check if configured DXVK version is installed
                        let dxvk_ready = if let Some(ref dxvk_ver) = configured_dxvk_version {
                            // Check if the specific configured DXVK version directory exists
                            components_dir.join("dxvk").join(dxvk_ver).exists()
                        } else {
                            // No DXVK configured, consider it ready
                            true
                        };
                        
                        debug!("Wine version - configured: {:?}, installed: {:?}, ready: {}", 
                            configured_wine_version, sys_status.wine.installed_version, wine_ready);
                        debug!("DXVK version - configured: {:?}, installed: {:?}, ready: {}", 
                            configured_dxvk_version, sys_status.dxvk.installed_version, dxvk_ready);
                        
                        state_signal.write().set_wine_ready(wine_ready);
                        state_signal.write().set_dxvk_ready(dxvk_ready);
                        wine_ready && dxvk_ready
                    }
                    crate::pages::game::state::RunnerType::Proton => {
                        // For Proton, check if the configured Proton version is installed
                        // and that UMU and SteamRT are also installed
                        let proton_version_ready = if let Some(ref proton_ver) = configured_proton_version {
                            // Check if the specific configured Proton version directory exists
                            components_dir.join("proton").join(proton_ver).exists()
                        } else {
                            false
                        };
                        let proton_ready = proton_version_ready && sys_status.umu.installed && sys_status.steamrt.installed;
                        
                        debug!("Proton version - configured: {:?}, installed: {:?}, version ready: {}, runtime ready: {}", 
                            configured_proton_version, sys_status.proton.installed_version, proton_version_ready, proton_ready);
                        
                        state_signal.write().set_proton_ready(proton_ready);
                        proton_ready
                    }
                };
                
                debug!("Setting runtime_ready to: {} for runner type: {:?}", runtime_ready, runner_type);

                let manifest = TweakManifest::new();
                let jadeite_ready = check_jadeite_installed(&sys_status);

                if let Some(ctx) = context.read_unchecked().as_ref() {
                    for game in &ctx.api_games {
                        if manifest.needs_jadeite(&game.id) {
                            state_signal
                                .write()
                                .set_tweaks_ready(&game.id, jadeite_ready);
                        }

                        if let Ok(settings) = settings_clone.read() {
                            let installed = settings.installed_games.contains_key(&game.id);
                            state_signal
                                .write()
                                .set_download_installed(&game.id, installed);
                        }
                    }
                }

                game_state_initialized.set(true);
            });
        }
    });

    // Note: Each Game page initializes its own GlobalGameState instance from default_preferences
    // There's no need for a reactive effect here since game pages handle their own state

    let preload_settings = settings;
    let mut has_preloaded = use_signal(|| false);

    use_effect(move || {
        if !has_preloaded()
            && let Some(context_data) = context.read_unchecked().as_ref() {
            let games = context_data.api_games.clone();
            let basic_info = context_data.api_game_basic_info.clone();
            let api_news = context_data.api_news.clone();
            let settings_signal = preload_settings;

            spawn(async move {
                let settings_lock = settings_signal.read();
                if let Ok(settings_data) = settings_lock.read() {
                    let cache_path = settings_data.cache_directory.display().to_string();

                    let mut image_urls: Vec<Url> = Vec::new();

                    for game in &games {
                        if let Ok(url) = game.display.background.url.parse() {
                            image_urls.push(url);
                        }
                    }

                    for game in &games {
                        if let Ok(url) = game.display.icon.url.parse() {
                            image_urls.push(url);
                        }
                    }

                    for content in api_news.values() {
                        for banner in &content.banners {
                            if let Ok(url) = banner.image.url.parse() {
                                image_urls.push(url);
                            }
                        }
                    }

                    components::preload_images(image_urls, cache_path.clone());

                    let mut video_urls: Vec<Url> = Vec::new();

                    for info in &basic_info {
                        if let Some(video_url) = get_video_url(&info.backgrounds)
                            && let Ok(url) = video_url.parse()
                        {
                            video_urls.push(url);
                        }
                    }

                    if !video_urls.is_empty() {
                        components::preload_videos(
                            video_urls,
                            settings_data.cache_directory.clone(),
                            3,
                        );
                    }
                }
            });

            has_preloaded.set(true);
        }
    });

    layout::app()
}

/// Check if Jadeite (tweaks component) is installed
fn check_jadeite_installed(status: &SystemStatus) -> bool {
    status.tweaks_ready()
}
