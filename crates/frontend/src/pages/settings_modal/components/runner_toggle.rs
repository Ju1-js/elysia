use super::super::styles::{INTERACTIVE_BG_SELECTED, INTERACTIVE_BG_HOVER, TEXT_SELECTED, TEXT_HOVER, TEXT_SECONDARY};
use super::super::types::RunnerType;
use freya::prelude::*;

/// Toggle button for selecting runner type (Wine/Proton)
#[component]
pub fn RunnerToggleButton(
    runner_type: RunnerType,
    selected: bool,
    on_select: EventHandler<RunnerType>,
) -> Element {
    let mut is_hovering = use_signal(|| false);

    let background = if selected {
        INTERACTIVE_BG_SELECTED
    } else if *is_hovering.read() {
        INTERACTIVE_BG_HOVER
    } else {
        "transparent"
    };

    let border = if selected {
        "1.5 solid rgb(70, 135, 200, 0.6)"
    } else {
        "0 solid transparent"
    };

    let shadow = if selected {
        "0 2 8 0 rgb(50, 110, 170, 0.3)"
    } else {
        "0 0 0 0 transparent"
    };

    let text_color = if selected {
        TEXT_SELECTED
    } else if *is_hovering.read() {
        TEXT_HOVER
    } else {
        TEXT_SECONDARY
    };

    rsx! {
        rect {
            width: "50%",
            height: "fill",
            background: "{background}",
            backdrop_blur: "12",
            corner_radius: "8",
            main_align: "center",
            cross_align: "center",
            border: "{border}",
            shadow: "{shadow}",
            onpointerpress: {
                to_owned![runner_type];
                move |_| on_select.call(runner_type.clone())
            },
            onmouseenter: move |_| is_hovering.set(true),
            onmouseleave: move |_| is_hovering.set(false),

            label {
                font_size: "14",
                font_weight: "600",
                color: "{text_color}",
                "{runner_type.display_name()}"
            }
        }
    }
}
