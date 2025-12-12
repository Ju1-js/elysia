use std::sync::{Arc, RwLock};
use std::rc::Rc;
use freya::prelude::*;
use reqwest::Url;

use crate::{
    components::{DownloadControl, DownloadProgress, MyButton, MyNetworkImage, MyNewsWidget},
    pages::GameSettings,
    context::Context,
};
use backend::{
    settings::GlobalSettings,
    game_providers::installer::InstallerManager,
    runners::Runner,
};

struct CrossfadeState {
    prev_url: Url,
    curr_url: Url,
    fade_progress: f64,
}

fn use_crossfade_background(url: Url) -> CrossfadeState {
    let mut prev = use_signal(|| url.clone());
    let mut curr = use_signal(|| url.clone());
    
    let anim = use_animation(|_| {
        AnimNum::new(0.0, 1.0)
            .time(700)
            .ease(Ease::InOut)
            .function(Function::Cubic)
    });

    let curr_url_str = curr.read().to_string();
    let new_url_str = url.to_string();
    
    if curr_url_str != new_url_str {
        prev.set(curr.read().clone());
        curr.set(url.clone());
        anim.start();
    }

    let fade_progress = if anim.is_running() {
        anim.get().read().read() as f64
    } else {
        1.0
    };

    CrossfadeState {
        prev_url: prev.read().clone(),
        curr_url: curr.read().clone(),
        fade_progress,
    }
}

fn create_progress_getter(
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> (String, Rc<dyn Fn(&str) -> Option<DownloadProgress>>) {
    let progress_key = format!("{}_streaming", game_id);
    
    let get_progress_fn = Rc::new(move |key: &str| {
        let settings = settings_sig.read();
        let s = settings.read().ok()?;
        
        let installer = InstallerManager::create_installer(
            &game_id,
            &biz,
            s.temp_directory.clone(),
            s.components_directory.clone(),
        )?;
        
        installer.get_progress(key).map(|p| DownloadProgress {
            downloaded: p.downloaded,
            total: p.total,
            mb_s: p.mb_s,
            part_index: p.part_index,
            parts_total: p.parts_total,
            status: p.status,
            is_busy: p.is_busy,
        })
    });

    (progress_key, get_progress_fn)
}

fn create_game_action_handler(
    mut settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        let settings = settings_sig.write();
        
        let installer = settings.write().ok().and_then(|settings| {
            if let Some(installed_game) = settings.installed_games.get(&game_id) {
                if let Err(e) = installed_game.runner.run_game(&settings, installed_game) {
                    eprintln!("Failed to run game: {}", e);
                }
                return None;
            }

            InstallerManager::create_installer(
                &game_id,
                &biz,
                settings.temp_directory.clone(),
                settings.components_directory.clone(),
            )
        });

        if let Some(inst) = installer {
            InstallerManager::spawn_install(
                settings.clone(),
                inst,
                game_id.clone(),
            );
        }
    })
}

fn check_game_installed(
    settings_sig: &Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: &str,
    biz: &str,
) -> bool {
    let settings = settings_sig.read();
    if let Ok(s) = settings.read() {
        InstallerManager::is_game_installed(
            &s,
            game_id,
            biz,
            s.temp_directory.clone(),
            s.components_directory.clone(),
        )
    } else {
        false
    }
}

