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

use crate::context::Context;
use backend::{
    game_providers::hoyoplay::{get_game_content, get_games},
    runners::{Runners, Wine},
    settings::{GlobalSettings, InstalledGame, RuntimeComponents},
};

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

    {
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
    }

    {
        to_owned![settings];
        use_drop(move || {
            if let Ok(settings) = settings().read() {
                if let Err(err) = settings.save() {
                    eprintln!("Failed to save settings: {}", err);
                }
                println!("Settings saved successfully");
            } else {
                println!("Failed to save settings");
            }
        });
    }
    {
        to_owned![settings];
        use_context_provider(move || settings);
    }

    use_init_theme(|| DARK_THEME);

    let ctx = use_resource(move || async move {
        let settings = settings.read();
        let settings = settings.read().unwrap().clone();

        // Get hoyoplay games
        let mut api_games = get_games(&settings)
            .await
            .map_err(|e| e.to_string())
            .map(|v| v.games)
            .unwrap_or_else(|e| {
                println!("Failed to load games from api: {e}");
                Vec::new()
            });

        // Get endfield games
        let endfield_games = backend::game_providers::endfield::get_games()
            .await
            .map(|v| v.games)
            .unwrap_or_else(|e| {
                println!("Failed to load endfield games: {e}");
                Vec::new()
            });

        // Merge games lists
        api_games.extend(endfield_games);

        let mut api_news = HashMap::new();

        for game in &api_games {
            let id = game.id.to_owned();
            let biz = game.biz.to_owned();

            let response = if biz == "endfield" {
                backend::game_providers::endfield::get_game_content(&id).await
            } else {
                get_game_content(&settings, &id).await
            };

            match response {
                Ok(response) => {
                    api_news.insert(id, response.content);
                }
                Err(e) => {
                    println!("Failed to load game content: {e}");
                }
            }
        }

        Context {
            api_games,
            api_news,
        }
    });

    use_context_provider(move || ctx);
    layout::app()
}
