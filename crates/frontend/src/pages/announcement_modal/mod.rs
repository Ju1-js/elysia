use freya::prelude::*;

#[component]
pub fn AnnouncementModal(on_close: EventHandler<()>, scale: f64, message: String, can_close: bool, seconds_remaining: u32) -> Element {
    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            main_align: "center",
            cross_align: "center",
            onpointerpress: move |_| {
                if can_close {
                    on_close.call(());
                }
            },

            // Modal content
            rect {
                width: "500",
                max_height: "600",
                background: "rgb(25, 25, 30)",
                background_opacity: "0.8",
                backdrop_blur: "28",
                corner_radius: "16",
                border: "1 solid rgb(100, 100, 110, 0.3)",
                padding: "32",
                direction: "vertical",
                spacing: "24",
                shadow: "0 8 32 0 rgb(0, 0, 0, 0.5)",
                scale: "{scale}",
                onpointerpress: move |e| {
                    e.stop_propagation();
                },

                // Close button
                rect {
                    position: "absolute",
                    position_top: "16",
                    position_right: "16",
                    width: "32",
                    height: "32",
                    corner_radius: "8",
                    main_align: "center",
                    cross_align: "center",
                    background: if can_close { "rgb(50, 50, 55)" } else { "rgb(40, 40, 45, 0.5)" },
                    onpointerpress: move |_| {
                        if can_close {
                            on_close.call(());
                        }
                    },
                    if can_close {
                        svg {
                            width: "16",
                            height: "16",
                            svg_content: r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                            </svg>"#
                        }
                    } else {
                        label {
                            font_size: "12",
                            color: "rgb(150, 150, 160)",
                            "{seconds_remaining}"
                        }
                    }
                }

                // Header with icon and title
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "16",
                    cross_align: "center",

                    // Elysia icon
                    rect {
                        width: "64",
                        height: "64",
                        corner_radius: "12",
                        overflow: "clip",
                        image {
                            width: "64",
                            height: "64",
                            sampling: "trilinear",
                            image_data: static_bytes(include_bytes!("../../../../../assets/elysia.png"))
                        }
                    }

                    label {
                        font_size: "24",
                        font_weight: "700",
                        color: "white",
                        "Announcements"
                    }
                }

                rect {
                    width: "fill",
                    height: "1",
                    background: "rgb(100, 100, 110, 0.3)",
                }

                // Message content
                ScrollView {
                    width: "fill",
                    height: "300",
                    padding: "4",
                    label {
                        width: "fill",
                        font_size: "16",
                        color: "rgb(220, 220, 230)",
                        text_align: "center",
                        max_lines: "999",
                        "{message}"
                    }
                }
            }
        }
    }
}
