use std::sync::{Arc, RwLock};
use freya::prelude::*;

use crate::components::{DownloadProgress, SetupProgress, SetupStep};
use backend::{
    settings::GlobalSettings,
    progress::ProgressTracker,
    components::{ComponentManager, ComponentType, ComponentVersion},
    status::SystemStatus,
};

pub fn create_runtime_progress_getter(
    progress_tracker: ProgressTracker,
) -> std::rc::Rc<dyn Fn(&str) -> Option<SetupProgress>> {
    std::rc::Rc::new(move |key: &str| -> Option<SetupProgress> {
        let comp_progress = progress_tracker.get(key)?;
        
        if comp_progress.is_finished {
            return None;
        }
        
        let step = match comp_progress.component_name.as_str() {
            "UMU Launcher" => SetupStep::DownloadUmu,
            "DXVK" => SetupStep::DownloadDxvk,
            name if name.starts_with("Steam Runtime") => SetupStep::DownloadSteamRuntime,
            _ => SetupStep::CheckDependencies,
        };
        
        let status = if comp_progress.downloaded == comp_progress.total 
            && comp_progress.total > 0 
            && comp_progress.is_busy {
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
            && comp_progress.is_busy {
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

pub fn create_runtime_setup_handler(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    mut runtime_active: Signal<bool>,
    mut runtime_ready: Signal<bool>,
    mut runtime_needs_update: Signal<bool>,
    progress_tracker: ProgressTracker,
    mut system_status: Signal<Option<SystemStatus>>,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        runtime_active.set(true);

        let settings_arc = settings.read().clone();
        let tracker = progress_tracker.clone();

        spawn(async move {
            let settings_guard = match settings_arc.read() {
                Ok(s) => s,
                Err(_) => {
                    runtime_active.set(false);
                    return;
                }
            };

            let mut component_manager = ComponentManager::new().await;
            
            component_manager.cache.entries.insert(
                ComponentType::Umu,
                vec![ComponentVersion {
                    version: "1.3.0".to_string(),
                    download_url: "https://github.com/Open-Wine-Components/umu-launcher/releases/download/1.3.0/umu-launcher-1.3.0-zipapp.tar".parse().unwrap(),
                }]
            );

            match backend::runners::Runners::download_proton_runtime(
                &settings_guard,
                &component_manager,
                Some(&tracker),
                "runtime_setup",
            ).await {
                Ok(_) => {
                    runtime_ready.set(true);
                    runtime_needs_update.set(false);
                    
                    drop(settings_guard);
                    if let Ok(s) = settings_arc.read() {
                        let new_status = SystemStatus::check(&s).await;
                        system_status.set(Some(new_status));
                    }
                }
                Err(e) => {
                    eprintln!("[RUNTIME_SETUP] Failed: {}", e);
                }
            }

            tracker.clear("runtime_setup");
            runtime_active.set(false);
        });
    })
}

pub fn create_tweaks_setup_handler(
    settings: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    mut tweaks_active: Signal<bool>,
    mut tweaks_ready: Signal<bool>,
    mut tweaks_need_update: Signal<bool>,
    progress_tracker: ProgressTracker,
    mut system_status: Signal<Option<SystemStatus>>,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        tweaks_active.set(true);

        let settings_arc = settings.read().clone();
        let game_id_owned = game_id.clone();
        let tracker = progress_tracker.clone();

        spawn(async move {
            let settings_guard = match settings_arc.read() {
                Ok(s) => s,
                Err(_) => {
                    tweaks_active.set(false);
                    return;
                }
            };

            let mut component_manager = ComponentManager::new().await;
            
            if let Err(e) = component_manager.refresh_component(ComponentType::Jadeite).await {
                eprintln!("[TWEAKS_SETUP] Failed to refresh Jadeite: {}", e);
                tweaks_active.set(false);
                tracker.clear("tweaks_setup");
                return;
            }

            match backend::runners::Runners::download_tweaks(
                &settings_guard,
                &game_id_owned,
                &component_manager,
                Some(&tracker),
                "tweaks_setup",
            ).await {
                Ok(_) => {
                    tweaks_ready.set(true);
                    tweaks_need_update.set(false);
                    
                    drop(settings_guard);
                    if let Ok(s) = settings_arc.read() {
                        let new_status = SystemStatus::check(&s).await;
                        system_status.set(Some(new_status));
                    }
                }
                Err(e) => {
                    eprintln!("[TWEAKS_SETUP] Failed: {}", e);
                }
            }

            tracker.clear("tweaks_setup");
            tweaks_active.set(false);
        });
    })
}
