use freya::prelude::*;
use std::sync::{Arc, RwLock};

use crate::components::{DownloadProgress, SetupProgress, SetupStep};
use crate::{debug, debug_error, debug_info};
use backend::{
    components::{ComponentManager, ComponentType, ComponentVersion},
    progress::ProgressTracker,
    settings::GlobalSettings,
    status::SystemStatus,
};

use super::state::GlobalGameStateSignal;

/// Create a progress getter for runtime component downloads
pub fn create_runtime_progress_getter(
    progress_tracker: ProgressTracker,
) -> std::rc::Rc<dyn Fn(&str) -> Option<SetupProgress>> {
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
) -> std::rc::Rc<dyn Fn(&str) -> Option<SetupProgress>> {
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
        let _missing = state_signal.read().get_missing_components();

        debug!(
            "Starting download for {:?}, missing: {:?}",
            runner_type, _missing
        );

        let service_opt = component_service.read().clone();
        let Some(service) = service_opt else {
            debug!("ComponentService not initialized yet");
            return;
        };

        spawn(async move {
            state_signal.write().set_runtime_active(true);

            let settings_guard = match settings_arc.read() {
                Ok(s) => s,
                Err(_) => {
                    debug_error!("Failed to acquire settings lock");
                    state_signal.write().set_runtime_active(false);
                    return;
                }
            };

            debug!("Re-checking component readiness...");
            match runner_type {
                super::state::RunnerType::Wine => {
                    let wine_installed = service
                        .is_installed(&settings_guard, backend::components::ComponentType::Wine);
                    let dxvk_installed = service
                        .is_installed(&settings_guard, backend::components::ComponentType::Dxvk);
                    state_signal.write().set_wine_ready(wine_installed);
                    state_signal.write().set_dxvk_ready(dxvk_installed);
                    debug!(
                        "Wine ready: {}, DXVK ready: {}",
                        wine_installed, dxvk_installed
                    );
                }
                super::state::RunnerType::Proton => {
                    let proton_installed = service
                        .is_installed(&settings_guard, backend::components::ComponentType::Proton);
                    let umu_installed = service
                        .is_installed(&settings_guard, backend::components::ComponentType::Umu);
                    let steamrt_installed = service.is_installed(
                        &settings_guard,
                        backend::components::ComponentType::SteamRuntime,
                    );

                    let all_ready = proton_installed && umu_installed && steamrt_installed;
                    state_signal.write().set_proton_ready(all_ready);
                    debug!(
                        "Proton ready: {} (proton: {}, umu: {}, steamrt: {})",
                        all_ready, proton_installed, umu_installed, steamrt_installed
                    );
                }
            }

            let _missing = state_signal.read().get_missing_components();
            debug!(
                "After re-check, missing components: {:?}",
                _missing
            );

            let manager_arc = service.manager();
            let mut component_manager = match manager_arc.try_write() {
                Ok(cm) => cm,
                Err(_) => {
                    debug_error!(
                        "ComponentManager is busy, please wait for current download to finish"
                    );
                    tracker.report("runtime_setup", "Runtime Setup", 0, 0, false, None, None);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    tracker.clear("runtime_setup");
                    state_signal.write().set_runtime_active(false);
                    return;
                }
            };

            match runner_type {
                super::state::RunnerType::Wine => {
                    if !state_signal.read().component_setup.wine_ready {
                        debug!("Downloading Wine...");
                        match backend::runners::Runners::download_wine(
                            &settings_guard,
                            &component_manager,
                            None,
                            Some(&tracker),
                            "runtime_setup",
                        )
                        .await
                        {
                            Ok(_) => {
                                state_signal.write().set_wine_ready(true);
                                debug!("Wine downloaded successfully");
                            }
                            Err(_e) => {
                                debug_error!("Failed to download Wine: {}", _e);
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                    }

                    if !state_signal.read().component_setup.dxvk_ready {
                        debug!("Downloading DXVK...");
                        match backend::runners::Runners::download_dxvk(
                            &settings_guard,
                            &component_manager,
                            None,
                            Some(&tracker),
                            "runtime_setup",
                        )
                        .await
                        {
                            Ok(_) => {
                                state_signal.write().set_dxvk_ready(true);
                                debug!("DXVK downloaded successfully");
                            }
                            Err(_e) => {
                                debug_error!("Failed to download DXVK: {}", _e);
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                    }
                }
                super::state::RunnerType::Proton => {
                    if !state_signal.read().component_setup.proton_ready {
                        debug!("Downloading Proton runtime components...");

                        match backend::runners::Runners::download_proton_runtime(
                            &settings_guard,
                            &mut *component_manager,
                            Some(&tracker),
                            "runtime_setup",
                        )
                        .await
                        {
                            Ok(_) => {
                                debug!("UMU and SteamRT downloaded successfully");
                            }
                            Err(_e) => {
                                debug_error!("Failed to download UMU/SteamRT: {}", _e);
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }

                        debug!("Step 3/3: Downloading Proton...");

                        match backend::runners::Runners::download_proton(
                            &settings_guard,
                            &mut *component_manager,
                            None,
                            Some(&tracker),
                            "runtime_setup",
                        )
                        .await
                        {
                            Ok(_) => {
                                state_signal.write().set_proton_ready(true);
                                debug!("All Proton components downloaded successfully");
                            }
                            Err(_e) => {
                                debug_error!("Failed to download Proton: {}", _e);
                                tracker.clear("runtime_setup");
                                state_signal.write().set_runtime_active(false);
                                return;
                            }
                        }
                    }
                }
            }

            drop(settings_guard);
            if let Ok(s) = settings_arc.read() {
                let new_status = SystemStatus::check(&s).await;
                system_status.set(Some(new_status));
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
            "Initializing Tweaks Setup",
            0,
            0,
            true,
            None,
            None,
        );

        spawn(async move {
            let settings_guard = match settings_arc.read() {
                Ok(s) => s,
                Err(_) => {
                    state_signal.write().set_tweaks_active(&game_id_clone, false);
                    return;
                }
            };

            // Re-check if Jadeite (tweaks) is already installed before downloading
            let jadeite_installed = service.is_installed(
                &settings_guard,
                backend::components::ComponentType::Jadeite,
            );
            state_signal.write().set_tweaks_ready(&game_id_clone, jadeite_installed);
            
            if jadeite_installed {
                state_signal.write().set_tweaks_active(&game_id_clone, false);
                tracker.clear("tweaks_setup");
                return;
            }

            let manager_arc = service.manager();
            let mut component_manager = match manager_arc.try_write() {
                Ok(cm) => cm,
                Err(_) => {
                    debug_error!(
                        "ComponentManager is busy, please wait for current download to finish"
                    );
                    state_signal.write().set_tweaks_active(&game_id_clone, false);
                    tracker.report("tweaks_setup", "Tweaks Setup", 0, 0, false, None, None);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    tracker.clear("tweaks_setup");
                    return;
                }
            };

            if let Err(_e) = component_manager
                .refresh_component(ComponentType::Jadeite)
                .await
            {
                debug_error!("Failed to refresh Jadeite: {}", _e);
                state_signal.write().set_tweaks_active(&game_id_clone, false);
                tracker.clear("tweaks_setup");
                return;
            }

            // Download Jadeite using ComponentManager
            match component_manager
                .download_jadeite(
                    &settings_guard,
                    None,
                    Some(Box::new({
                        let tracker = tracker.clone();
                        move |current, total| {
                            tracker.report("tweaks_setup", "Jadeite", current, total, true, None, None);
                        }
                    })),
                )
                .await
            {
                Ok(_) => {
                    state_signal.write().set_tweaks_ready(&game_id_clone, true);

                    drop(settings_guard);
                    if let Ok(s) = settings_arc.read() {
                        let new_status = SystemStatus::check(&s).await;
                        system_status.set(Some(new_status));
                    }
                }
                Err(_e) => {
                    debug_error!("Failed to download Jadeite: {}", _e);
                }
            }

            tracker.clear("tweaks_setup");
            state_signal.write().set_tweaks_active(&game_id_clone, false);
        });
    })
}
