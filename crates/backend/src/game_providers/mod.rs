pub mod common;
pub mod endfield;
pub mod game_info;
pub mod hoyoplay;
pub mod installer;

use anyhow::Result;
use async_trait::async_trait;

use crate::{game_providers::game_info::GameInfo, settings::GlobalSettings};
pub use common::Progress;

#[async_trait]
pub trait GameProvider: Send + Sync {
    //TODO: include latet version for each edition
    async fn fetch_games(&self, _settings: &GlobalSettings) -> Result<Vec<GameInfo>>;
}
