use crate::pages::settings_modal::{SettingsModal, SettingsContext};
use freya::prelude::*;

#[component]
pub fn GameSettingsModal(on_close: EventHandler<()>, game_name: String, scale: f64) -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    
    // Get the game_id
    let game_id = match &*selected_game_id.read() {
        Some(id) => id.clone(),
        None => return rsx! { rect {} },
    };

    rsx! {
        SettingsModal {
            on_close: on_close,
            context: SettingsContext::Game {
                game_id,
                game_name,
            },
            scale: scale,
        }
    }
}
