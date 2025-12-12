use std::sync::{Arc, RwLock};
use freya::prelude::*;
use reqwest::Url;

use crate::components::{MyButton, MyNetworkImage};
use backend::settings::GlobalSettings;

#[component]
pub fn GameSettings(
    on_back: EventHandler<PressEvent>,
    background_url: Url,
    game_name: String,
) -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    let ctx = use_context::<crate::context::Context>();
    let _settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    
    let mut launch_wrapper = use_signal(|| String::new());
    let mut selected_runner = use_signal(|| "GE-Proton10-5".to_string());
    let mut show_runner_dropdown = use_signal(|| false);
    
    let game_id = selected_game_id.read();
    let Some(ref game_id_str) = *game_id else {
        return rsx! { 
            rect { 
                width: "fill", 
                height: "fill",
                main_align: "center",
                cross_align: "center",
                label { "No game selected" }
            } 
        };
    };

    let Some(_game) = ctx.api_games.iter().find(|g| &g.id == game_id_str).cloned() else {
        return rsx! { 
            rect { 
                width: "fill",
                height: "fill",
                main_align: "center",
                cross_align: "center",
                label { "Game not found" }
            }
        };
    };

    let runner_options = [
        "GE-Proton10-5",
        "Proton Experimental",
        "dwproton-10.0-9",
    ];

    rsx! {
        rect {
            width: "fill",
            height: "fill",
            
            // Blurred background image
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                main_align: "end",
                cross_align: "end",
                layer: "-1",
                shadow: "0 0 100 100 rgb(0, 0, 0)",
                MyNetworkImage {
                    url: background_url.clone(),
                    sampling: "trilinear",
                }
            }

            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                main_align: "start",
                cross_align: "start",
                layer: "-1",
                shadow: "0 0 100 100 rgb(0, 0, 0)",
                MyNetworkImage {
                    url: background_url,
                    sampling: "trilinear",
                }
            }
            
            // Dark overlay for better readability
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                background: "rgb(0, 0, 0)",
                opacity: "0.7",
                layer: "-1",
            }
            
            // Content
            rect {
                width: "fill",
                height: "fill",
                padding: "40 40 40 132",
                
                rect {
                    width: "fill",
                    max_width: "800",
                    direction: "vertical",
                    spacing: "32",
                    
                    // Header
                    rect {
                        direction: "vertical",
                        spacing: "8",
                        
                        label {
                            font_size: "32",
                            font_weight: "700",
                            color: "white",
                            "Game Settings"
                        }
                        
                        label {
                            font_size: "20",
                            font_weight: "400",
                            color: "rgb(160, 160, 170)",
                            "{game_name}"
                        }
                    }
                    
                    // Divider
                    rect {
                        width: "fill",
                        height: "1",
                        background: "rgb(60, 60, 70)",
                    }
                    
                    // Launch Wrapper Section
                    rect {
                        direction: "vertical",
                        spacing: "16",
                        
                        label {
                            font_size: "18",
                            font_weight: "600",
                            color: "white",
                            "Launch Command Wrapper"
                        }
                        
                        label {
                            font_size: "14",
                            color: "rgb(160, 160, 170)",
                            "Add environment variables or commands before %command%"
                        }
                        
                        rect {
                            width: "fill",
                            height: "48",
                            background: "rgb(30, 30, 35)",
                            corner_radius: "8",
                            padding: "12",
                            border: "1 solid rgb(60, 60, 70)",
                            
                            Input {
                                value: launch_wrapper.read().clone(),
                                onchange: move |e| launch_wrapper.set(e),
                                placeholder: "",
                                width: "fill",
                            }
                        }
                        
                        label {
                            font_size: "12",
                            color: "rgb(120, 120, 130)",
                            "Example: WINE_CANONICAL_HOLE=skip_volatile_check %command% or PROTON_LOG=1 mangohud %command%"
                        }
                    }
                    
                    // Proton Runner Section
                    rect {
                        direction: "vertical",
                        spacing: "16",
                        
                        label {
                            font_size: "18",
                            font_weight: "600",
                            color: "white",
                            "Proton Runner"
                        }
                        
                        label {
                            font_size: "14",
                            color: "rgb(160, 160, 170)",
                            "Select which Proton/Wine version to use"
                        }
                        
                        // Dropdown button
                        rect {
                            width: "fill",
                            height: "48",
                            background: "rgb(30, 30, 35)",
                            corner_radius: "8",
                            padding: "12 16",
                            border: "1 solid rgb(60, 60, 70)",
                            direction: "horizontal",
                            main_align: "space-between",
                            cross_align: "center",
                            onclick: move |_| show_runner_dropdown.set(!show_runner_dropdown()),
                            
                            label {
                                font_size: "14",
                                color: "white",
                                "{selected_runner.read()}"
                            }
                            
                            svg {
                                width: "16",
                                height: "16",
                                svg_content: if *show_runner_dropdown.read() {
                                    r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                        <path d="M7 14l5-5 5 5z"/>
                                    </svg>"#
                                } else {
                                    r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                        <path d="M7 10l5 5 5-5z"/>
                                    </svg>"#
                                }
                            }
                        }
                        
                        // Dropdown menu
                        if *show_runner_dropdown.read() {
                            rect {
                                width: "fill",
                                background: "rgb(30, 30, 35)",
                                corner_radius: "8",
                                border: "1 solid rgb(60, 60, 70)",
                                direction: "vertical",
                                
                                for runner in runner_options {
                                    rect {
                                        width: "fill",
                                        height: "44",
                                        padding: "12 16",
                                        background: if selected_runner.read().as_str() == runner {
                                            "rgb(50, 50, 60)"
                                        } else {
                                            "transparent"
                                        },
                                        onclick: move |_| {
                                            selected_runner.set(runner.to_string());
                                            show_runner_dropdown.set(false);
                                        },
                                        
                                        label {
                                            font_size: "14",
                                            color: "white",
                                            "{runner}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Divider
                    rect {
                        width: "fill",
                        height: "1",
                        background: "rgb(60, 60, 70)",
                    }
                    
                    // Action buttons
                    rect {
                        direction: "horizontal",
                        spacing: "12",
                        main_align: "end",
                        
                        MyButton {
                            onpress: on_back,
                            rect {
                                padding: "12 24",
                                label {
                                    font_size: "14",
                                    font_weight: "500",
                                    color: "white",
                                    "Cancel"
                                }
                            }
                        }
                        
                        MyButton {
                            onpress: move |event| {
                                println!("Saving wrapper: {}", launch_wrapper.read());
                                println!("Saving runner: {}", selected_runner.read());
                                on_back.call(event);
                            },
                            rect {
                                padding: "12 24",
                                background: "rgb(255, 149, 0)",
                                corner_radius: "8",
                                label {
                                    font_size: "14",
                                    font_weight: "500",
                                    color: "white",
                                    "Save Changes"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}