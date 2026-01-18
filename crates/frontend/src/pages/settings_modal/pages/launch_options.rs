use crate::pages::settings_modal::components::{ToggleOption, check_command_exists};
use crate::pages::settings_modal::styles::TEXT_SECONDARY;
use freya::prelude::*;

/// Launch options settings page
#[component]
pub fn LaunchOptionsPage(
    launch_wrapper: Signal<String>,
    enable_winewayland: Signal<bool>,
    enable_mangohud: Signal<bool>,
    enable_gamemode: Signal<bool>,
) -> Element {
    let gamemode_available = check_command_exists("gamemoderun");

    rsx! {
        rect {
            width: "fill",
            direction: "vertical",
            spacing: "28",
            padding: "0 16 16 0",

            // Launch Command Wrapper
            rect {
                direction: "vertical",
                spacing: "12",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Launch Command Wrapper"
                }

                label {
                    font_size: "13",
                    color: "{TEXT_SECONDARY}",
                    "Add a command wrapper for launching (e.g., obs-gamecapture %command%)"
                }

                Input {
                    value: launch_wrapper.read().clone(),
                    onchange: move |e| launch_wrapper.set(e),
                    placeholder: "",
                    width: "fill",
                }
            }

            // Wine Options
            rect {
                direction: "vertical",
                spacing: "16",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Wine Options"
                }

                ToggleOption {
                    label_text: "Enable Wine-Wayland".to_string(),
                    description: "Enables Wine's native Wayland driver if supported",
                    enabled: enable_winewayland,
                    available: true,
                }
            }

            // Performance Tools
            rect {
                direction: "vertical",
                spacing: "16",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Performance Tools"
                }

                ToggleOption {
                    label_text: "Enable MangoHud".to_string(),
                    description: Some("Display performance overlay".to_string()),
                    enabled: enable_mangohud,
                    available: true,
                }

                ToggleOption {
                    label_text: "Enable GameMode".to_string(),
                    description: Some(if gamemode_available {
                        "Optimize system performance for gaming".to_string()
                    } else {
                        "GameMode not found in PATH".to_string()
                    }),
                    enabled: enable_gamemode,
                    available: gamemode_available,
                }
            }
        }
    }
}
