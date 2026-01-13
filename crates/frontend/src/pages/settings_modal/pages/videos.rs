use crate::pages::settings_modal::components::ToggleOption;
use crate::pages::settings_modal::styles::*;
use freya::prelude::*;

/// Videos settings page
#[component]
pub fn VideosPage(disable_videos: Signal<bool>) -> Element {
    rsx! {
        rect {
            width: "fill",
            direction: "vertical",
            spacing: "28",
            padding: "0 16 16 0",

            // Video Background Settings
            rect {
                direction: "vertical",
                spacing: "16",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Video Backgrounds"
                }

                ToggleOption {
                    label_text: "Disable video backgrounds".to_string(),
                    description: Some("Use only static images instead of videos".to_string()),
                    enabled: disable_videos,
                    available: true,
                }
            }
        }
    }
}
