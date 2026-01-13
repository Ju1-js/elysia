use crate::components::ComponentDownloadProgress;
use crate::pages::settings_modal::components::ComponentVersionSection;
use crate::pages::settings_modal::types::ComponentVersionInfo;
use backend::components::ComponentType;
use backend::settings::GlobalSettings;
use freya::prelude::*;
use std::sync::{Arc, RwLock};

#[component]
pub fn ProtonSection(
    selected_proton: Signal<String>,
    proton_versions: Signal<Vec<ComponentVersionInfo>>,
    downloading_proton: Signal<bool>,
    proton_installed: Signal<bool>,
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    component_download_progress: Signal<Option<ComponentDownloadProgress>>,
    component_progress_tracker: Signal<backend::progress::ProgressTracker>,
) -> Element {
    rsx! {
        ComponentVersionSection {
            component_type: ComponentType::Proton,
            component_display_name: "Proton",
            version_label: "Proton Version",
            description: "Select which Proton version to use",
            selected_version: selected_proton,
            available_versions: proton_versions,
            is_downloading: downloading_proton,
            is_installed: proton_installed,
            settings_signal: settings_sig,
            download_progress: component_download_progress,
            progress_tracker: component_progress_tracker,
        }
    }
}
