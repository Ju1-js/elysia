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
) -> EventHandler<PressEvent> {
    debug!("Creating component setup handler");
    EventHandler::new(move |_| {
        debug!("Component setup handler called");
        let settings_arc = settings.read().clone();
        let tracker = progress_tracker.clone();
        let mut state_signal = game_state;

        let runner_type = state_signal.read().current_runner_type.clone();
        let missing = state_signal.read().get_missing_components();

        debug!(
            "Starting download for {:?}, missing: {:?}",
            runner_type, missing
        );

        let service_opt = component_service.read().clone();
        let Some(service) = service_opt else {
            debug!("ComponentService not initialized yet");
            return;
        };

        spawn(async move {
            state_signal.write().set_runtime_active(true);

            let settings_data = if let Ok(s) = settings_arc.read() { s.clone() } else {
                debug_error!("Failed to acquire settings lock");
                state_signal.write().set_runtime_active(false);
                return;
            };

            debug!("Re-checking component readiness...");
            match runner_type {
                super::state::RunnerType::Wine => {
                    let wine_installed = service
                        .is_installed(&settings_data, backend::components::ComponentType::Wine).await;
                    let dxvk_installed = service
                        .is_installed(&settings_data, backend::components::ComponentType::Dxvk).await;
                    state_signal.write().set_wine_ready(wine_installed);
                    state_signal.write().set_dxvk_ready(dxvk_installed);
                    debug!(
                        "Wine ready: {}, DXVK ready: {}",
                        wine_installed, dxvk_installed
                    );
                }
                super::state::RunnerType::Proton => {
                    let proton_installed = service
                        .is_installed(&settings_data, backend::components::ComponentType::Proton).await;
                    let umu_installed = service
                        .is_installed(&settings_data, backend::components::ComponentType::Umu).await;
                    let steamrt_installed = service.is_installed(
                        &settings_data,
                        backend::components::ComponentType::SteamRuntime,
                    ).await;

                    let all_ready = proton_installed && umu_installed && steamrt_installed;
                    state_signal.write().set_proton_ready(all_ready);
                    debug!(
                        "Proton ready: {} (proton: {}, umu: {}, steamrt: {})",
                        all_ready, proton_installed, umu_installed, steamrt_installed
                    );
                }
            }

            let missing = state_signal.read().get_missing_components();
            debug!(
                "After re-check, missing components: {:?}",
                missing
            );

            let manager_arc = service.manager();
            
            // Note: tokio::sync::RwLock always succeeds in acquiring locks eventually
            // There's no need to check availability as the lock is async-aware

            let setup_result: Result<(), anyhow::Error> = match runner_type {
                super::state::RunnerType::Wine => {
                    let mut result = Ok(());
                    
                    if !state_signal.read().component_setup.wine_ready {
                        debug!("Downloading Wine...");
                        let wine_result = {
                            let component_manager = manager_arc.read().await;
                            backend::runners::Runners::download_wine(
                                &settings_data,
                                &component_manager,
                                None,
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
                                    if let Err(e) = settings_guard.save() {
                                        debug_error!("Failed to save settings after Wine download: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                debug_error!("Failed to download Wine: {}", e);
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
                                    if let Err(e) = settings_guard.save() {
                                        debug_error!("Failed to save settings after DXVK download: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                debug_error!("Failed to download DXVK: {}", e);
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

                    // install_proton_runtime already checks needs_update internally,
                    // so we call it regardless of proton_ready status to handle both
                    // initial installation and updates
                    let result = {
                        let mut component_manager = manager_arc.write().await;
                        backend::runners::Runners::download_proton_runtime(
                            &settings_data,
                            &mut component_manager,
                            Some(&tracker),
                            "runtime_setup",
                        )
                        .await
                    };
                    
                    match result {
                        Ok(()) => {
                            debug!("UMU and SteamRT download completed (installed or already up to date)");
                        }
                        Err(ref e) => {
                            debug_error!("Failed to download UMU/SteamRT: {}", e);
                            tracker.clear("runtime_setup");
                            state_signal.write().set_runtime_active(false);
                            return;
                        }
                    }

                    // Only download Proton if it's not ready or needs update
                    if state_signal.read().component_setup.proton_ready {
                        debug!("Proton already installed");
                        Ok(())
                    } else {
                        debug!("Downloading Proton...");

                        let proton_result = {
                            let mut component_manager = manager_arc.write().await;
                            backend::runners::Runners::download_proton(
                                &settings_data,
                                &mut component_manager,
                                None,
                                Some(&tracker),
                                "runtime_setup",
                            )
                            .await
                        };
                        
                        match &proton_result {
                            Ok(downloaded_version) => {
                                state_signal.write().set_proton_ready(true);
                                debug!("All Proton components downloaded successfully: Proton version {}", downloaded_version);
                                
                                // Update default_preferences with the downloaded Proton version
                                if let Ok(mut settings_guard) = settings_arc.write() {
                                    settings_guard.default_preferences.runner = 
                                        backend::runners::Runners::Proton(backend::runners::Proton {
                                            version: downloaded_version.clone(),
                                        });
                                    if let Err(e) = settings_guard.save() {
                                        debug_error!("Failed to save settings after Proton download: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                debug_error!("Failed to download Proton: {}", e);
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                        proton_result.map(|_| ())
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
    mut game_state: GlobalGameStateSignal,
    component_service: Signal<Option<crate::services::ComponentService>>,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        let game_id_clone = game_id.clone();

        game_state.write().set_tweaks_active(&game_id_clone, true);

        let settings_arc = settings.read().clone();
        let tracker = progress_tracker.clone();
        let mut state_signal = game_state;

        let service_opt = component_service.read().clone();
        let Some(service) = service_opt else {
            debug!("ComponentService not initialized yet");
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

        spawn(async move {
            let settings_data = if let Ok(s) = settings_arc.read() { s.clone() } else {
                state_signal.write().set_tweaks_active(&game_id_clone, false);
                return;
            };

            let manager_arc = service.manager();
            
            // Note: tokio::sync::RwLock always succeeds in acquiring locks eventually
            // There's no need to check availability as the lock is async-aware

            // Refresh component in a scope
            let refresh_result = {
                let mut component_manager = manager_arc.write().await;
                component_manager
                    .refresh_component(ComponentType::Jadeite)
                    .await
            };

            if let Err(e) = refresh_result {
                debug_error!("Failed to refresh Jadeite: {}", e);
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
                    
                    if let Err(e) = cleanup_result {
                        debug_error!("Failed to cleanup old Jadeite versions: {}", e);
                    }

                    // Clone settings data before await to avoid holding lock
                    let settings_data = settings_arc.read().ok().map(|guard| guard.clone());
                    if let Some(settings) = settings_data {
                        let manager_guard = manager_arc.read().await;
                        let new_status = SystemStatus::check(&settings, &manager_guard);
                        system_status.set(Some(new_status));
                    }
                }
                Err(e) => {
                    debug_error!("Failed to download Jadeite: {}", e);
                }
            }

            tracker.clear("tweaks_setup");
            state_signal.write().set_tweaks_active(&game_id_clone, false);
        });
    })
}
