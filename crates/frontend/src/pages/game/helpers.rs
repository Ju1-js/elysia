use std::sync::{Arc, RwLock};
use freya::prelude::*;

use crate::components::DownloadProgress;
use backend::{
    settings::GlobalSettings,
    game_providers::installer::InstallerManager,
    runners::Runner,
};

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
    mut is_downloading: Signal<bool>,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        let settings_arc = settings.read().clone();
        
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
        is_downloading.set(true);

        spawn(async move {
            let installer = {
                let settings_guard = match settings_arc.read() {
                    Ok(s) => s,
                    Err(_) => {
                        is_downloading.set(false);
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
                            settings_guard.installed_games.insert(game_id_owned, installed_game);
                            if let Err(e) = settings_guard.save() {
                                eprintln!("Failed to save settings: {}", e);
                            } else {
                                drop(settings_guard);
                                let new_settings = settings_arc.read().unwrap().clone();
                                settings_mut.set(Arc::new(RwLock::new(new_settings)));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to install game: {}", e);
                    }
                }
            }

            is_downloading.set(false);
        });
    })
}

pub fn is_game_installed(
    settings: &Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: &str,
    biz: &str,
) -> bool {
    let settings_guard = settings.read();
    if let Ok(s) = settings_guard.read() {
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
