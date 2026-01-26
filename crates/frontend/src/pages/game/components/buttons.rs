use crate::components::MyButton;
use backend::meow::meow;
use freya::prelude::*;

#[component]
pub fn TopRightButtons() -> Element {
    rsx! {
        rect {
            position: "absolute",
            position_top: "32",
            position_left: "0",
            width: "100%",
            height: "0",
            direction: "horizontal",
            main_align: "end",
            padding: "0 32",

            MyButton {
                onpress: move |_| meow(),
                label {
                    font_size: "16",
                    font_weight: "500",
                    color: "white",
                    "Meow 🐾"
                }
            }
        }
    }
}

#[component]
pub fn BottomRightButtons(on_settings: EventHandler<()>, game_id: String) -> Element {
    let settings = use_context::<Signal<std::sync::Arc<std::sync::RwLock<backend::settings::GlobalSettings>>>>();
    let game_state = use_context::<crate::pages::game::state::GlobalGameStateSignal>();
    
    let (is_installed, total_playtime_seconds) = {
        let settings_guard = settings.read();
        if let Ok(s) = settings_guard.read() {
            let is_installed = s.installed_games.contains_key(&game_id);
            // Get playtime from game_preferences, or 0 if not set
            let playtime = s.game_preferences
                .get(&game_id)
                .map_or(0, |prefs| prefs.playtime_seconds);
            (is_installed, playtime)
        } else {
            (false, 0)
        }
    };
    
    // Add current session playtime if game is running
    let game_state_read = game_state.read();
    let current_session_playtime = if game_state_read.is_game_running() {
        game_state_read.get_elapsed_playtime()
    } else {
        0
    };
    
    let total_seconds = total_playtime_seconds + current_session_playtime;
    
    // Format playtime as "Xh Ym"
    let playtime_display = if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        if minutes == 0 {
            "0m".to_string()
        } else {
            format!("{minutes}m")
        }
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{hours}h")
        }
    };
    
    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            direction: "horizontal",
            main_align: "end",
            cross_align: "end",
            spacing: "12",
            padding: "32",

            // Only show playtime tracker if game is installed
            if is_installed {
                MyButton {
                    onpress: move |_| println!("Game tracker clicked!"),
                    rect {
                        direction: "horizontal",
                        cross_align: "center",
                        spacing: "8",
                        svg {
                            width: "20",
                            height: "20",
                            svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10 10-4.5 10-10S17.5 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm.5-13H11v6l5.2 3.2.8-1.3-4.5-2.7V7z"/></svg>"#
                        }
                        label {
                            font_size: "16",
                            font_weight: "500",
                            color: "white",
                            "{playtime_display}"
                        }
                    }
                }
            }

            MyButton {
                onpress: move |_| on_settings.call(()),
                rect {
                    direction: "horizontal",
                    cross_align: "center",
                    spacing: "8",
                    svg {
                        width: "20",
                        height: "20",
                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z"/></svg>"#
                    }
                    label {
                        font_size: "16",
                        font_weight: "500",
                        color: "white",
                        "Game Settings"
                    }
                }
            }
        }
    }
}
