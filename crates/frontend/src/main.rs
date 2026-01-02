#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod components;
mod context;
mod layout;
mod pages;

use std::path::PathBuf;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use freya::prelude::*;
use reqwest::Url;

use crate::context::Context;
use backend::{
    game_providers::hoyoplay::{get_game_content, get_games, get_video_url},
    runners::{Runners, Wine},
    settings::{GlobalSettings, InstalledGame, RuntimeComponents},
    status::SystemStatus,
    components::tweaks::TweakManifest,
};
use crate::pages::state::GlobalGameState;

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

fn app() -> Element {
    let mut settings = use_signal(|| {
        Arc::new(RwLock::new(match GlobalSettings::load() {
            Ok(mut settings) => {
                settings.validate();
                settings
            }
            Err(_) => {
                let mut settings = GlobalSettings::default();
                settings.validate();
                if let Err(err) = settings.save() {
                    eprintln!("Failed to save settings: {}", err);
                }
                settings
            }
        }))
    });

    use_hook(|| {
        let settings = settings.write();
        if let Ok(mut _settings) = settings.clone().write() {
            _settings.installed_games.insert(
                "U5hbdsT9W7".to_string(),
                InstalledGame {
                    id: "U5hbdsT9W7".to_string(),
                    biz_name: "nap_global".to_string(),
                    command_arguments: None,
                    command_wrapper: None,
                    environment: HashMap::new(),
                    executable_path: PathBuf::from("ZenlessZoneZero.exe"),
                    install_path: PathBuf::from("/path/to/Zenless Zone Zero/"),
                    runner: Runners::Wine(Wine {
                        version: "Spritz-Wine-TkG-10.15-3".to_string(),
                    }),
                    runtime_components: vec![RuntimeComponents::Dxvk("2.7.1".to_string())],
                },
            );
        }
    });

    {
        to_owned![settings];
        use_drop(move || {
            if let Ok(settings_data) = settings().read() {
                if let Err(err) = settings_data.save() {
                    eprintln!("Failed to save settings: {}", err);
                }
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
            .map_err(|err| err.to_string())
            .map(|response| response.games)
            .unwrap_or_else(|err| {
                eprintln!("Failed to load games from api: {err}");
                Vec::new()
            });

        let endfield_games = backend::game_providers::endfield::get_games()
            .await
            .map(|response| response.games)
            .unwrap_or_else(|err| {
                eprintln!("Failed to load endfield games: {err}");
                Vec::new()
            });

        api_games.extend(endfield_games);

        let mut api_news = HashMap::new();

        for game in &api_games {
            let game_id = game.id.to_owned();
            let game_biz = game.biz.to_owned();

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
                    eprintln!("Failed to load game content: {err}");
                }
            }
        }

        let api_game_basic_info = backend::game_providers::hoyoplay::get_all_game_basic_info(&settings_data, None)
            .await
            .map(|info| info.game_info_list)
            .unwrap_or_else(|err| {
                eprintln!("Failed to load game basic info: {err}");
                Vec::new()
            });

        Context {
            api_games,
            api_news,
            api_game_basic_info,
        }
    });

    use_context_provider(move || context);

    let mut system_status = use_signal(|| None::<SystemStatus>);
    use_context_provider(|| system_status);
    
    // Initialize game state
    let game_state = use_signal(|| GlobalGameState::new());
    use_context_provider(|| game_state);

    let init_settings = settings.clone();
    let mut game_state_initialized = use_signal(|| false);
    
    use_effect(move || {
        if !game_state_initialized() {
            let settings_clone = init_settings.read().clone();
            let mut state_signal = game_state;
            
            spawn(async move {
                // Check system status
                let settings_guard = match settings_clone.read() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to read settings for initialization: {}", e);
                        return;
                    }
                };
                
                let sys_status = SystemStatus::check(&settings_guard).await;
                drop(settings_guard);
                system_status.set(Some(sys_status.clone()));
                
                let runtime_ready = check_runtime_components(&sys_status);
                eprintln!("[INIT] Setting runtime_ready to: {}", runtime_ready);
                
                let manifest = TweakManifest::new();
                let jadeite_ready = check_jadeite_installed(&sys_status);
                
                // Modify state directly
                state_signal.write().set_runtime_ready(runtime_ready);
                
                if let Some(ctx) = context.read_unchecked().as_ref() {
                    for game in &ctx.api_games {
                        if manifest.needs_jadeite(&game.id) {
                            state_signal.write().set_tweaks_ready(&game.id, jadeite_ready);
                        }
                        
                        if let Ok(settings) = settings_clone.read() {
                            let installed = settings.installed_games.contains_key(&game.id);
                            state_signal.write().set_download_installed(&game.id, installed);
                        }
                    }
                }
                
                game_state_initialized.set(true);
            });
        }
    });
    
    let preload_settings = settings.clone();
    let mut has_preloaded = use_signal(|| false);

    use_effect(move || {
        if !has_preloaded() {
            if let Some(context_data) = context.read_unchecked().as_ref() {
                let games = context_data.api_games.clone();
                let basic_info = context_data.api_game_basic_info.clone();
                let api_news = context_data.api_news.clone();
                let settings_signal = preload_settings.clone();
                
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
                        
                        components::preload_images(
                            image_urls,
                            cache_path.clone(),
                        );
                        
                        let mut video_urls: Vec<Url> = Vec::new();
                        
                        for info in &basic_info {
                            if let Some(video_url) = get_video_url(&info.backgrounds) {
                                if let Ok(url) = video_url.parse() {
                                    video_urls.push(url);
                                }
                            }
                        }
                        
                        if !video_urls.is_empty() {
                            components::preload_videos(video_urls, settings_data.cache_directory.clone(), 3);
                        }
                    }
                });
                
                has_preloaded.set(true);
            }
        }
    });
    
    layout::app()
}

fn check_runtime_components(status: &SystemStatus) -> bool {
    status.runtime_ready()
}

fn check_jadeite_installed(status: &SystemStatus) -> bool {
    status.tweaks_ready()
}
