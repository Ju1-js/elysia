use anyhow::Result;
use std::collections::HashMap;
use crate::settings::GlobalSettings;
use crate::components::{ComponentManager, ComponentType, steamrt};
use crate::progress::ProgressTracker;

#[derive(Debug, Clone)]
pub struct ComponentRequirement {
    pub component_type: ComponentType,
    pub display_name: String,
}

pub async fn install_components(
    settings: &GlobalSettings,
    component_manager: &ComponentManager,
    requirements: Vec<ComponentRequirement>,
    progress_tracker: Option<&ProgressTracker>,
    progress_key: &str,
) -> Result<HashMap<String, String>> {
    let env_vars = HashMap::new();
    let total_steps = requirements.len();
    
    if total_steps == 0 {
        return Ok(env_vars);
    }

    for (step_idx, req) in requirements.iter().enumerate() {
        if req.component_type == ComponentType::SteamRuntime {
            let steamrt_setup = steamrt::prepare_steamrt(settings).await?;
            if steamrt_setup.needs_download {
                if let Some(tracker) = progress_tracker {
                    tracker.report(
                        progress_key,
                        &format!("Steam Runtime {}", steamrt::STEAMRT_VERSION),
                        0,
                        100,
                        true,
                        Some(step_idx),
                        Some(total_steps),
                    );
                }
                
                let progress_callback = progress_tracker.map(|tracker| {
                    let tracker = tracker.clone();
                    let key = progress_key.to_string();
                    let name = format!("Steam Runtime {}", steamrt::STEAMRT_VERSION);
                    Box::new(move |downloaded, total| {
                        tracker.report(&key, &name, downloaded, total, true, Some(step_idx), Some(total_steps));
                    }) as Box<dyn Fn(u64, u64) + Send>
                });

                steamrt::download_steamrt(settings, progress_callback).await?;
            }
        } else {
            if component_manager.is_installed(settings, req.component_type) {
                continue;
            }
            
            if let Some(tracker) = progress_tracker {
                tracker.report(
                    progress_key,
                    &req.display_name,
                    0,
                    100,
                    true,
                    Some(step_idx),
                    Some(total_steps),
                );
            }
            
            let progress_callback = progress_tracker.map(|tracker| {
                let tracker = tracker.clone();
                let key = progress_key.to_string();
                let name = req.display_name.clone();
                Box::new(move |downloaded, total| {
                    tracker.report(&key, &name, downloaded, total, true, Some(step_idx), Some(total_steps));
                }) as Box<dyn Fn(u64, u64) + Send>
            });

            component_manager
                .download_component(settings, req.component_type, None, progress_callback)
                .await?;
        }
    }

    if let Some(tracker) = progress_tracker {
        tracker.finish(progress_key);
    }

    Ok(env_vars)
}

pub async fn install_proton_runtime(
    settings: &GlobalSettings,
    component_manager: &ComponentManager,
    progress_tracker: Option<&ProgressTracker>,
    progress_key: &str,
) -> Result<()> {
    let mut requirements = Vec::new();
    
    let should_download_umu = !component_manager.is_installed(settings, ComponentType::Umu)
        || component_manager.needs_update(settings, ComponentType::Umu);
    
    if should_download_umu {
        requirements.push(ComponentRequirement {
            component_type: ComponentType::Umu,
            display_name: "UMU Launcher".to_string(),
        });
    }
    
    let steamrt_setup = steamrt::prepare_steamrt(settings).await?;
    if steamrt_setup.needs_download {
        requirements.push(ComponentRequirement {
            component_type: ComponentType::SteamRuntime,
            display_name: "Steam Runtime".to_string(),
        });
    }
    
    if requirements.is_empty() {
        return Ok(());
    }
    
    let total_steps = requirements.len();
    
    for (step_idx, req) in requirements.iter().enumerate() {
        if req.component_type == ComponentType::SteamRuntime {
            if let Some(tracker) = progress_tracker {
                tracker.report(progress_key, &req.display_name, 0, 100, true, Some(step_idx), Some(total_steps));
            }
            
            let progress_callback = progress_tracker.map(|tracker| {
                let tracker = tracker.clone();
                let key = progress_key.to_string();
                let name = req.display_name.clone();
                Box::new(move |downloaded, total| {
                    tracker.report(&key, &name, downloaded, total, true, Some(step_idx), Some(total_steps));
                }) as Box<dyn Fn(u64, u64) + Send>
            });
            
            steamrt::download_steamrt(settings, progress_callback).await?;
        } else {
            if let Some(tracker) = progress_tracker {
                tracker.report(progress_key, &req.display_name, 0, 100, true, Some(step_idx), Some(total_steps));
            }
            
            let progress_callback = progress_tracker.map(|tracker| {
                let tracker = tracker.clone();
                let key = progress_key.to_string();
                let name = req.display_name.clone();
                Box::new(move |downloaded, total| {
                    tracker.report(&key, &name, downloaded, total, true, Some(step_idx), Some(total_steps));
                }) as Box<dyn Fn(u64, u64) + Send>
            });
            
            component_manager.download_component(
                settings,
                req.component_type,
                None,
                progress_callback,
            ).await?;
            
            component_manager.cleanup_old_versions(settings, req.component_type)?;
        }
    }
    
    if let Some(tracker) = progress_tracker {
        tracker.finish(progress_key);
    }
    
    Ok(())
}

pub async fn install_tweaks(
    settings: &GlobalSettings,
    _game_id: &str,
    component_manager: &ComponentManager,
    progress_tracker: Option<&ProgressTracker>,
    progress_key: &str,
) -> Result<()> {
    let should_download = !component_manager.is_installed(settings, ComponentType::Jadeite)
        || component_manager.needs_update(settings, ComponentType::Jadeite);
    
    if !should_download {
        return Ok(());
    }
    
    if let Some(tracker) = progress_tracker {
        tracker.report(progress_key, "Jadeite", 0, 100, true, Some(0), Some(1));
    }
    
    let progress_callback = progress_tracker.map(|tracker| {
        let tracker = tracker.clone();
        let key = progress_key.to_string();
        Box::new(move |downloaded: u64, total: u64| {
            tracker.report(&key, "Jadeite", downloaded, total, true, Some(0), Some(1));
        }) as Box<dyn Fn(u64, u64) + Send>
    });
    
    component_manager.download_component(
        settings,
        ComponentType::Jadeite,
        None,
        progress_callback,
    ).await?;
    
    component_manager.cleanup_old_versions(settings, ComponentType::Jadeite)?;
    
    if let Some(tracker) = progress_tracker {
        tracker.finish(progress_key);
    }
    
    Ok(())
}
