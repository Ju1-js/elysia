use crate::components::{ComponentDownloadProgress, download_control::DownloadProgress};
use crate::pages::game::state::GlobalGameStateSignal;
use crate::pages::settings_modal::types::SettingsPage;
use crate::theme;
use freya::prelude::*;

/// Component to display active downloads in the game settings modal
/// Shows:
/// - Game downloads (always)
/// - Component downloads (only when not already visible inline in Runner page)
#[component]
pub fn ActiveDownloadWidget(
    game_id: String,
    game_state: GlobalGameStateSignal,
    component_download_progress: Signal<Option<ComponentDownloadProgress>>,
    current_page: SettingsPage,
) -> Element {
    // Use a signal to store the current game download progress
    let mut game_download = use_signal(|| None::<DownloadProgress>);

    // Poll the game state for download progress
    let game_id_clone = game_id.clone();
    use_effect(move || {
        let game_id = game_id_clone.clone();
        spawn(async move {
            loop {
                let state = game_state.read();
                let download_state = state.get_download_state(&game_id);
                if download_state.active {
                    game_download.set(download_state.progress.clone());
                } else {
                    game_download.set(None);
                }
                drop(state);

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    });

    let has_game_download = game_download.read().is_some();
    let has_component_download = component_download_progress.read().is_some();
    let has_runtime_setup = game_state.read().is_runtime_setup_active();

    // Show component downloads only when NOT on the Runner page
    // (on Runner page, they're shown inline, so would be duplicate)
    let show_component_download = has_component_download && current_page != SettingsPage::Runner;

    // Show runtime setup downloads from main UI on all pages (including Runner)
    // since they're not shown inline anywhere in game settings
    let show_runtime_setup = has_runtime_setup;

    if !has_game_download && !show_component_download && !show_runtime_setup {
        return rsx! { rect {} };
    }

    rsx! {
        rect {
            width: "fill",
            padding: "16",
            background: "rgb(40, 40, 48, 0.8)",
            backdrop_blur: "20",
            corner_radius: "12",
            border: "1 solid rgb(100, 100, 110, 0.3)",
            direction: "vertical",
            spacing: "12",
            shadow: "0 4 16 0 rgb(0, 0, 0, 0.3)",

            label {
                font_size: "14",
                font_weight: "700",
                color: "rgb(240, 240, 250)",
                "Active Downloads"
            }

            // Display game download progress
            if let Some(progress) = game_download.read().as_ref() {
                GameDownloadProgress {
                    progress: progress.clone(),
                }
            }

            // Display component download progress (only when not on Runner page)
            if show_component_download {
                if let Some(component_progress) = component_download_progress.read().as_ref() {
                    ComponentDownloadProgressWidget {
                        progress: component_progress.clone(),
                    }
                }
            }

            // Display runtime setup active notification (only when not on Runner page)
            if show_runtime_setup {
                RuntimeSetupActiveWidget {}
            }
        }
    }
}

#[component]
fn GameDownloadProgress(progress: DownloadProgress) -> Element {
    #[allow(clippy::cast_precision_loss)]
    let pct = if progress.total > 0 {
        (progress.downloaded as f64 / progress.total as f64) * 100.0
    } else {
        0.0
    };

    let size_info = if progress.total > 0 {
        #[allow(clippy::cast_precision_loss)]
        let dl_gb = progress.downloaded as f64 / 1_000_000_000.0;
        #[allow(clippy::cast_precision_loss)]
        let total_gb = progress.total as f64 / 1_000_000_000.0;
        format!("{dl_gb:.2} GB / {total_gb:.2} GB")
    } else {
        String::new()
    };

    rsx! {
        rect {
            width: "fill",
            padding: "12",
            background: "rgb(35, 35, 45, 0.6)",
            corner_radius: "8",
            border: "1 solid rgb(80, 120, 160, 0.4)",
            direction: "vertical",
            spacing: "8",

            label {
                color: "rgb(220, 220, 230)",
                font_size: "13",
                font_weight: "600",
                "Game Download"
            }

            label {
                color: "rgb(180, 180, 200)",
                font_size: "12",
                "{progress.status}"
            }

            // Progress bar
            rect {
                width: "fill",
                height: "6",
                background: "rgb(40, 40, 50)",
                corner_radius: "3",
                overflow: "clip",

                rect {
                    width: "{pct:.0}%",
                    height: "6",
                    background: "rgb(50, 110, 170)",
                    corner_radius: "3",
                }
            }

            // Progress info
            rect {
                width: "fill",
                direction: "horizontal",
                main_align: "space-between",
                cross_align: "center",

                label {
                    color: "rgb(160, 160, 180)",
                    font_size: "11",
                    "{size_info}"
                }

                label {
                    color: "rgb(160, 160, 180)",
                    font_size: "11",
                    font_weight: "600",
                    "{pct:.1}%"
                }
            }

            if progress.speed_mb_s > 0.0 {
                label {
                    color: "rgb(160, 160, 180)",
                    font_size: "11",
                    "{progress.speed_mb_s:.1} MB/s"
                }
            }
        }
    }
}

#[component]
fn ComponentDownloadProgressWidget(progress: ComponentDownloadProgress) -> Element {
    #[allow(clippy::cast_precision_loss)]
    let pct = if progress.total > 0 {
        (progress.downloaded as f64 / progress.total as f64) * 100.0
    } else {
        0.0
    };

    let size_info = if progress.total > 0 {
        #[allow(clippy::cast_precision_loss)]
        let dl_mb = progress.downloaded as f64 / 1_000_000.0;
        #[allow(clippy::cast_precision_loss)]
        let total_mb = progress.total as f64 / 1_000_000.0;
        format!("{dl_mb:.1} MB / {total_mb:.1} MB")
    } else {
        String::new()
    };

    rsx! {
        rect {
            width: "fill",
            padding: "12",
            background: "rgb(35, 35, 45, 0.6)",
            corner_radius: "8",
            border: "1 solid rgb(80, 120, 160, 0.4)",
            direction: "vertical",
            spacing: "8",

            label {
                color: "rgb(220, 220, 230)",
                font_size: "13",
                font_weight: "600",
                "Downloading {progress.component_name}"
            }

            label {
                color: "rgb(180, 180, 200)",
                font_size: "12",
                "{progress.status}"
            }

            // Progress bar
            rect {
                width: "fill",
                height: "6",
                background: "rgb(40, 40, 50)",
                corner_radius: "3",
                overflow: "clip",

                rect {
                    width: "{pct:.0}%",
                    height: "6",
                    background: "{theme::ACCENT_SECONDARY}",
                    corner_radius: "3",
                }
            }

            // Progress info
            rect {
                width: "fill",
                direction: "horizontal",
                main_align: "space-between",
                cross_align: "center",

                label {
                    color: "rgb(160, 160, 180)",
                    font_size: "11",
                    "{size_info}"
                }

                label {
                    color: "rgb(160, 160, 180)",
                    font_size: "11",
                    font_weight: "600",
                    "{pct:.1}%"
                }
            }
        }
    }
}

#[component]
fn RuntimeSetupActiveWidget() -> Element {
    rsx! {
        rect {
            width: "fill",
            padding: "12",
            background: "rgb(35, 35, 45, 0.6)",
            corner_radius: "8",
            border: "1 solid rgb(80, 120, 160, 0.4)",
            direction: "vertical",
            spacing: "8",

            label {
                color: "rgb(220, 220, 230)",
                font_size: "13",
                font_weight: "600",
                "Runtime Setup Download"
            }

            label {
                color: "rgb(180, 180, 200)",
                font_size: "12",
                "A component download is currently in progress. Please wait for it to complete before starting another download."
            }
        }
    }
}
