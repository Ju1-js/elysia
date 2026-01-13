mod animations;
mod components;
mod content;
mod handlers;
mod helpers;
pub mod state;
mod video_state;

use content::GameContent;
use freya::prelude::*;
use video_state::VideoState;

pub use crate::pages::game_settings_modal::GameSettingsModal;

pub use components::{BackgroundLayers, BottomRightButtons, CrossfadeState, TopRightButtons};
pub use state::{GlobalGameState, GlobalGameStateSignal};

use backend::runners::Runners;
use backend::settings::GlobalSettings;
use std::sync::{Arc, RwLock};

/// Game page component that displays selected game content
#[component]
pub fn Game() -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    let settings = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    let video_state = use_signal(|| VideoState::new());
    let game_state =
        use_signal(|| std::sync::Arc::new(std::sync::RwLock::new(GlobalGameState::new())));

    use_context_provider(|| video_state);
    use_context_provider(|| game_state);
    
    // Initialize game state based on default_preferences
    let mut game_state_init = game_state;
    let settings_init = settings.clone();
    use_effect(move || {
        spawn(async move {
            let settings_read = settings_init.read();
            let settings_guard = match settings_read.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            
            // Read the runner type from default_preferences
            let default_runner = &settings_guard.default_preferences.runner;
            let (runner_type, configured_wine_version, configured_proton_version) = match default_runner {
                Runners::Wine(wine) => (
                    state::RunnerType::Wine,
                    Some(wine.version.clone()),
                    None,
                ),
                Runners::Proton(proton) => (
                    state::RunnerType::Proton,
                    None,
                    Some(proton.version.clone()),
                ),
                Runners::Native => {
                    (state::RunnerType::Proton, None, None)
                }
            };
            
            // Get configured DXVK version
            let configured_dxvk_version = settings_guard
                .default_preferences
                .runtime_components
                .iter()
                .find_map(|component| {
                    if let backend::settings::RuntimeComponents::Dxvk(version) = component {
                        Some(version.clone())
                    } else {
                        None
                    }
                });
            
            let components_dir = settings_guard.components_directory.clone();
            drop(settings_guard);
            
            // Update runner type in game's GlobalGameState
            if let Ok(mut state) = game_state_init.write().write() {
                state.set_runner_type(runner_type.clone());
                
                // Check and update component readiness
                match runner_type {
                    state::RunnerType::Wine => {
                        let wine_ready = if configured_wine_version.as_deref() == Some("system") {
                            true
                        } else if let Some(ref wine_ver) = configured_wine_version {
                            components_dir.join("wine").join(wine_ver).exists()
                        } else {
                            false
                        };
                        
                        let dxvk_ready = if let Some(ref dxvk_ver) = configured_dxvk_version {
                            components_dir.join("dxvk").join(dxvk_ver).exists()
                        } else {
                            true
                        };
                        
                        state.set_wine_ready(wine_ready);
                        state.set_dxvk_ready(dxvk_ready);
                    }
                    state::RunnerType::Proton => {
                        let proton_ready = if let Some(ref proton_ver) = configured_proton_version {
                            components_dir.join("proton").join(proton_ver).exists()
                        } else {
                            false
                        };
                        
                        state.set_proton_ready(proton_ready);
                    }
                }
            }
        });
    });

    // Read the signal to establish reactivity - component will re-render when selected_game_id changes
    let current_game_id = selected_game_id.read();
    let _game_id_key = current_game_id
        .as_ref()
        .map(|s| s.clone())
        .unwrap_or_else(|| "none".to_string());

    rsx! {
        rect {
            key: "game-root-stable",
            width: "fill",
            height: "fill",
            GameContent {
                key: "{_game_id_key}",
                selected_game_id: selected_game_id,
            }
        }
    }
}
