use freya::prelude::*;

#[allow(non_snake_case)]
/// Sidebar item component with hover effects
#[component]
pub fn MySidebarItem(
    /// Inner content for the SidebarItem.
    children: Element,
    /// Whether this sidebar item is currently active.
    #[props(default = false)]
    is_active: bool,
) -> Element {
    let font_theme = use_applied_theme!(None, sidebar_item).font_theme;
    let mut status = use_signal(ButtonStatus::default);
    let platform = use_platform();

    use_drop(move || {
        if *status.read() == ButtonStatus::Hovering {
            platform.set_cursor(CursorIcon::default());
        }
    });

    let onmouseenter = move |_| {
        platform.set_cursor(CursorIcon::Pointer);
        status.set(ButtonStatus::Hovering);
    };

    let onmouseleave = move |_| {
        platform.set_cursor(CursorIcon::default());
        status.set(ButtonStatus::default());
    };

    let background = "rgb(255,255,255,.02)";
    let hover_background = "rgb(255,255,255,.12)";
    let active_background = "rgb(255,255,255,.18)";

    let background = match *status.read() {
        _ if is_active => active_background,
        ButtonStatus::Hovering => hover_background,
        ButtonStatus::Idle => background,
    };

    rsx! {
        rect {
            overflow: "clip",
            margin: "2 0",
            onmouseenter,
            onmouseleave,
            width: "auto",
            height: "auto",
            color: "{font_theme.color}",
            corner_radius: "10",
            padding: "6",
            background: "{background}",
            shadow: "0 2 8 0 rgb(0, 0, 0, 40)",
            {children}
        }
    }
}
