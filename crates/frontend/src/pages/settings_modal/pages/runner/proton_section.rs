use crate::components::ComponentDownloadProgress;
use crate::pages::settings_modal::components::ComponentVersionSection;
use crate::pages::settings_modal::styles::{TEXT_SECONDARY, INTERACTIVE_BG_SELECTED, INTERACTIVE_BG_SELECTED_HOVER, INTERACTIVE_BORDER_SELECTED, TEXT_SELECTED};
use crate::pages::settings_modal::types::ComponentVersionInfo;
use backend::components::ComponentType;
use backend::settings::GlobalSettings;
use freya::prelude::*;
use std::sync::{Arc, RwLock};

/// Helper function to check if a component is installed
fn check_component_installation(path: &std::path::Path) -> bool {
    path.exists() && std::fs::read_dir(path)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

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
    let component_service = use_context::<Signal<Option<crate::services::ComponentService>>>();
    
    // Track UMU and SteamRuntime installation status
    let mut umu_installed = use_signal(|| false);
    let mut steamrt_installed = use_signal(|| false);
    let mut downloading_runtime = use_signal(|| false);
    let runtime_download_progress = use_signal(|| None::<ComponentDownloadProgress>);

    // Poll runtime download progress separately from Proton download
    use_effect(move || {
        let tracker = component_progress_tracker.read().clone();
        let mut progress_signal = runtime_download_progress;
        spawn(async move {
            loop {
                if let Some(progress) = tracker.get("runtime_download") {
                    progress_signal.set(Some(ComponentDownloadProgress {
                        component_name: progress.component_name.clone(),
                        downloaded: progress.downloaded,
                        total: progress.total,
                        status: if progress.downloaded == progress.total && progress.total > 0 {
                            "Extracting...".to_string()
                        } else {
                            "Downloading...".to_string()
                        },
                        is_active: progress.is_busy,
                    }));
                } else if progress_signal.read().is_some() {
                    progress_signal.set(None);
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    });

    // Check if UMU and SteamRuntime are installed when proton is installed
    use_effect(use_reactive!(|proton_installed| {
        let is_installed = proton_installed();
        if is_installed {
            let settings = settings_sig.read();
            let settings_data = settings.read().expect("Settings lock poisoned");
            let components_dir = settings_data.components_directory.clone();

            spawn(async move {
                // Check UMU installation
                let umu_path = components_dir.join("umu");
                umu_installed.set(check_component_installation(&umu_path));

                // Check SteamRuntime installation
                let steamrt_path = components_dir.join("steamrt");
                steamrt_installed.set(check_component_installation(&steamrt_path));
            });
        }
    }));

    let runtime_needs_install = *proton_installed.read() && (!*umu_installed.read() || !*steamrt_installed.read());

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

        // Show runtime dependencies section if Proton is installed but runtime is missing
        if runtime_needs_install {
            rect {
                direction: "vertical",
                spacing: "12",
                padding: "16 0 0 0",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Proton Runtime Dependencies"
                }

                label {
                    font_size: "13",
                    color: "{TEXT_SECONDARY}",
                    "Proton requires UMU Launcher and Steam Runtime to function"
                }

                if !*umu_installed.read() {
                    label {
                        font_size: "13",
                        color: "rgb(255, 200, 100)",
                        "⚠ UMU Launcher is not installed"
                    }
                }

                if !*steamrt_installed.read() {
                    label {
                        font_size: "13",
                        color: "rgb(255, 200, 100)",
                        "⚠ Steam Runtime is not installed"
                    }
                }

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
                                background: if *downloading_runtime.read() {
                                    "rgb(60, 60, 75, 0.5)".into()
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
                                if *downloading_runtime.read() {
                                    return;
                                }

                                let settings = settings_sig.read();
                                let settings_data = settings.read().expect("Settings lock poisoned").clone();
                                let service_option = component_service.read().clone();
                                let tracker = component_progress_tracker.read().clone();

                                downloading_runtime.set(true);

                                spawn(async move {
                                    let Some(service) = service_option else {
                                        downloading_runtime.set(false);
                                        return;
                                    };

                                    let manager_arc = service.manager();
                                    let mut component_manager = manager_arc.write().await;

                                    // Download both UMU and SteamRuntime using a separate progress key
                                    if let Err(e) = backend::components::install_proton_runtime(
                                        &settings_data,
                                        &mut component_manager,
                                        Some(&tracker),
                                        "runtime_download",
                                    ).await {
                                        // Log error for debugging
                                        use crate::debug_error;
                                        debug_error!("Failed to install Proton runtime: {}", e);
                                    }

                                    // Update installation status
                                    let components_dir = settings_data.components_directory.clone();
                                    
                                    let umu_path = components_dir.join("umu");
                                    umu_installed.set(check_component_installation(&umu_path));

                                    let steamrt_path = components_dir.join("steamrt");
                                    steamrt_installed.set(check_component_installation(&steamrt_path));

                                    downloading_runtime.set(false);
                                    tracker.clear("runtime_download");
                                });
                            },
                            label {
                                font_size: "15",
                                font_weight: "600",
                                if *downloading_runtime.read() {
                                    "Downloading Runtime..."
                                } else {
                                    "Download Runtime Dependencies"
                                }
                            }
                        }
                    }
                }

                // Show download progress for runtime
                if let Some(ref progress) = *runtime_download_progress.read() {
                    if progress.is_active && *downloading_runtime.read() {
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
            }
        }
    }
}
