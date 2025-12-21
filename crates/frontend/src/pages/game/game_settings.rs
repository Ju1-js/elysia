use std::sync::{Arc, RwLock};
use freya::prelude::*;

use backend::settings::GlobalSettings;

#[component]
pub fn GameSettingsModal(
    on_close: EventHandler<()>,
    game_name: String,
    scale: f64,
) -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    let ctx = use_context::<crate::context::Context>();
    let _settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    
    let mut launch_wrapper = use_signal(|| String::new());
    let mut selected_runner = use_signal(|| "GE-Proton10-5".to_string());
    
    let game_id = selected_game_id.read();
    let Some(ref game_id_str) = *game_id else {
        return rsx! { rect {} };
    };

    let Some(_game) = ctx.api_games.iter().find(|g| &g.id == game_id_str).cloned() else {
        return rsx! { rect {} };
    };

    let runner_options = vec![
        "GE-Proton10-5".to_string(),
        "Proton Experimental".to_string(),
        "dwproton-10.0-9".to_string(),
    ];

    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "80",
            width: "calc(100% - 80)",
            height: "100%",
            background: "rgb(0, 0, 0, 0.3)",
            padding: "32",
            onclick: move |_| {
                on_close.call(());
            },
            
            rect {
                width: "fill",
                height: "fill",
                background: "rgb(40, 40, 48, 0.6)",
                backdrop_blur: "20",
                corner_radius: "16",
                border: "1 solid rgb(100, 100, 110, 0.2)",
                padding: "32",
                direction: "vertical",
                spacing: "24",
                onclick: move |e| e.stop_propagation(),
                rotate: "0deg",
                scale: "{scale}",
                
                rect {
                    width: "fill",
                    direction: "horizontal",
                    main_align: "space-between",
                    cross_align: "center",
                    
                    rect {
                        direction: "vertical",
                        spacing: "4",
                        
                        label {
                            font_size: "28",
                            font_weight: "700",
                            color: "white",
                            "Game Settings"
                        }
                        
                        label {
                            font_size: "16",
                            font_weight: "400",
                            color: "rgb(200, 200, 210)",
                            "{game_name}"
                        }
                    }
                    
                    rect {
                        width: "36",
                        height: "36",
                        corner_radius: "8",
                        background: "rgb(90, 90, 100, 0.6)",
                        border: "1 solid rgb(120, 120, 130, 0.3)",
                        main_align: "center",
                        cross_align: "center",
                        onclick: move |_| {
                            on_close.call(());
                        },
                        
                        svg {
                            width: "18",
                            height: "18",
                            svg_content: r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                <path d="M18 6L6 18M6 6l12 12" stroke="white" stroke-width="2" stroke-linecap="round"/>
                            </svg>"#
                        }
                    }
                }
                
                rect {
                    width: "fill",
                    height: "1",
                    background: "rgb(100, 100, 110, 0.3)",
                }
                
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "28",
                    
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
                            color: "rgb(200, 200, 210)",
                            "Add environment variables or commands before %command%"
                        }
                        
                        Input {
                            value: launch_wrapper.read().clone(),
                            onchange: move |e| launch_wrapper.set(e),
                            placeholder: "",
                            width: "fill",
                        }
                        
                        label {
                            font_size: "11",
                            color: "rgb(170, 170, 180)",
                            "Example: WINE_CANONICAL_HOLE=skip_volatile_check %command%"
                        }
                    }
                    
                    rect {
                        direction: "vertical",
                        spacing: "12",
                        
                        label {
                            font_size: "18",
                            font_weight: "600",
                            color: "white",
                            "Compatibility Tool"
                        }
                        
                        label {
                            font_size: "13",
                            color: "rgb(200, 200, 210)",
                            "Select which Proton/Wine version to use"
                        }
                        
                        Dropdown {
                            theme: theme_with!(DropdownTheme {
                                dropdown_background: "rgb(50, 50, 60, 0.85)".into(),
                                background_button: "rgb(50, 50, 60, 0.5)".into(),
                                hover_background: "rgb(60, 60, 70, 0.6)".into(),
                                border_fill: "rgb(100, 100, 110, 0.3)".into(),
                                focus_border_fill: "rgb(100, 100, 110, 0.5)".into(),
                                arrow_fill: "white".into(),
                                width: "fill".into(),
                                margin: "0".into(),
                                font_theme: theme_with!(FontTheme {
                                    color: "white".into(),
                                }),
                            }),
                            selected_item: rsx! {
                                rect {
                                    width: "fill",
                                    padding: "8",
                                    direction: "horizontal",
                                    main_align: "start",
                                    label { 
                                        font_size: "15",
                                        "{selected_runner.read()}" 
                                    }
                                }
                            },
                            for runner in runner_options {
                                DropdownItem {
                                    theme: theme_with!(DropdownItemTheme {
                                        background: "transparent".into(),
                                        select_background: "rgb(70, 70, 85, 0.5)".into(),
                                        hover_background: "rgb(60, 60, 70, 0.3)".into(),
                                        border_fill: "transparent".into(),
                                        select_border_fill: "transparent".into(),
                                        font_theme: theme_with!(FontTheme {
                                            color: "white".into(),
                                        }),
                                    }),
                                    onpress: {
                                        to_owned![runner];
                                        move |_| {
                                            selected_runner.set(runner.clone());
                                        }
                                    },
                                    rect {
                                        padding: "8",
                                        direction: "horizontal",
                                        main_align: "start",
                                        label { 
                                            font_size: "15",
                                            "{runner}" 
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
