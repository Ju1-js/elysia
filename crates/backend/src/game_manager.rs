use anyhow::{Result, anyhow};
use futures_util::future::join_all;

use crate::{GAME_PROVIDERS, game_providers::game_info::GameInfo, settings::GlobalSettings};

#[derive(Debug, Default)]
pub struct GameManager {
    pub games: Vec<GameInfo>,
}

impl GameManager {
    pub fn new() -> Self {
        Self { games: Vec::new() }
    }

    pub async fn init(&mut self, settings: &GlobalSettings) -> Result<()> {
        let handles: Vec<_> = GAME_PROVIDERS
            .iter()
            .map(|provider| {
                let settings = settings.clone();
                tokio::spawn(async move { provider.fetch_games(&settings).await })
            })
            .collect();

        let results = join_all(handles).await;

        for result in results {
            match result {
                Ok(games) => {
                    self.games.extend(games?);
                }
                Err(err) => {
                    eprintln!("Failed to initialize game provider: {}", err);
                }
            }
        }

        if self.games.is_empty() {
            return Err(anyhow!("Failed to fetch games from all providers"));
        }

        Ok(())
    }
}
