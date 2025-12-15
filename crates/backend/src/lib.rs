pub mod components;
pub mod game_manager;
pub mod game_providers;
pub mod globals;
pub mod runners;
pub mod settings;
pub mod task_manager;

use crate::game_providers::{GameProvider, endfield::EndfieldProvider, hoyoplay::HoyoplayProvider};

pub(crate) const GAME_PROVIDERS: &[&dyn GameProvider] =
    &[&EndfieldProvider {}, &HoyoplayProvider {}];
