use crate::pages::settings_modal::types::ComponentVersionInfo;
use crate::services::ComponentService;
use crate::{debug, debug_error, debug_info};
use backend::runners::Runners;
use backend::settings::GlobalSettings;

/// Load Wine versions including system Wine if available
/// Returns (`is_specific_version_installed`, `version_infos`)
pub async fn load_wine_versions(
    service: &ComponentService,
    settings: &GlobalSettings,
    selected_version: &str,
) -> Option<(bool, Vec<ComponentVersionInfo>)> {
    debug_info!("Loading Wine versions... (checking for: {})", selected_version);

    let wine_runner = Runners::Wine(backend::runners::Wine {
        version: String::new(),
    });

    if let Some((_, current, versions)) = service.get_wine_status(settings, &wine_runner).await {
        debug!("Current installed: {:?}", current);
        debug!("Available: {} versions", versions.len());

        let mut version_infos: Vec<ComponentVersionInfo> = versions
            .iter()
            .map(|component_version| {
                debug!(
                    "{} -> {}",
                    component_version.version, component_version.display_name
                );
                ComponentVersionInfo {
                    internal_name: component_version.version.clone(),
                    display_name: component_version.display_name.clone(),
                }
            })
            .collect();

        // Add System Wine at the END of the list if it exists
        if backend::runners::is_system_wine_available() {
            version_infos.push(
                ComponentVersionInfo {
                    internal_name: "system".to_string(),
                    display_name: "System Wine".to_string(),
                },
            );
            debug!("system -> System Wine (detected from PATH)");
        }

        // Check if the specific selected version is installed
        let specific_installed = if selected_version == "system" {
            backend::runners::is_system_wine_available()
        } else if !selected_version.is_empty() {
            // Check if the selected version directory exists
            let wine_path = settings.components_directory
                .join("wine")
                .join(selected_version);
            wine_path.exists()
        } else {
            // No version selected yet, check if any version is installed
            current.is_some()
        };

        debug!("Selected version '{}' installed: {}", selected_version, specific_installed);
        Some((specific_installed, version_infos))
    } else {
        debug!("get_wine_status returned None");
        None
    }
}

/// Load Proton versions
/// Returns (`is_specific_version_installed`, `version_infos`)
pub async fn load_proton_versions(
    service: &ComponentService,
    settings: &GlobalSettings,
    selected_version: &str,
) -> Option<(bool, Vec<ComponentVersionInfo>)> {
    debug_info!("Loading Proton versions... (checking for: {})", selected_version);

    let proton_runner = Runners::Proton(backend::runners::Proton {
        version: String::new(),
    });

    if let Some((_, current, versions)) = service.get_proton_status(settings, &proton_runner).await {
        debug!("Current installed: {:?}", current);
        debug!("Available: {} versions", versions.len());

        let version_infos: Vec<ComponentVersionInfo> = versions
            .iter()
            .map(|component_version| {
                debug!(
                    "{} -> {}",
                    component_version.version, component_version.display_name
                );
                ComponentVersionInfo {
                    internal_name: component_version.version.clone(),
                    display_name: component_version.display_name.clone(),
                }
            })
            .collect();

        // Check if the specific selected version is installed
        let specific_installed = if selected_version.is_empty() {
            // No version selected yet, check if any version is installed
            current.is_some()
        } else {
            // Check if the selected version directory exists
            let proton_path = settings.components_directory
                .join("proton")
                .join(selected_version);
            proton_path.exists()
        };

        debug!("Selected version '{}' installed: {}", selected_version, specific_installed);
        Some((specific_installed, version_infos))
    } else {
        debug!("get_proton_status returned None");
        None
    }
}

/// Load DXVK versions
/// Returns (`is_specific_version_installed`, `version_infos`)
pub async fn load_dxvk_versions(
    service: &ComponentService,
    settings: &GlobalSettings,
    selected_version: &str,
) -> Option<(bool, Vec<ComponentVersionInfo>)> {
    debug_info!("Loading DXVK versions... (checking for: {})", selected_version);

    let wine_runner = Runners::Wine(backend::runners::Wine {
        version: String::new(),
    });

    if let Some((_, current, versions)) = service.get_dxvk_status(settings, &wine_runner).await {
        debug!("Current installed: {:?}", current);
        debug!("Available: {} versions", versions.len());

        let version_infos: Vec<ComponentVersionInfo> = versions
            .iter()
            .map(|component_version| {
                debug!(
                    "{} -> {}",
                    component_version.version, component_version.display_name
                );
                ComponentVersionInfo {
                    internal_name: component_version.version.clone(),
                    display_name: component_version.display_name.clone(),
                }
            })
            .collect();

        // Check if the specific selected version is installed
        let specific_installed = if selected_version.is_empty() {
            // No version selected yet, check if any version is installed
            current.is_some()
        } else {
            // Check if the selected version directory exists
            let dxvk_path = settings.components_directory
                .join("dxvk")
                .join(selected_version);
            dxvk_path.exists()
        };

        debug!("Selected version '{}' installed: {}", selected_version, specific_installed);
        Some((specific_installed, version_infos))
    } else {
        debug!("get_dxvk_status returned None");
        None
    }
}
