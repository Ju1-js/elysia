use crate::components::ComponentDownloadProgress;
use crate::pages::settings_modal::components::ComponentVersionSection;
use crate::pages::settings_modal::types::ComponentVersionInfo;
use backend::components::ComponentType;
use backend::settings::GlobalSettings;
use freya::prelude::*;
use std::sync::{Arc, RwLock};

#[component]
pub fn WineSection(
    selected_wine: Signal<String>,
    selected_dxvk: Signal<String>,
    wine_versions: Signal<Vec<ComponentVersionInfo>>,
    dxvk_versions: Signal<Vec<ComponentVersionInfo>>,
    downloading_wine: Signal<bool>,
    downloading_dxvk: Signal<bool>,
    wine_installed: Signal<bool>,
    dxvk_installed: Signal<bool>,
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    component_download_progress: Signal<Option<ComponentDownloadProgress>>,
    component_progress_tracker: Signal<backend::progress::ProgressTracker>,
) -> Element {
    rsx! {
        ComponentVersionSection {
            component_type: ComponentType::Wine,
            component_display_name: "Wine",
            version_label: "Wine Version",
            description: "Select which Wine version to use",
            selected_version: selected_wine,
            available_versions: wine_versions,
            is_downloading: downloading_wine,
            is_installed: wine_installed,
            settings_signal: settings_sig,
            download_progress: component_download_progress,
            progress_tracker: component_progress_tracker,
            allow_system_version: true,
        }

        ComponentVersionSection {
            component_type: ComponentType::Dxvk,
            component_display_name: "DXVK",
            version_label: "DXVK Version",
            description: "Select DXVK version for Wine (latest 3 versions available)",
            selected_version: selected_dxvk,
            available_versions: dxvk_versions,
            is_downloading: downloading_dxvk,
            is_installed: dxvk_installed,
            settings_signal: settings_sig,
            download_progress: component_download_progress,
            progress_tracker: component_progress_tracker,
        }
    }
}
