use backend::{components::ComponentType, progress::ProgressTracker, settings::GlobalSettings};
use freya::prelude::*;

use crate::components::ComponentDownloadProgress;
use crate::services::ComponentService;
use crate::{debug_error, debug_info};

/// Result of a component download operation
#[derive(Debug, Clone)]
pub struct ComponentDownloadResult {
    pub success: bool,
    pub installed: bool,
    pub error_message: Option<String>,
}

/// Parameters for initiating a component download
pub struct ComponentDownloadParams {
    pub component_type: ComponentType,
    pub component_display_name: String,
    pub version: String,
    pub version_display_name: String,
}

/// Downloads a component with progress tracking
pub async fn download_component(
    service: &ComponentService,
    settings: &GlobalSettings,
    params: ComponentDownloadParams,
    progress_tracker: &ProgressTracker,
    mut progress_signal: Signal<Option<ComponentDownloadProgress>>,
    mut is_downloading_signal: Signal<bool>,
) -> ComponentDownloadResult {
    // Validate that a version is specified
    if params.version.is_empty() {
        debug_error!(
            "Cannot download {}: no version specified",
            params.component_display_name
        );
        is_downloading_signal.set(false);
        return ComponentDownloadResult {
            success: false,
            installed: false,
            error_message: Some(format!(
                "No version specified for {}",
                params.component_display_name
            )),
        };
    }

    let manager_arc = service.manager();

    let mut component_manager = match manager_arc.try_write() {
        Ok(manager) => manager,
        Err(_) => {
            debug_error!(
                "ComponentManager is busy (download in progress). Please wait and try again."
            );
            progress_signal.set(Some(ComponentDownloadProgress {
                component_name: params.component_display_name.clone(),
                downloaded: 0,
                total: 0,
                status: "ComponentManager is busy. Please wait for current download to finish."
                    .to_string(),
                is_active: false,
            }));
            is_downloading_signal.set(false);

            // Clear the message after a delay
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                progress_signal.set(None);
            });

            return ComponentDownloadResult {
                success: false,
                installed: false,
                error_message: Some("ComponentManager is busy".to_string()),
            };
        }
    };

    debug_info!(
        "Refreshing {} component cache before download...",
        params.component_display_name
    );
    if let Err(error) = component_manager
        .refresh_component(params.component_type)
        .await
    {
        let error_msg = format!(
            "Failed to refresh {} cache: {}",
            params.component_display_name, error
        );
        debug_error!("{}", error_msg);
        is_downloading_signal.set(false);
        progress_signal.set(None);
        return ComponentDownloadResult {
            success: false,
            installed: false,
            error_message: Some(error_msg),
        };
    }

    let version_for_callback = params.version_display_name.clone();
    let tracker_clone = progress_tracker.clone();

    match component_manager
        .download_component(
            settings,
            params.component_type,
            Some(&params.version),
            Some(Box::new(move |downloaded, total| {
                tracker_clone.report(
                    "component_download",
                    &version_for_callback,
                    downloaded,
                    total,
                    true,
                    None,
                    None,
                );
            })),
        )
        .await
    {
        Ok(_) => {
            debug_info!(
                "{} {} downloaded successfully",
                params.component_display_name, params.version
            );

            let installed = component_manager.is_installed(settings, params.component_type);
            debug_info!(
                "Updated {}_installed to: {}",
                params.component_display_name.to_lowercase(),
                installed
            );

            ComponentDownloadResult {
                success: true,
                installed,
                error_message: None,
            }
        }
        Err(error) => {
            let error_msg = format!(
                "Failed to download {}: {}",
                params.component_display_name, error
            );
            debug_error!("{}", error_msg);
            ComponentDownloadResult {
                success: false,
                installed: false,
                error_message: Some(error_msg),
            }
        }
    }
}

/// Initiates a component download with UI state management
pub fn initiate_component_download(
    service_option: Option<ComponentService>,
    settings: GlobalSettings,
    params: ComponentDownloadParams,
    progress_tracker: ProgressTracker,
    mut progress_signal: Signal<Option<ComponentDownloadProgress>>,
    mut is_downloading_signal: Signal<bool>,
    on_complete: impl Fn(ComponentDownloadResult) + 'static,
) {
    debug_info!(
        "Download button pressed for {} version: {} (display: {})",
        params.component_display_name, params.version, params.version_display_name
    );

    // Set initial progress state
    progress_signal.set(Some(ComponentDownloadProgress {
        component_name: params.version_display_name.clone(),
        downloaded: 0,
        total: 0,
        status: "Starting download...".to_string(),
        is_active: true,
    }));

    is_downloading_signal.set(true);

    spawn(async move {
        let Some(service) = service_option else {
            debug_error!("ComponentService not initialized");
            is_downloading_signal.set(false);
            progress_signal.set(None);
            on_complete(ComponentDownloadResult {
                success: false,
                installed: false,
                error_message: Some("ComponentService not initialized".to_string()),
            });
            return;
        };

        let result = download_component(
            &service,
            &settings,
            params,
            &progress_tracker,
            progress_signal,
            is_downloading_signal,
        )
        .await;

        is_downloading_signal.set(false);
        progress_signal.set(None);
        on_complete(result);
    });
}
