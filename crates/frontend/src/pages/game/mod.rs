mod components;
mod video_state;
mod game_settings;
mod handlers;
mod content;
mod helpers;

use freya::prelude::*;

use content::GameContent;
use video_state::VideoState;

pub use components::{BackgroundLayers, TopRightButtons, BottomRightButtons, CrossfadeState};
pub use game_settings::GameSettingsModal;

#[component]
pub fn Game() -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    
    let video_state = use_signal(|| VideoState::new());
    use_context_provider(|| video_state);
    
    rsx! {
        rect {
            key: "game-root-stable",
            width: "fill",
            height: "fill",
            GameContent {
                selected_game_id: selected_game_id,
            }
        }
    }
}
