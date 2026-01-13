use super::super::styles::*;
use super::super::types::*;
use freya::prelude::*;

/// Toggle option component for boolean settings
#[component]
pub fn ToggleOption(
    label_text: String,
    description: Option<String>,
    enabled: Signal<bool>,
    available: bool,
) -> Element {
    let background = if available {
        INTERACTIVE_BG
    } else {
        INTERACTIVE_BG_DISABLED
    };

    let text_color = if available {
        TEXT_PRIMARY
    } else {
        TEXT_DISABLED
    };

    let desc_color = if available {
        TEXT_SECONDARY
    } else {
        TEXT_DISABLED
    };

    let height = if description.is_some() { "64" } else { "48" };

    rsx! {
        rect {
            width: "fill",
            height: "{height}",
            padding: "12 16",
            background: "{background}",
            backdrop_blur: "12",
            corner_radius: "10",
            border: "1.5 solid {INTERACTIVE_BORDER}",
            direction: "horizontal",
            main_align: "space-between",
            cross_align: "center",

            rect {
                direction: "vertical",
                spacing: "2",

                label {
                    font_size: "15",
                    font_weight: "500",
                    color: "{text_color}",
                    "{label_text}"
                }

                if let Some(ref desc) = description {
                    label {
                        font_size: "12",
                        color: "{desc_color}",
                        "{desc}"
                    }
                }
            }

            Switch {
                enabled: if available { enabled.read().clone() } else { false },
                ontoggled: move |_| {
                    if available {
                        let current = enabled.read().clone();
                        enabled.set(!current);
                    }
                }
            }
        }
    }
}
