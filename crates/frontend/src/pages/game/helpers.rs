use crate::components::DownloadProgress;
use backend::{
    game_providers::installer::InstallerManager, runners::Runner, settings::GlobalSettings,
};
use freya::prelude::*;
use std::sync::{Arc, RwLock};

use super::state::GlobalGameStateSignal;

/// Create a progress getter for game download progress tracking
pub fn create_progress_getter(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> (
    String,
    std::rc::Rc<dyn Fn(&str) -> Option<DownloadProgress>>,
) {
    let key = format!("{}_streaming", game_id);
    let getter = std::rc::Rc::new(move |progress_key: &str| {
        let settings_guard = settings.read();
        let settings_data = settings_guard.read().ok()?;
        let installer = InstallerManager::create_installer(
            &game_id,
            &biz,
            settings_data.temp_directory.clone(),
            settings_data.components_directory.clone(),
        )?;
        installer
            .get_progress(progress_key)
            .map(|p| DownloadProgress {
                downloaded: p.downloaded,
                total: p.total,
                speed_mb_s: p.mb_s as f64,
                status: p.status,
                is_busy: p.is_busy,
            })
    });

    (key, getter)
}

/// Create an event handler for game download and launching
pub fn create_game_download_handler(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
    mut game_state: GlobalGameStateSignal,
    mut video_state: Signal<super::video_state::VideoState>,
) -> EventHandler<PressEvent>
{
    EventHandler::new(move |_| {
        let settings_arc = settings.read().clone();
        let game_id_clone = game_id.clone();

        // CRITICAL: Always check actual settings.installed_games, not just state signal
        // The state signal can be stale when switching between games
        let settings_guard = match settings_arc.read() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[Game Handler] Failed to read settings");
                return;
            }
        };

        let is_actually_installed = settings_guard.installed_games.contains_key(&game_id);
        
        eprintln!(
            "[Game Handler] game_id: {}, is_actually_installed: {}, has_entry: {}",
            game_id,
            is_actually_installed,
            settings_guard.installed_games.get(&game_id).is_some()
        );

        if is_actually_installed {
            // Game is installed - LAUNCH IT
            let Some(installed_game) = settings_guard.installed_games.get(&game_id) else {
                eprintln!("[Game Launch] ERROR: Game {} in installed_games but get() returned None!", game_id);
                return;
            };

            eprintln!("[Game Launch] Launching game: {}", game_id);
            
            // Extract wine info (both path and prefix) if using Wine runner
            let (wine_path_opt, wine_prefix_opt) = match &installed_game.runner {
                backend::runners::Runners::Wine(wine) => {
                    let components_path = settings_guard.components_directory.join("wine");
                    let wine_dir = components_path.join(&wine.version);
                    let prefix = settings_guard.wineprefixes_directory.join(&installed_game.biz_name);
                    (
                        Some(wine_dir.to_string_lossy().to_string()),
                        Some(prefix.to_string_lossy().to_string())
                    )
                }
                _ => (None, None),
            };
            
            // Launch the game and get the process handle
            match installed_game
                .runner
                .run_game(&settings_guard, installed_game)
            {
                Ok(child) => {
                    let pid = child.id();
                    eprintln!("[Game Launch] Game process started with PID: {}, waiting for completion...", pid);
                    
                    // Set game running state with PID, wine_path, and wine_prefix
                    game_state.write().set_game_running(true, Some(pid), wine_path_opt, wine_prefix_opt);
                    
                    // Pause the video player to save resources
                    eprintln!("[Game Launch] Pausing video player");
                    video_state.write().pause();
                    
                    // Spawn a blocking task to wait for the game to close
                    spawn(async move {
                        // Use spawn_blocking to wait for the process in a separate thread
                        let mut child_handle = child;
                        let result = tokio::task::spawn_blocking(move || {
                            child_handle.wait()
                        }).await;
                        
                        match result {
                            Ok(Ok(status)) => {
                                eprintln!("[Game Launch] Game exited with status: {:?}", status);
                            }
                            Ok(Err(e)) => {
                                eprintln!("[Game Launch] Error waiting for game process: {}", e);
                            }
                            Err(e) => {
                                eprintln!("[Game Launch] Task join error: {}", e);
                            }
                        }
                        
                        eprintln!("[Game Launch] Game closed, clearing running state and resuming video");
                        // Clear game running state after game closes
                        game_state.write().set_game_running(false, None, None, None);
                        
                        // Resume the video player
                        video_state.write().resume();
                    });
                }
                Err(e) => {
                    eprintln!("[Game Launch] Failed to run game: {}", e);
                    // Clear running state if launch failed
                    game_state.write().set_game_running(false, None, None, None);
                }
            }
            return;
        }

        // Game is NOT installed - DOWNLOAD IT
        eprintln!("[Game Download] Starting download for game: {}", game_id);
        
        drop(settings_guard);
        
        let biz_clone = biz.clone();
        let mut settings_mut = settings;
        let mut state_signal = game_state;

        state_signal
            .write()
            .set_download_active(&game_id_clone, true);

        spawn(async move {
            let installer = {
                let settings_guard = match settings_arc.read() {
                    Ok(s) => s,
                    Err(_) => {
                        state_signal
                            .write()
                            .set_download_active(&game_id_clone, false);
                        return;
                    }
                };

                InstallerManager::create_installer(
                    &game_id_clone,
                    &biz_clone,
                    settings_guard.temp_directory.clone(),
                    settings_guard.components_directory.clone(),
                )
            };

            if let Some(inst) = installer {
                match inst.install().await {
                    Ok(installed_game) => {
                        if let Ok(mut settings_guard) = settings_arc.write() {
                            settings_guard
                                .installed_games
                                .insert(game_id_clone.clone(), installed_game);

                            if let Err(e) = settings_guard.save() {
                                eprintln!("Failed to save settings: {}", e);
                            } else {
                                drop(settings_guard);
                                let new_settings = settings_arc.read().unwrap().clone();
                                settings_mut.set(Arc::new(RwLock::new(new_settings)));
                            }
                        }

                        state_signal
                            .write()
                            .set_download_installed(&game_id_clone, true);
                    }
                    Err(e) => {
                        eprintln!("Failed to install game: {}", e);
                    }
                }
            }

            state_signal
                .write()
                .set_download_active(&game_id_clone, false);
        });
    })
}
