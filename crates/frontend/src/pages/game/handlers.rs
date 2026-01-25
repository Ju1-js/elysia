use freya::prelude::*;
use std::sync::{Arc, RwLock};
use std::rc::Rc;

use crate::components::{DownloadProgress, SetupProgress, SetupStep};
use crate::{debug, debug_error, debug_info};
use backend::{
    components::{ComponentManager, ComponentType, ComponentVersion},
    progress::ProgressTracker,
    settings::GlobalSettings,
    status::SystemStatus,
};

use super::state::GlobalGameStateSignal;

type SetupProgressGetter = Rc<dyn Fn(&str) -> Option<SetupProgress>>;

/// Create a progress getter for runtime component downloads
pub fn create_runtime_progress_getter(
    progress_tracker: ProgressTracker,
) -> SetupProgressGetter {
    std::rc::Rc::new(move |key: &str| -> Option<SetupProgress> {
        let comp_progress = progress_tracker.get(key)?;

        if comp_progress.is_finished {
            return None;
        }

        let step = {
            let name_lower = comp_progress.component_name.to_lowercase();
            if name_lower.contains("umu") {
                SetupStep::DownloadUmu
            } else if name_lower.contains("dxvk") {
                SetupStep::DownloadDxvk
            } else if name_lower.contains("wine") {
                SetupStep::DownloadWine
            } else if name_lower.contains("steam runtime") {
                SetupStep::DownloadSteamRuntime
            } else if name_lower.contains("proton") {
                SetupStep::DownloadProton
            } else if name_lower.contains("jadeite") {
                SetupStep::DownloadJadeite
            } else {
                SetupStep::CheckDependencies
            }
        };

        let status = if comp_progress.downloaded == comp_progress.total
            && comp_progress.total > 0
            && comp_progress.is_busy
        {
            format!("Extracting {}", comp_progress.component_name)
        } else {
            format!("Downloading {}", comp_progress.component_name)
        };

        Some(SetupProgress {
            current_step: step,
            total_steps: comp_progress.total_steps.unwrap_or(2),
            current_step_index: comp_progress.step_index.unwrap_or(0),
            step_progress: Some(DownloadProgress {
                downloaded: comp_progress.downloaded,
                total: comp_progress.total,
                speed_mb_s: 0.0,
                status,
                is_busy: comp_progress.is_busy,
            }),
        })
    })
}

/// Create a progress getter for tweaks downloads
pub fn create_tweaks_progress_getter(
    progress_tracker: ProgressTracker,
) -> SetupProgressGetter {
    std::rc::Rc::new(move |key: &str| -> Option<SetupProgress> {
        let comp_progress = progress_tracker.get(key)?;

        if comp_progress.is_finished {
            return None;
        }

        let step = SetupStep::DownloadJadeite;

        let status = if comp_progress.downloaded == comp_progress.total
            && comp_progress.total > 0
            && comp_progress.is_busy
        {
            format!("Extracting {}", comp_progress.component_name)
        } else {
            format!("Downloading {}", comp_progress.component_name)
        };

        Some(SetupProgress {
            current_step: step,
            total_steps: 1,
            current_step_index: 0,
            step_progress: Some(DownloadProgress {
                downloaded: comp_progress.downloaded,
                total: comp_progress.total,
                speed_mb_s: 0.0,
                status,
                is_busy: comp_progress.is_busy,
            }),
        })
    })
}

