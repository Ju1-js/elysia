use std::sync::{Arc, RwLock};
use freya::prelude::*;
use crate::components::DownloadProgress;
use backend::{
    settings::GlobalSettings,
    game_providers::installer::InstallerManager,
    runners::Runner,
};

use super::state::GlobalGameStateSignal;

pub fn create_progress_getter(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> (String, std::rc::Rc<dyn Fn(&str) -> Option<DownloadProgress>>) {
    let key = format!("{}_streaming", game_id);
    let getter = std::rc::Rc::new(move |progress_key: &str| {
        let settings_guard = settings.read();
        let s = settings_guard.read().ok()?;
        let installer = InstallerManager::create_installer(
            &game_id,
            &biz,
            s.temp_directory.clone(),
            s.components_directory.clone(),
        )?;
        installer.get_progress(progress_key).map(|p| DownloadProgress {
            downloaded: p.downloaded,
            total: p.total,
            speed_mb_s: p.mb_s,
            status: p.status,
            is_busy: p.is_busy,
        })
    });
    
    (key, getter)
}

pub fn create_game_download_handler(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
    game_state: GlobalGameStateSignal,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        let settings_arc = settings.read().clone();
        
        // Check if game is already installed and run it
        {
            let settings_guard = match settings_arc.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            
            if let Some(installed_game) = settings_guard.installed_games.get(&game_id) {
                if let Err(e) = installed_game.runner.run_game(&settings_guard, installed_game) {
                    eprintln!("Failed to run game: {}", e);
                }
                return;
            }
        }

        let game_id_owned = game_id.clone();
        let biz_owned = biz.clone();
        let mut settings_mut = settings;
        let mut state = game_state;
        
        // Mark download as active in global state
        if let Ok(mut gs) = state.write().write() {
            gs.set_download_active(&game_id_owned, true);
        }

        spawn(async move {
            let installer = {
                let settings_guard = match settings_arc.read() {
                    Ok(s) => s,
                    Err(_) => {
                        if let Ok(mut gs) = state.write().write() {
                            gs.set_download_active(&game_id_owned, false);
                        }
                        return;
                    }
                };
                
                InstallerManager::create_installer(
                    &game_id_owned,
                    &biz_owned,
                    settings_guard.temp_directory.clone(),
                    settings_guard.components_directory.clone(),
                )
            };

            if let Some(inst) = installer {
                match inst.install().await {
                    Ok(installed_game) => {
                        if let Ok(mut settings_guard) = settings_arc.write() {
                            settings_guard.installed_games.insert(game_id_owned.clone(), installed_game);
                            
                            if let Err(e) = settings_guard.save() {
                                eprintln!("Failed to save settings: {}", e);
                            } else {
                                drop(settings_guard);
                                let new_settings = settings_arc.read().unwrap().clone();
                                settings_mut.set(Arc::new(RwLock::new(new_settings)));
                            }
                        }
                        
                        // Mark as installed in global state
                        if let Ok(mut gs) = state.write().write() {
                            gs.set_download_installed(&game_id_owned, true);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to install game: {}", e);
                    }
                }
            }
            
            // Mark download as inactive in global state
            if let Ok(mut gs) = state.write().write() {
                gs.set_download_active(&game_id_owned, false);
            }
        });
    })
}

