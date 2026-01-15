use super::super::styles::{INTERACTIVE_BG_HOVER, INTERACTIVE_BG, INTERACTIVE_BORDER, TEXT_PRIMARY, TEXT_SECONDARY, INTERACTIVE_BORDER_SELECTED, TEXT_SELECTED, TEXT_HOVER};
use super::super::types::ComponentVersionInfo;
use freya::prelude::*;

#[component]
pub fn StylizedDropdown(
    options: Vec<ComponentVersionInfo>,
    selected: String,
    on_select: EventHandler<String>,
    width: String,
) -> Element {
    let mut is_open = use_signal(|| false);
    let mut hovering_button = use_signal(|| false);

    let background = if *hovering_button.read() {
        INTERACTIVE_BG_HOVER
    } else {
        INTERACTIVE_BG
    };

    let selected_display = options
        .iter()
        .find(|v| v.internal_name == selected)
        .map_or_else(|| {
            // If selected is empty and options exist, show first option's display name
            if selected.is_empty() && !options.is_empty() {
                options[0].display_name.clone()
            } else {
                selected.clone()
            }
        }, |v| v.display_name.clone());

    rsx! {
        rect {
            width: "{width}",
            direction: "vertical",
            spacing: "0",

            rect {
                width: "fill",
                height: "48",
                background: "{background}",
                backdrop_blur: "12",
                corner_radius: "10",
                border: "1.5 solid {INTERACTIVE_BORDER}",
                padding: "12 16",
                direction: "horizontal",
                main_align: "space-between",
                cross_align: "center",
                onclick: move |e| {
                    e.stop_propagation();
                    is_open.set(!is_open());
                },
                onmouseenter: move |_| hovering_button.set(true),
                onmouseleave: move |_| hovering_button.set(false),

                label {
                    font_size: "15",
                    font_weight: "500",
                    color: "{TEXT_PRIMARY}",
                    "{selected_display}"
                }

                label {
                    font_size: "14",
                    color: "{TEXT_SECONDARY}",
                    if *is_open.read() { "▲" } else { "▼" }
                }
            }

            if *is_open.read() {
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "4",
                    padding: "4",
                    margin: "4 0 0 0",
                    background: "rgb(35, 35, 45, 0.95)",
                    backdrop_blur: "16",
                    corner_radius: "10",
                    border: "1.5 solid {INTERACTIVE_BORDER}",
                    shadow: "0 4 16 0 rgb(0, 0, 0, 0.3)",
                    onclick: move |e| {
                        e.stop_propagation();
                    },

                    for version_info in options.clone() {
                        StylizedDropdownOption {
                            option: version_info.display_name.clone(),
                            internal_name: version_info.internal_name.clone(),
                            selected: selected.clone(),
                            on_select: on_select,
                            on_close: move |()| is_open.set(false),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StylizedDropdownOption(
    option: String,
    internal_name: String,
    selected: String,
    on_select: EventHandler<String>,
    on_close: EventHandler<()>,
) -> Element {
    let mut is_hovering = use_signal(|| false);

    let is_selected = internal_name == selected;
    let background = if is_selected {
        "rgb(50, 110, 170, 0.3)"
    } else if *is_hovering.read() {
        INTERACTIVE_BG_HOVER
    } else {
        "transparent"
    };

    let border_color = if is_selected {
        INTERACTIVE_BORDER_SELECTED
    } else {
        "transparent"
    };

    let text_color = if is_selected {
        TEXT_SELECTED
    } else if *is_hovering.read() {
        TEXT_HOVER
    } else {
        TEXT_SECONDARY
    };

    rsx! {
        rect {
            width: "fill",
            height: "42",
            padding: "10 12",
            background: "{background}",
            backdrop_blur: "12",
            corner_radius: "8",
            border: "1.5 solid {border_color}",
            onclick: {
                to_owned![internal_name];
                move |_| {
                    on_select.call(internal_name.clone());
                    on_close.call(());
                }
            },
            onmouseenter: move |_| is_hovering.set(true),
            onmouseleave: move |_| is_hovering.set(false),

            label {
                font_size: "15",
                color: "{text_color}",
                "{option}"
            }
        }
    }
}
