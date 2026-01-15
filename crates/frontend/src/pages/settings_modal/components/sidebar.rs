use super::super::styles::{INTERACTIVE_BG_HOVER, INTERACTIVE_BORDER_SELECTED, TEXT_SELECTED, TEXT_HOVER, TEXT_SECONDARY};
use super::super::types::SettingsPage;
use freya::prelude::*;

/// Sidebar option for selecting settings pages
#[component]
pub fn SidebarOption(
    page: SettingsPage,
    current_page: SettingsPage,
    on_select: EventHandler<SettingsPage>,
) -> Element {
    let mut is_hovering = use_signal(|| false);
    let is_selected = page == current_page;

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

    let font_weight = if is_selected { "600" } else { "500" };

    rsx! {
        rect {
            width: "fill",
            height: "44",
            padding: "12 16",
            background: "{background}",
            corner_radius: "10",
            border: "1.5 solid {border_color}",
            main_align: "start",
            cross_align: "center",
            onclick: {
                to_owned![page];
                move |_| on_select.call(page.clone())
            },
            onmouseenter: move |_| is_hovering.set(true),
            onmouseleave: move |_| is_hovering.set(false),

            label {
                font_size: "15",
                font_weight: "{font_weight}",
                color: "{text_color}",
                "{page.display_name()}"
            }
        }
    }
}
