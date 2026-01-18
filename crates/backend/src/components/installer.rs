use crate::components::{ComponentManager, ComponentType, ComponentVersion, steamrt};
use crate::progress::ProgressTracker;
use crate::settings::GlobalSettings;
use anyhow::Result;
use reqwest::Url;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ComponentRequirement {
    pub component_type: ComponentType,
    pub display_name: String,
}

/// # Errors
/// Returns an error if component installation fails.
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
                        crate::progress::ReportParams {
                            downloaded: 0,
                            total: 100,
                            is_busy: true,
                            step_index: Some(step_idx),
                            total_steps: Some(total_steps),
                        },
                    );
                }

                let progress_callback = progress_tracker.map(|tracker| {
                    let tracker = tracker.clone();
                    let key = progress_key.to_string();
                    let name = format!("Steam Runtime {}", steamrt::STEAMRT_VERSION);
                    Box::new(move |downloaded, total| {
                        tracker.report(
                            &key,
                            &name,
                            crate::progress::ReportParams {
                                downloaded,
                                total,
                                is_busy: true,
                                step_index: Some(step_idx),
                                total_steps: Some(total_steps),
                            },
                        );
                    }) as Box<dyn Fn(u64, u64) + Send>
                });

                steamrt::download_steamrt(settings, progress_callback).await?;
            }
        } else {
            if component_manager.is_installed(settings, req.component_type) {
                continue;
            }

            // Get the version that will be downloaded to use its display_name
            let version_to_download = component_manager.get_latest_version(req.component_type);
            let display_name = version_to_download.map_or_else(|| req.display_name.clone(), |v| v.display_name.clone());

            if let Some(tracker) = progress_tracker {
                tracker.report(
                    progress_key,
                    &display_name,
                    crate::progress::ReportParams {
                        downloaded: 0,
                        total: 100,
                        is_busy: true,
                        step_index: Some(step_idx),
                        total_steps: Some(total_steps),
                    },
                );
            }

            let progress_callback = progress_tracker.map(|tracker| {
                let tracker = tracker.clone();
                let key = progress_key.to_string();
                let name = display_name.clone();
                Box::new(move |downloaded, total| {
                    tracker.report(
                        &key,
                        &name,
                        crate::progress::ReportParams {
                            downloaded,
                            total,
                            is_busy: true,
                            step_index: Some(step_idx),
                            total_steps: Some(total_steps),
                        },
                    );
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

/// # Errors
/// Returns an error if Proton runtime installation fails.
/// # Panics
/// Panics if the hardcoded UMU URL is invalid.
#[allow(clippy::too_many_lines)]
pub async fn install_proton_runtime(
    settings: &GlobalSettings,
    component_manager: &mut ComponentManager,
    progress_tracker: Option<&ProgressTracker>,
    progress_key: &str,
) -> Result<()> {
    let mut requirements = Vec::new();

    let should_download_umu = !component_manager.is_installed(settings, ComponentType::Umu)
        || component_manager.needs_update(settings, ComponentType::Umu);

    if should_download_umu {
        // Manually add UMU 1.3.0 to cache to avoid rate limiting from GitHub API
        let umu_version = ComponentVersion {
            version: "1.3.0".to_string(),
            download_url: Url::parse("https://github.com/Open-Wine-Components/umu-launcher/releases/download/1.3.0/umu-launcher-1.3.0-zipapp.tar")
                .expect("Failed to parse hardcoded UMU URL"),
            display_name: "1.3.0".to_string(),
            source: None,
        };

        component_manager
            .cache
            .entries
            .entry(ComponentType::Umu)
            .or_default()
            .push(umu_version);

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
                tracker.report(
                    progress_key,
                    &req.display_name,
                    crate::progress::ReportParams {
                        downloaded: 0,
                        total: 100,
                        is_busy: true,
                        step_index: Some(step_idx),
                        total_steps: Some(total_steps),
                    },
                );
            }

            let progress_callback = progress_tracker.map(|tracker| {
                let tracker = tracker.clone();
                let key = progress_key.to_string();
                let name = req.display_name.clone();
                Box::new(move |downloaded, total| {
                    tracker.report(
                        &key,
                        &name,
                        crate::progress::ReportParams {
                            downloaded,
                            total,
                            is_busy: true,
                            step_index: Some(step_idx),
                            total_steps: Some(total_steps),
                        },
                    );
                }) as Box<dyn Fn(u64, u64) + Send>
            });

            steamrt::download_steamrt(settings, progress_callback).await?;
        } else {
            // Get the version that will be downloaded to use its display_name
            let version = if req.component_type == ComponentType::Umu {
                Some("1.3.0")
            } else {
                None
            };

            let version_to_download = if let Some(ver) = version {
                component_manager
                    .cache
                    .entries
                    .get(&req.component_type)
                    .and_then(|versions| versions.iter().find(|v| v.version == ver))
            } else {
                component_manager.get_latest_version(req.component_type)
            };

            let display_name = version_to_download.map_or_else(|| req.display_name.clone(), |v| v.display_name.clone());

            if let Some(tracker) = progress_tracker {
                tracker.report(
                    progress_key,
                    &display_name,
                    crate::progress::ReportParams {
                        downloaded: 0,
                        total: 100,
                        is_busy: true,
                        step_index: Some(step_idx),
                        total_steps: Some(total_steps),
                    },
                );
            }

            let progress_callback = progress_tracker.map(|tracker| {
                let tracker = tracker.clone();
                let key = progress_key.to_string();
                let name = display_name.clone();
                Box::new(move |downloaded, total| {
                    tracker.report(
                        &key,
                        &name,
                        crate::progress::ReportParams {
                            downloaded,
                            total,
                            is_busy: true,
                            step_index: Some(step_idx),
                            total_steps: Some(total_steps),
                        },
                    );
                }) as Box<dyn Fn(u64, u64) + Send>
            });

            component_manager
                .download_component(settings, req.component_type, version, progress_callback)
                .await?;

            component_manager.cleanup_old_versions(settings, req.component_type)?;
        }
    }

    if let Some(tracker) = progress_tracker {
        tracker.finish(progress_key);
    }

    Ok(())
}

/// # Errors
/// Returns an error if tweak installation fails.
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

    // Get the version that will be downloaded to use its display_name
    let version_to_download = component_manager.get_latest_version(ComponentType::Jadeite);
    let display_name = version_to_download.map_or_else(|| "Jadeite".to_string(), |v| v.display_name.clone());

    if let Some(tracker) = progress_tracker {
        tracker.report(progress_key, &display_name, crate::progress::ReportParams {
            downloaded: 0,
            total: 100,
            is_busy: true,
            step_index: Some(0),
            total_steps: Some(1),
        });
    }

    let progress_callback = progress_tracker.map(|tracker| {
        let tracker = tracker.clone();
        let key = progress_key.to_string();
        let name = display_name.clone();
        Box::new(move |downloaded: u64, total: u64| {
            tracker.report(&key, &name, crate::progress::ReportParams {
                downloaded,
                total,
                is_busy: true,
                step_index: Some(0),
                total_steps: Some(1),
            });
        }) as Box<dyn Fn(u64, u64) + Send>
    });

    component_manager
        .download_component(settings, ComponentType::Jadeite, None, progress_callback)
        .await?;

    component_manager.cleanup_old_versions(settings, ComponentType::Jadeite)?;

    if let Some(tracker) = progress_tracker {
        tracker.finish(progress_key);
    }

    Ok(())
}