#[component]
pub fn Game() -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    let ctx = use_context::<Context>();
    let settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    
    let mut current_page = use_signal(|| "game");
    let mut prev_game_id = use_signal(|| None::<String>);

    let current_game_id = selected_game_id.read().clone();
    if prev_game_id.read().as_ref() != current_game_id.as_ref() {
        current_page.set("game");
        prev_game_id.set(current_game_id.clone());
    }
 
    let game_id = selected_game_id.read();
    let Some(ref game_id_str) = *game_id else {
        return rsx! { rect { width: "fill", height: "fill" } };
    };

    let Some(game_data) = ctx.api_games.iter().find(|g| &g.id == game_id_str).cloned() else {
        return rsx! { 
            rect { 
                width: "fill",
                height: "fill",
                label { "Game not found" }
            }
        };
    };

    let url = use_memo(move || {
        let game_id = selected_game_id.read();
        game_id.as_ref().and_then(|id| {
            ctx.api_games.iter()
                .find(|g| &g.id == id)
                .and_then(|g| g.display.background.url.parse::<Url>().ok())
        })
    });

    let Some(parsed_url) = url.read().clone() else {
        return rsx! {
            rect {
                width: "fill",
                height: "fill",
                label { "Invalid background image URL" }
            }
        };
    };

    let mut news_carousel_index = use_signal(|| 0);
    let game_id_for_effect = game_id_str.clone();
    use_effect(use_reactive!(|game_id_for_effect| {
        let _ = game_id_for_effect;
        news_carousel_index.set(0);
    }));

    let is_installed = check_game_installed(&settings_sig, &game_data.id, &game_data.biz);
    let (progress_key, get_progress_fn) = create_progress_getter(
        settings_sig,
        game_data.id.clone(),
        game_data.biz.clone()
    );
    let onpress = create_game_action_handler(settings_sig, game_data.id.clone(), game_data.biz.clone());
    let crossfade = use_crossfade_background(parsed_url);

    if *current_page.read() != "game" {
        return rsx! {
            GameSettings {
                on_back: move |_| current_page.set("game"),
                background_url: crossfade.curr_url.clone(),
                game_name: game_data.display.name.clone(),
            }
        };
    }

    rsx! {
        rect {
            width: "fill",
            height: "fill",
            
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                main_align: "end",
                cross_align: "end",
                layer: "2",
                opacity: "{1.0 - crossfade.fade_progress}",
                MyNetworkImage {
                    url: crossfade.prev_url,
                    sampling: "trilinear",
                }
            }
            
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                main_align: "end",
                cross_align: "end",
                layer: "2",
                opacity: "{crossfade.fade_progress}",
                MyNetworkImage {
                    url: crossfade.curr_url.clone(),
                    sampling: "trilinear",
                }
            }
            
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                main_align: "start",
                cross_align: "start",
                layer: "3",
                MyNetworkImage {
                    url: crossfade.curr_url.clone(),
                    sampling: "trilinear",
                }
            }
            
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                direction: "horizontal",
                main_align: "end",
                cross_align: "start",
                spacing: "20",
                padding: "32",
                layer: "-1",
                
                MyButton {
                    onpress: move |_| println!("Meow clicked!"),
                    rect {
                        direction: "horizontal",
                        cross_align: "center",
                        spacing: "8",
                        label {
                            font_size: "16",
                            font_weight: "500",
                            color: "white",
                            "Meow 🐾"
                        }
                    }
                }
            }
            
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "92",
                width: "550",
                height: "100%",
                direction: "vertical",
                main_align: "end",
                cross_align: "start",
                padding: "40",
                spacing: "20",
                layer: "-1",
                
                rect {
                    width: "450",
                    MyNewsWidget {
                        game_id: game_data.id.clone(),
                        carousel_index: news_carousel_index,
                    }
                }
                
                DownloadControl {
                    game_id: game_data.id.clone(),
                    progress_key: progress_key,
                    installed: is_installed,
                    get_progress: get_progress_fn,
                    accent_color: "#ff9500".to_string(),
                    onpress: onpress,
                }
            }
                
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
                            svg_content: r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10 10-4.5 10-10S17.5 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm.5-13H11v6l5.2 3.2.8-1.3-4.5-2.7V7z"/>
                            </svg>"#
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
                    onpress: move |_| current_page.set("settings"),
                    rect {
                        direction: "horizontal",
                        cross_align: "center",
                        spacing: "8",
                        svg {
                            width: "20",
                            height: "20",
                            svg_content: r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                <path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z"/>
                            </svg>"#
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
}