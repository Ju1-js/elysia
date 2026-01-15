use crate::theme;
use freya::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentDownloadProgress {
    pub component_name: String,
    pub downloaded: u64,
    pub total: u64,
    pub status: String,
    pub is_active: bool,
}

/// Widget displaying component download progress
#[component]
pub fn ComponentDownloadWidget(progress: Signal<Option<ComponentDownloadProgress>>) -> Element {
    let prog = progress.read();

    if prog.is_none() {
        return rsx! { rect {} };
    }

    let p = prog.as_ref().unwrap();

    #[allow(clippy::cast_precision_loss)]
    let pct = if p.total > 0 {
        (p.downloaded as f64 / p.total as f64) * 100.0
    } else {
        0.0
    };

    let size_info = if p.total > 0 {
        #[allow(clippy::cast_precision_loss)]
        let dl_mb = p.downloaded as f64 / 1_000_000.0;
        #[allow(clippy::cast_precision_loss)]
        let total_mb = p.total as f64 / 1_000_000.0;
        format!("{dl_mb:.1} MB / {total_mb:.1} MB")
    } else {
        String::new()
    };

    rsx! {
        rect {
            position: "absolute",
            position_top: "20",
            position_right: "20",
            width: "320",
            padding: "16",
            corner_radius: "12",
            background: "rgb(35, 35, 40)",
            background_opacity: "0.95",
            border: "1 solid rgb(255, 255, 255, 0.15)",
            direction: "vertical",
            spacing: "12",
            shadow: "0 8 24 0 rgb(0, 0, 0, 0.4)",
            backdrop_blur: "20",

            // Header
            rect {
                width: "fill",
                direction: "horizontal",
                main_align: "space-between",
                cross_align: "center",

                label {
                    color: "rgb(240, 240, 250)",
                    font_size: "14",
                    font_weight: "700",
                    "Downloading {p.component_name}"
                }
            }

            // Status
            label {
                color: "rgb(200, 200, 210)",
                font_size: "13",
                "{p.status}"
            }

            // Progress bar
            rect {
                width: "fill",
                height: "6",
                background: "rgb(40, 40, 45)",
                corner_radius: "3",
                overflow: "clip",

                rect {
                    width: "{pct:.0}%",
                    height: "6",
                    background: "{theme::ACCENT_SECONDARY}",
                    corner_radius: "3",
                }
            }

            // Progress text
            rect {
                width: "fill",
                direction: "horizontal",
                main_align: "space-between",
                cross_align: "center",

                label {
                    color: "rgb(200, 200, 210)",
                    font_size: "12",
                    "{size_info}"
                }

                label {
                    color: "rgb(200, 200, 210)",
                    font_size: "12",
                    font_weight: "600",
                    "{pct:.1}%"
                }
            }
        }
    }
}
