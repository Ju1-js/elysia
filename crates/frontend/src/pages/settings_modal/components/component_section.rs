/// Shared components for displaying and managing component versions (Wine, Proton, DXVK)
///
/// This module consolidates the repeated UI patterns for component selection, download,
/// and status display that were previously duplicated across `wine_section` and `proton_section`.
use crate::components::ComponentDownloadProgress;
use crate::pages::settings_modal::components::StylizedDropdown;
use crate::pages::settings_modal::styles::{TEXT_SECONDARY, INTERACTIVE_BG_DISABLED, INTERACTIVE_BG_SELECTED, INTERACTIVE_BG_SELECTED_HOVER, INTERACTIVE_BORDER_SELECTED, TEXT_SELECTED};
use crate::pages::settings_modal::types::ComponentVersionInfo;
use crate::services::{ComponentDownloadParams, initiate_component_download};
use backend::components::ComponentType;
use backend::settings::GlobalSettings;
use freya::prelude::*;
use std::sync::{Arc, RwLock};

/// Generic component version section that handles selection, download, and status display
#[component]
pub fn ComponentVersionSection(
    component_type: ComponentType,
    component_display_name: &'static str,
    version_label: &'static str,
    description: &'static str,
    selected_version: Signal<String>,
    available_versions: Signal<Vec<ComponentVersionInfo>>,
    is_downloading: Signal<bool>,
    is_installed: Signal<bool>,
    settings_signal: Signal<Arc<RwLock<GlobalSettings>>>,
    download_progress: Signal<Option<ComponentDownloadProgress>>,
    progress_tracker: Signal<backend::progress::ProgressTracker>,
    /// Special handling for system wine (doesn't need to be downloaded)
    #[props(default = false)]
    allow_system_version: bool,
) -> Element {
    let component_service = use_context::<Signal<Option<crate::services::ComponentService>>>();

    // Track if the selected version is installed
    let mut selected_version_installed = use_signal(|| false);

    // Check if selected version is installed when selection or installation status changes
    use_effect(use_reactive!(|selected_version, is_installed| {
        let selected = selected_version();
        let _ = is_installed;
        let settings = settings_signal.read();
        let settings_data = settings.read().expect("Settings lock poisoned");
        let components_dir = settings_data.components_directory.clone();

        spawn(async move {
            // System Wine doesn't need to be "installed" via download
            if allow_system_version && selected == "system" {
                selected_version_installed.set(true);
            } else if !selected.is_empty() {
                // Check if the specific selected version directory exists
                // Map ComponentType to directory name
                let component_dir_name = match component_type {
                    backend::components::ComponentType::Dxvk => "dxvk",
                    backend::components::ComponentType::Wine => "wine",
                    backend::components::ComponentType::Proton => "proton",
                    backend::components::ComponentType::Jadeite => "jadeite",
                    backend::components::ComponentType::Umu => "umu",
                    backend::components::ComponentType::SteamRuntime => "steamrt",
                };
                let version_path = components_dir
                    .join(component_dir_name)
                    .join(&selected);
                let is_current_installed = version_path.exists();
                selected_version_installed.set(is_current_installed);
            } else {
                selected_version_installed.set(false);
            }
        });
    }));

    rsx! {
        rect {
            direction: "vertical",
            spacing: "12",

            label {
                font_size: "18",
                font_weight: "600",
                color: "white",
                "{version_label}"
            }

            label {
                font_size: "13",
                color: "{TEXT_SECONDARY}",
                "{description}"
            }

            if !available_versions.read().is_empty() {
                StylizedDropdown {
                    options: available_versions.read().clone(),
                    selected: selected_version.read().clone(),
                    width: "fill".to_string(),
                    on_select: move |version: String| {
                        selected_version.set(version);
                    }
                }
            }

            // Show download button if selected version is NOT installed
            if !*selected_version_installed.read() {
                ComponentDownloadButton {
                    component_type,
                    component_display_name,
                    selected_version,
                    available_versions,
                    is_downloading,
                    settings_signal,
                    download_progress,
                    component_service,
                    progress_tracker,
                    on_success: move |installed| {
                        is_installed.set(installed);
                        selected_version_installed.set(installed);
                    }
                }

                // Show download progress
                if let Some(ref progress) = *download_progress.read() {
                    if progress.is_active && *is_downloading.read() {
                        DownloadProgressDisplay { progress: progress.clone() }
                    }
                }
            } else {
                // Show status when installed
                InstalledStatusDisplay {}
            }
        }
    }
}