/// Create an event handler for component setup
#[allow(clippy::too_many_lines)]
pub fn create_component_setup_handler(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    progress_tracker: ProgressTracker,
    mut system_status: Signal<Option<SystemStatus>>,
    game_state: GlobalGameStateSignal,
    component_service: Signal<Option<crate::services::ComponentService>>,
    game_id: String,
) -> EventHandler<PressEvent> {
    debug!("Creating component setup handler");
    EventHandler::new(move |_| {
        debug!("Component setup handler called");
        
        let mut state_signal = game_state;
        
        // Atomically check and set the active flag
        {
            let mut state = state_signal.write();
            if state.is_runtime_setup_active() {
                debug!("Runtime setup already in progress, ignoring duplicate click");
                return;
            }
            // Set active flag immediately to block other clicks
            state.set_runtime_active(true);
        }
        
        let settings_arc = settings.read().clone();
        let tracker = progress_tracker.clone();
        let game_id_for_lookup = game_id.clone();

        let runner_type = state_signal.read().current_runner_type.clone();
        let missing = state_signal.read().get_missing_components();

        debug!(
            "Starting download for {:?}, missing: {:?}",
            runner_type, missing
        );

        let service_opt = component_service.read().clone();
        let Some(service) = service_opt else {
            debug!("ComponentService not initialized yet");
            // Reset the flag since we're not actually starting
            state_signal.write().set_runtime_active(false);
            return;
        };
        
        // Initialize progress tracker
        tracker.report("runtime_setup", "Initializing", backend::progress::ReportParams {
            downloaded: 0,
            total: 100,
            is_busy: true,
            step_index: None,
            total_steps: None,
        });

        spawn_forever(async move {

            let settings_data = if let Ok(s) = settings_arc.read() { s.clone() } else {
                debug_error!("Failed to acquire settings lock");
                state_signal.write().set_runtime_active(false);
                return;
            };

            debug!("Re-checking component readiness...");
            
            let (configured_wine_version, configured_proton_version) = {
                let runner = settings_data.game_preferences.get(&game_id_for_lookup)
                    .and_then(|prefs| prefs.runner.as_ref())
                    .unwrap_or(&settings_data.default_preferences.runner);
                
                match runner {
                    backend::runners::Runners::Wine(wine) => (Some(wine.version.clone()), None),
                    backend::runners::Runners::Proton(proton) => (None, Some(proton.version.clone())),
                    backend::runners::Runners::Native => (None, None),
                }
            };
            
            match runner_type {
                super::state::RunnerType::Wine => {
                    let wine_ready = if configured_wine_version.as_deref() == Some("system") {
                        // System wine is always "ready" if available
                        backend::runners::is_system_wine_available()
                    } else if let Some(version) = configured_wine_version.as_ref() {
                        if version.is_empty() || version == "auto" {
                            service.is_installed(&settings_data, backend::components::ComponentType::Wine).await
                        } else {
                            let wine_path = settings_data.components_directory.join("wine").join(version);
                            debug!("Checking for specific Wine version at: {:?}", wine_path);
                            wine_path.exists() && wine_path.is_dir()
                        }
                    } else {
                        false
                    };
                    
                    let dxvk_installed = service
                        .is_installed(&settings_data, backend::components::ComponentType::Dxvk).await;
                    state_signal.write().set_wine_ready(wine_ready);
                    state_signal.write().set_dxvk_ready(dxvk_installed);
                    debug!(
                        "Wine ready: {} (version: {:?}), DXVK ready: {}",
                        wine_ready, configured_wine_version, dxvk_installed
                    );
                }
                super::state::RunnerType::Proton => {
                    let proton_installed = if let Some(version) = configured_proton_version.as_ref() {
                        if version.is_empty() || version == "auto" {
                            service.is_installed(&settings_data, backend::components::ComponentType::Proton).await
                        } else {
                            let proton_path = settings_data.components_directory.join("proton").join(version);
                            debug!("Checking for specific Proton version at: {:?}", proton_path);
                            proton_path.exists() && proton_path.is_dir()
                        }
                    } else {
                        false
                    };
                    
                    let umu_installed = service
                        .is_installed(&settings_data, backend::components::ComponentType::Umu).await;
                    let steamrt_installed = service.is_installed(
                        &settings_data,
                        backend::components::ComponentType::SteamRuntime,
                    ).await;

                    state_signal.write().set_proton_ready(proton_installed);
                    state_signal.write().set_umu_ready(umu_installed);
                    state_signal.write().set_steamrt_ready(steamrt_installed);
                    
                    debug!(
                        "Proton ready: {} (version: {:?}), UMU ready: {}, SteamRT ready: {}",
                        proton_installed, configured_proton_version, umu_installed, steamrt_installed
                    );
                }
            }

            let missing = state_signal.read().get_missing_components();
            debug!(
                "After re-check, missing components: {:?}",
                missing
            );

            let manager_arc = service.manager();

            let setup_result: Result<(), anyhow::Error> = match runner_type {
                super::state::RunnerType::Wine => {
                    let mut result = Ok(());
                    
                    if !state_signal.read().component_setup.wine_ready {
                        debug!("Downloading Wine...");
                        
                        // Convert "auto" or empty to None for latest version
                        let wine_version_to_download = configured_wine_version.as_deref()
                            .filter(|v| !v.is_empty() && *v != "auto");
                        
                        let wine_result = {
                            let component_manager = manager_arc.read().await;
                            backend::runners::Runners::download_wine(
                                &settings_data,
                                &component_manager,
                                wine_version_to_download,
                                Some(&tracker),
                                "runtime_setup",
                            )
                            .await
                        };
                        
                        match &wine_result {
                            Ok(downloaded_version) => {
                                state_signal.write().set_wine_ready(true);
                                debug!("Wine downloaded successfully: version {}", downloaded_version);
                                
                                // Update default_preferences with the downloaded Wine version
                                if let Ok(mut settings_guard) = settings_arc.write() {
                                    settings_guard.default_preferences.runner = 
                                        backend::runners::Runners::Wine(backend::runners::Wine {
                                            version: downloaded_version.clone(),
                                        });
                                    if let Err(_e) = settings_guard.save() {
                                        debug_error!("Failed to save settings after Wine download");
                                    }
                                }
                            }
                            Err(_e) => {
                                debug_error!("Failed to download Wine");
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                        result = wine_result.map(|_| ());
                    }

                    if !state_signal.read().component_setup.dxvk_ready {
                        debug!("Downloading DXVK...");
                        let dxvk_result = {
                            let component_manager = manager_arc.read().await;
                            backend::runners::Runners::download_dxvk(
                                &settings_data,
                                &component_manager,
                                None,
                                Some(&tracker),
                                "runtime_setup",
                            )
                            .await
                        };
                        
                        match &dxvk_result {
                            Ok(downloaded_version) => {
                                state_signal.write().set_dxvk_ready(true);
                                debug!("DXVK downloaded successfully: version {}", downloaded_version);
                                
                                // Update default_preferences with the downloaded DXVK version
                                if let Ok(mut settings_guard) = settings_arc.write() {
                                    // Replace or add DXVK to runtime_components
                                    let mut components = settings_guard.default_preferences.runtime_components.clone();
                                    components.retain(|c| !matches!(c, backend::settings::RuntimeComponents::Dxvk(_)));
                                    components.push(backend::settings::RuntimeComponents::Dxvk(downloaded_version.clone()));
                                    settings_guard.default_preferences.runtime_components = components;
                                    if let Err(_e) = settings_guard.save() {
                                        debug_error!("Failed to save settings after DXVK download");
                                    }
                                }
                            }
                            Err(_e) => {
                                debug_error!("Failed to download DXVK");
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                        result = dxvk_result.map(|_| ());
                    }
                    result
                }
                super::state::RunnerType::Proton => {
                    debug!("Checking Proton runtime components...");

                    // Check current installation status before downloading anything
                    let proton_installed = state_signal.read().component_setup.proton_ready;
                    let umu_installed = state_signal.read().component_setup.umu_ready;
                    let steamrt_installed = state_signal.read().component_setup.steamrt_ready;

                    debug!("Current status - proton: {}, umu: {}, steamrt: {}", 
                        proton_installed, umu_installed, steamrt_installed);

                    if !umu_installed || !steamrt_installed {
                        debug!("Downloading runtime dependencies (UMU/SteamRT)...");
                        let runtime_result = {
                            let mut component_manager = manager_arc.write().await;
                            backend::runners::Runners::download_proton_runtime(
                                &settings_data,
                                &mut component_manager,
                                Some(&tracker),
                                "runtime_setup",
                            )
                            .await
                        };
                        
                        match runtime_result {
                            Ok(()) => {
                                debug!("UMU and SteamRT download completed");
                                // Re-check installation status
                                let umu_now = service
                                    .is_installed(&settings_data, backend::components::ComponentType::Umu).await;
                                let steamrt_now = service.is_installed(
                                    &settings_data,
                                    backend::components::ComponentType::SteamRuntime,
                                ).await;
                                
                                let proton_now = if let Some(version) = configured_proton_version.as_ref() {
                                    if version.is_empty() || version == "auto" {
                                        service.is_installed(&settings_data, backend::components::ComponentType::Proton).await
                                    } else {
                                        let proton_path = settings_data.components_directory.join("proton").join(version);
                                        proton_path.exists() && proton_path.is_dir()
                                    }
                                } else {
                                    false
                                };
                                
                                state_signal.write().set_umu_ready(umu_now);
                                state_signal.write().set_steamrt_ready(steamrt_now);
                                state_signal.write().set_proton_ready(proton_now);
                                debug!("Runtime dependencies (UMU/SteamRT) installed successfully");
                            }
                            Err(ref _e) => {
                                debug_error!("Failed to download UMU/SteamRT");
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                        runtime_result
                    } else if !proton_installed {
                        debug!("Runtime dependencies already installed. Downloading Proton...");
                        state_signal.write().set_umu_ready(true);
                        state_signal.write().set_steamrt_ready(true);
                        
                        // Convert "auto" or empty to None for latest version
                        let proton_version_to_download = configured_proton_version.as_deref()
                            .filter(|v| !v.is_empty() && *v != "auto");
                        
                        let proton_result = {
                            let mut component_manager = manager_arc.write().await;
                            backend::runners::Runners::download_proton(
                                &settings_data,
                                &mut component_manager,
                                proton_version_to_download,
                                Some(&tracker),
                                "runtime_setup",
                            )
                            .await
                        };
                        
                        match &proton_result {
                            Ok(downloaded_version) => {
                                debug!("Proton downloaded successfully: version {}", downloaded_version);
                                state_signal.write().set_proton_ready(true);
                                
                                // Update default_preferences with the downloaded Proton version
                                if let Ok(mut settings_guard) = settings_arc.write() {
                                    settings_guard.default_preferences.runner = 
                                        backend::runners::Runners::Proton(backend::runners::Proton {
                                            version: downloaded_version.clone(),
                                        });
                                    if let Err(_e) = settings_guard.save() {
                                        debug_error!("Failed to save settings after Proton download");
                                    }
                                }
                            }
                            Err(_e) => {
                                debug_error!("Failed to download Proton");
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                        proton_result.map(|_| ())
                    } else {
                        // Everything is already installed
                        debug!("All Proton components already installed");
                        state_signal.write().set_proton_ready(true);
                        state_signal.write().set_umu_ready(true);
                        state_signal.write().set_steamrt_ready(true);
                        Ok(())
                    }
                }
            };

            if setup_result.is_ok() {
                // Clone settings data before await to avoid holding lock
                let settings_data = settings_arc.read().ok().map(|guard| guard.clone());
                if let Some(settings) = settings_data {
                    let manager_guard = manager_arc.read().await;
                    let new_status = SystemStatus::check(&settings, &manager_guard);
                    system_status.set(Some(new_status));
                }
            }

            tracker.clear("runtime_setup");
            state_signal.write().set_runtime_active(false);
        });
    })
}

/// Create an event handler for tweaks setup
pub fn create_tweaks_setup_handler(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    progress_tracker: ProgressTracker,
    mut system_status: Signal<Option<SystemStatus>>,
    game_state: GlobalGameStateSignal,
    component_service: Signal<Option<crate::services::ComponentService>>,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        let game_id_clone = game_id.clone();
        
        let mut state_signal = game_state;
        
        // Atomically check and set the active flag
        {
            let mut state = state_signal.write();
            if state.get_tweaks_state(&game_id_clone).active {
                debug!("Tweaks setup already in progress for {}, ignoring duplicate click", game_id_clone);
                return;
            }
            // Set active flag immediately to block other clicks
            state.set_tweaks_active(&game_id_clone, true);
        }

        let settings_arc = settings.read().clone();
        let tracker = progress_tracker.clone();

        let service_opt = component_service.read().clone();
        let Some(service) = service_opt else {
            debug!("ComponentService not initialized yet");
            // Reset the flag since we're not actually starting
            state_signal.write().set_tweaks_active(&game_id_clone, false);
            return;
        };

        tracker.report(
            "tweaks_setup",
            "Tweaks Setup",
            backend::progress::ReportParams {
                downloaded: 0,
                total: 0,
                is_busy: true,
                step_index: None,
                total_steps: None,
            },
        );

        spawn_forever(async move {
            let settings_data = if let Ok(s) = settings_arc.read() { s.clone() } else {
                state_signal.write().set_tweaks_active(&game_id_clone, false);
                return;
            };

            let manager_arc = service.manager();

            // Refresh component in a scope
            let refresh_result = {
                let mut component_manager = manager_arc.write().await;
                component_manager
                    .refresh_component(ComponentType::Jadeite)
                    .await
            };

            if let Err(_e) = refresh_result {
                debug_error!("Failed to refresh Jadeite");
                state_signal.write().set_tweaks_active(&game_id_clone, false);
                tracker.clear("tweaks_setup");
                return;
            }

            // Download Jadeite in a scope
            let download_result = {
                let component_manager = manager_arc.read().await;
                component_manager
                    .download_jadeite(
                        &settings_data,
                        None,
                        Some(Box::new({
                            let tracker = tracker.clone();
                            move |current, total| {
                                tracker.report("tweaks_setup", "Jadeite", backend::progress::ReportParams {
                                    downloaded: current,
                                    total,
                                    is_busy: true,
                                    step_index: None,
                                    total_steps: None,
                                });
                            }
                        })),
                    )
                    .await
            };

            match download_result {
                Ok(_) => {
                    state_signal.write().set_tweaks_ready(&game_id_clone, true);

                    // Cleanup old versions
                    let cleanup_result = {
                        let component_manager = manager_arc.read().await;
                        component_manager.cleanup_old_versions(&settings_data, ComponentType::Jadeite)
                    };
                    
                    if let Err(_e) = cleanup_result {
                        debug_error!("Failed to cleanup old Jadeite versions");
                    }

                    // Clone settings data before await to avoid holding lock
                    let settings_data = settings_arc.read().ok().map(|guard| guard.clone());
                    if let Some(settings) = settings_data {
                        let manager_guard = manager_arc.read().await;
                        let new_status = SystemStatus::check(&settings, &manager_guard);
                        system_status.set(Some(new_status));
                    }
                }
                Err(_e) => {
                    debug_error!("Failed to download Jadeite");
                }
            }

            tracker.clear("tweaks_setup");
            state_signal.write().set_tweaks_active(&game_id_clone, false);
        });
    })
}
