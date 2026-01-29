use freya::prelude::*;

const ELYSIA_VERSION: &str = "v0.2";

#[component]
pub fn ElysiaModal(on_close: EventHandler<()>, scale: f64) -> Element {
    let on_open_folder = move |_| {
        let data_dir = backend::globals::DATA_PATH.to_string_lossy().to_string();
        
        if let Err(e) = std::process::Command::new("xdg-open")
            .arg(&data_dir)
            .spawn()
        {
            eprintln!("Failed to open launcher folder: {e}");
        }
    };

    let on_open_repository = move |_| {
        let url = "https://dawn.wine/elysia/elysia";
        
        if let Err(e) = std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
        {
            eprintln!("Failed to open repository URL: {e}");
        }
    };

    let button_theme = theme_with!(ButtonTheme {
        background: "rgb(60, 60, 75, 0.9)".into(),
        hover_background: "rgb(70, 70, 85, 1)".into(),
        border_fill: "rgb(100, 100, 120, 0.5)".into(),
        focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
        padding: "12 20".into(),
        corner_radius: "10".into(),
        shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
        font_theme: theme_with!(FontTheme {
            color: "white".into(),
        }),
    });

    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            main_align: "center",
            cross_align: "center",
            onpointerpress: move |_| on_close.call(()),

            // Modal content
            rect {
                width: "400",
                background: "rgb(35, 35, 40)",
                background_opacity: "0.7",
                backdrop_blur: "20",
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
                    background: "rgb(50, 50, 55)",
                    onpointerpress: move |_| on_close.call(()),
                    svg {
                        width: "16",
                        height: "16",
                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                            <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                        </svg>"#
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
                        "Elysia"
                    }
                }

                rect {
                    width: "fill",
                    height: "1",
                    background: "rgb(100, 100, 110, 0.3)",
                }

                // Buttons
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "12",
                    main_align: "center",
                    cross_align: "center",

                    // Open Launcher Folder button
                    Button {
                        theme: button_theme.clone(),
                        onpress: on_open_folder,
                        label {
                            font_size: "15",
                            font_weight: "600",
                            "Open Launcher Folder"
                        }
                    }

                    // Open Repository button
                    Button {
                        theme: button_theme,
                        onpress: on_open_repository,
                        label {
                            font_size: "15",
                            font_weight: "600",
                            "Open Repository"
                        }
                    }
                }

                rect {
                    width: "fill",
                    height: "1",
                    background: "rgb(100, 100, 110, 0.3)",
                }

                // Version info
                rect {
                    width: "fill",
                    main_align: "center",
                    cross_align: "center",
                    label {
                        font_size: "13",
                        color: "rgb(150, 150, 160)",
                        "Elysia - {ELYSIA_VERSION}"
                    }
                }
            }
        }
    }
}