/// Download button component for installing components
#[component]
fn ComponentDownloadButton(
    component_type: ComponentType,
    component_display_name: &'static str,
    selected_version: Signal<String>,
    available_versions: Signal<Vec<ComponentVersionInfo>>,
    is_downloading: Signal<bool>,
    settings_signal: Signal<Arc<RwLock<GlobalSettings>>>,
    download_progress: Signal<Option<ComponentDownloadProgress>>,
    component_service: Signal<Option<crate::services::ComponentService>>,
    progress_tracker: Signal<backend::progress::ProgressTracker>,
    on_success: EventHandler<bool>,
) -> Element {
    rsx! {
        rect {
            direction: "horizontal",
            spacing: "12",
            main_align: "start",

            rect {
                backdrop_blur: "12",
                corner_radius: "10",
                overflow: "clip",

                Button {
                    theme: theme_with!(ButtonTheme {
                        background: if *is_downloading.read() {
                            INTERACTIVE_BG_DISABLED.into()
                        } else {
                            INTERACTIVE_BG_SELECTED.into()
                        },
                        hover_background: INTERACTIVE_BG_SELECTED_HOVER.into(),
                        border_fill: INTERACTIVE_BORDER_SELECTED.into(),
                        focus_border_fill: INTERACTIVE_BORDER_SELECTED.into(),
                        padding: "12 22".into(),
                        corner_radius: "10".into(),
                        shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                        font_theme: theme_with!(FontTheme {
                            color: TEXT_SELECTED.into(),
                        }),
                    }),
                    onpress: move |_| {
                        if *is_downloading.read() {
                            return;
                        }
                        
                        // Set downloading flag immediately to prevent double-clicks
                        is_downloading.set(true);

                        let version = selected_version.read().clone();
                        let settings = settings_signal.read();
                        let settings_data = settings.read().expect("Settings lock poisoned").clone();

                        // Find the display name for the selected version
                        let version_display_name = available_versions.read()
                            .iter()
                            .find(|v| v.internal_name == version).map_or_else(|| version.clone(), |v| v.display_name.clone());

                        let service_option = component_service.read().clone();
                        let tracker = progress_tracker.read().clone();

                        // Use the consolidated download handler
                        let params = ComponentDownloadParams {
                            component_type,
                            component_display_name: component_display_name.to_string(),
                            version: version.clone(),
                            version_display_name,
                        };

                        initiate_component_download(
                            service_option,
                            settings_data,
                            params,
                            tracker,
                            download_progress,
                            is_downloading,
                            move |result| {
                                if result.success {
                                    on_success.call(result.installed);
                                }
                            },
                        );
                    },
                    label {
                        font_size: "15",
                        font_weight: "600",
                        if *is_downloading.read() {
                            "Downloading..."
                        } else {
                            "Download {component_display_name}"
                        }
                    }
                }
            }
        }
    }
}

/// Display download progress with progress bar
#[component]
fn DownloadProgressDisplay(progress: ComponentDownloadProgress) -> Element {
    rsx! {
        rect {
            width: "fill",
            padding: "12",
            background: "rgb(40, 50, 60, 0.5)",
            border: "1.5 solid rgb(80, 120, 160, 0.5)",
            corner_radius: "8",
            direction: "vertical",
            spacing: "8",

            label {
                font_size: "13",
                color: "rgb(180, 200, 220)",
                "{progress.status}"
            }

            if progress.total > 0 {
                rect {
                    width: "100%",
                    height: "6",
                    background: "rgb(40, 40, 50)",
                    corner_radius: "3",
                    overflow: "clip",

                    rect {
                        width: "{(progress.downloaded as f64 / progress.total as f64 * 100.0):.0}%",
                        height: "6",
                        background: "{INTERACTIVE_BG_SELECTED}",
                        corner_radius: "3",
                    }
                }

                label {
                    font_size: "12",
                    color: "rgb(160, 180, 200)",
                    "{(progress.downloaded as f64 / progress.total as f64 * 100.0):.1}%"
                }
            }
        }
    }
}

/// Display installed status indicator
#[component]
fn InstalledStatusDisplay() -> Element {
    rsx! {
        rect {
            padding: "12 16",
            background: "rgb(40, 80, 60, 0.3)",
            border: "1.5 solid rgb(80, 160, 120, 0.5)",
            corner_radius: "8",
            direction: "horizontal",
            spacing: "8",
            cross_align: "center",

            label {
                font_size: "14",
                color: "rgb(150, 220, 180)",
                "✓ Installed"
            }
        }
    }
}
