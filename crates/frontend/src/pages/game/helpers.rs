use crate::components::DownloadProgress;
use backend::{
    game_providers::installer::InstallerManager, 
    progress::ProgressTracker,
    runners::Runner, 
    settings::GlobalSettings,
};
use freya::prelude::*;
use dioxus::prelude::*;
use std::sync::{Arc, RwLock};
use std::rc::Rc;

use super::state::GlobalGameStateSignal;

type DownloadProgressGetter = Rc<dyn Fn(&str) -> Option<DownloadProgress>>;

/// Create a progress getter for game download progress tracking
pub fn create_progress_getter(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> (
    String,
    DownloadProgressGetter,
) {
    let key = format!("{game_id}_streaming");
    let getter = std::rc::Rc::new(move |progress_key: &str| {
        let settings_guard = settings.read();
        let settings_data = settings_guard.read().ok()?;
        let installer = InstallerManager::create_installer(
            &game_id,
            &biz,
            settings_data.temp_directory.clone(),
            settings_data.games_directory.clone(),
        )?;
        installer
            .get_progress(progress_key)
            .map(|p| DownloadProgress {
                downloaded: p.downloaded,
                total: p.total,
                speed_mb_s: f64::from(p.mb_s),
                status: p.status,
                is_busy: p.is_busy,
            })
    });

    (key, getter)
}

/// Create an event handler for game download and launching
#[allow(clippy::too_many_lines)]
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
        let Ok(settings_guard) = settings_arc.read() else {
            eprintln!("[Game Handler] Failed to read settings");
            return;
        };

        let is_actually_installed = settings_guard.installed_games.contains_key(&game_id);
        
        eprintln!(
            "[Game Handler] game_id: {}, is_actually_installed: {}, has_entry: {}",
            game_id,
            is_actually_installed,
            settings_guard.installed_games.contains_key(&game_id)
        );

        if is_actually_installed {
            // Game is installed - LAUNCH IT
            let Some(installed_game) = settings_guard.installed_games.get(&game_id) else {
                eprintln!("[Game Launch] ERROR: Game {game_id} in installed_games but get() returned None!");
                return;
            };

            eprintln!("[Game Launch] Launching game: {game_id}");
            
            // Extract runner info (both path and prefix) for Wine/Proton
            let (runner_path_opt, wine_prefix_opt) = match &installed_game.runner {
                backend::runners::Runners::Wine(wine) => {
                    // Resolve the Wine version (handles "auto" and empty versions)
                    let resolved_version = wine.resolve_version(&settings_guard)
                        .unwrap_or_else(|e| {
                            eprintln!("[Game Launch] Warning: Failed to resolve Wine version: {e}, using configured version: {}", wine.version);
                            wine.version.clone()
                        });
                    
                    // For system wine, use the directory from PATH
                    let wine_dir = if resolved_version == "system" {
                        backend::runners::get_system_wine_dir()
                            .unwrap_or_else(|| {
                                eprintln!("[Game Launch] Warning: System wine not found in PATH, using /usr/bin as fallback");
                                std::path::PathBuf::from("/usr/bin")
                            })
                    } else {
                        let components_path = settings_guard.components_directory.join("wine");
                        components_path.join(&resolved_version)
                    };
                    let prefix = settings_guard.wineprefixes_directory.join(&installed_game.biz_name);
                    (
                        Some(wine_dir.to_string_lossy().to_string()),
                        Some(prefix.to_string_lossy().to_string())
                    )
                }
                backend::runners::Runners::Proton(proton) => {
                    // Resolve the Proton version (handles "auto" and empty versions)
                    let resolved_version = proton.resolve_version(&settings_guard)
                        .unwrap_or_else(|e| {
                            eprintln!("[Game Launch] Warning: Failed to resolve Proton version: {e}, using configured version: {}", proton.version);
                            proton.version.clone()
                        });
                    
                    let components_path = settings_guard.components_directory.join("proton");
                    let proton_dir = components_path.join(&resolved_version);
                    let prefix = settings_guard.wineprefixes_directory.join(&installed_game.biz_name);
                    (
                        Some(proton_dir.to_string_lossy().to_string()),
                        Some(prefix.to_string_lossy().to_string())
                    )
                }
                backend::runners::Runners::Native => (None, None),
            };
            
            // Launch the game and get the process handle
            match installed_game
                .runner
                .run_game(&settings_guard, installed_game)
            {
                Ok(child) => {
                    let pid = child.id();
                    eprintln!("[Game Launch] Game process started with PID: {pid}, waiting for completion...");
                    
                    // Drop settings_guard before spawning async task
                    drop(settings_guard);
                    
                    // Set game running state with PID, wine_path, and wine_prefix
                    game_state.write().set_game_running(true, Some(pid), runner_path_opt, wine_prefix_opt);
                    
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
                                eprintln!("[Game Launch] Game exited with status: {status:?}");
                            }
                            Ok(Err(e)) => {
                                eprintln!("[Game Launch] Error waiting for game process: {e}");
                            }
                            Err(e) => {
                                eprintln!("[Game Launch] Task join error: {e}");
                            }
                        }
                        
                        eprintln!("[Game Launch] Game closed, clearing running state and resuming video");
                        // Calculate playtime
                        let elapsed_seconds = game_state.read().get_elapsed_playtime();
                        eprintln!("[Game Launch] Game played for {elapsed_seconds} seconds");
                        
                        // Clear game running state after game closes
                        game_state.write().set_game_running(false, None, None, None);
                        
                        // Update playtime in game_preferences
                        if let Ok(mut settings_guard) = settings_arc.write() {
                            // Get or create game preferences for this game
                            let prefs = settings_guard.game_preferences
                                .entry(game_id_clone.clone())
                                .or_insert_with(backend::settings::GamePreferences::default);
                            
                            prefs.playtime_seconds += elapsed_seconds;
                            eprintln!("[Game Launch] Total playtime for {}: {} seconds", game_id_clone, prefs.playtime_seconds);
                            
                            // Save settings to persist playtime
                            if let Err(e) = settings_guard.save() {
                                eprintln!("[Game Launch] Failed to save playtime: {e}");
                            }
                        }
                        
                        // Resume the video player
                        video_state.write().resume();
                    });
                }
                Err(e) => {
                    eprintln!("[Game Launch] Failed to run game: {e}");
                    // Clear running state if launch failed
                    game_state.write().set_game_running(false, None, None, None);
                }
            }
            return;
        }

        // Game is NOT installed - DOWNLOAD IT
        eprintln!("[Game Download] Starting download for game: {game_id}");
        
        drop(settings_guard);
        
        let biz_clone = biz.clone();
        let mut settings_mut = settings;
        let mut state_signal = game_state;

        state_signal
            .write()
            .set_download_active(&game_id_clone, true);

        spawn(async move {
            let installer = {
                let Ok(settings_guard) = settings_arc.read() else {
                    state_signal
                        .write()
                        .set_download_active(&game_id_clone, false);
                    return;
                };

                InstallerManager::create_installer(
                    &game_id_clone,
                    &biz_clone,
                    settings_guard.temp_directory.clone(),
                    settings_guard.games_directory.clone(),
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
                                eprintln!("Failed to save settings: {e}");
                            } else {
                                drop(settings_guard);
                                let new_settings = settings_arc.read().expect("Settings lock poisoned").clone();
                                settings_mut.set(Arc::new(RwLock::new(new_settings)));
                            }
                        }

                        state_signal
                            .write()
                            .set_download_installed(&game_id_clone, true);
                    }
                    Err(e) => {
                        eprintln!("Failed to install game: {e}");
                    }
                }
            }

            state_signal
                .write()
                .set_download_active(&game_id_clone, false);
        });
    })
}
