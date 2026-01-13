mod proton_section;
mod wine_section;

pub use proton_section::ProtonSection;
pub use wine_section::WineSection;

use crate::components::ComponentDownloadProgress;
use crate::pages::settings_modal::components::{RunnerToggleButton, StylizedDropdown};
use crate::pages::settings_modal::styles::*;
use crate::pages::settings_modal::types::{ComponentVersionInfo, RunnerType};
use backend::settings::GlobalSettings;
use freya::prelude::*;
use std::sync::{Arc, RwLock};

/// Runner configuration page for Wine and Proton settings
#[component]
pub fn RunnerPage(
    selected_runner_type: Signal<RunnerType>,
    selected_proton: Signal<String>,
    selected_wine: Signal<String>,
    selected_dxvk: Signal<String>,
    wine_versions: Signal<Vec<ComponentVersionInfo>>,
    proton_versions: Signal<Vec<ComponentVersionInfo>>,
    dxvk_versions: Signal<Vec<ComponentVersionInfo>>,
    downloading_wine: Signal<bool>,
    downloading_proton: Signal<bool>,
    downloading_dxvk: Signal<bool>,
    wine_installed: Signal<bool>,
    proton_installed: Signal<bool>,
    dxvk_installed: Signal<bool>,
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    component_download_progress: Signal<Option<ComponentDownloadProgress>>,
    component_progress_tracker: Signal<backend::progress::ProgressTracker>,
) -> Element {
    let is_wine_selected = *selected_runner_type.read() == RunnerType::Wine;

    rsx! {
        rect {
            width: "fill",
            direction: "vertical",
            spacing: "28",
            padding: "0 16 16 0",

            rect {
                direction: "vertical",
                spacing: "12",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Runner Type"
                }

                label {
                    font_size: "13",
                    color: "{TEXT_SECONDARY}",
                    "Choose between Proton or Wine"
                }

                rect {
                    width: "400",
                    height: "48",
                    background: "rgb(35, 35, 45, 0.9)",
                    backdrop_blur: "12",
                    corner_radius: "10",
                    border: "1.5 solid {INTERACTIVE_BORDER}",
                    padding: "3",
                    direction: "horizontal",
                    spacing: "3",
                    overflow: "clip",

                    RunnerToggleButton {
                        runner_type: RunnerType::Proton,
                        selected: *selected_runner_type.read() == RunnerType::Proton,
                        on_select: move |rt| selected_runner_type.set(rt),
                    }

                    RunnerToggleButton {
                        runner_type: RunnerType::Wine,
                        selected: *selected_runner_type.read() == RunnerType::Wine,
                        on_select: move |rt| selected_runner_type.set(rt),
                    }
                }
            }

            if is_wine_selected {
                WineSection {
                    selected_wine,
                    selected_dxvk,
                    wine_versions,
                    dxvk_versions,
                    downloading_wine,
                    downloading_dxvk,
                    wine_installed,
                    dxvk_installed,
                    settings_sig,
                    component_download_progress,
                    component_progress_tracker,
                }
            } else {
                ProtonSection {
                    selected_proton,
                    proton_versions,
                    downloading_proton,
                    proton_installed,
                    settings_sig,
                    component_download_progress,
                    component_progress_tracker,
                }
            }
        }
    }
}
