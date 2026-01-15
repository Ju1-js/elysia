use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::game_providers::Progress;
use serde::{Deserialize, Serialize};

use crate::settings::{GlobalSettings, InstalledGame};
use async_trait::async_trait;

#[async_trait]
pub trait GameInstaller: Send + Sync {
    fn progress_key(&self) -> String;
    fn clear_progress(&self);
    fn get_progress(&self, key: &str) -> Option<Progress>;
    fn get_install_path(&self) -> PathBuf;
    async fn install(&self) -> Result<InstalledGame, String>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstallationManifest {
    pub game_id: String,
}

pub struct InstallerManager;

impl InstallerManager {
    #[must_use] 
    pub fn create_installer(
        game_id: &str,
        biz: &str,
        temp_dir: PathBuf,
        components_dir: &Path,
    ) -> Option<Box<dyn GameInstaller>> {
        let games_dir = components_dir
            .parent()
            .unwrap_or(components_dir)
            .join("games");

        match biz {
            "endfield" => {
                use crate::game_providers::endfield::EndfieldInstaller;

                Some(Box::new(EndfieldInstaller::new(
                    game_id.to_string(),
                    temp_dir,
                    games_dir,
                    biz.to_string(),
                )))
            }
            _ => None,
        }
    }

    pub fn spawn_install(
        settings: Arc<RwLock<GlobalSettings>>,
        installer: Box<dyn GameInstaller>,
        game_id: String,
    ) {
        tokio::spawn(async move {
            match installer.install().await {
                Ok(installed_game) => {
                    let settings = settings.write();
                    if let Ok(mut settings) = settings
                        && let Err(e) =
                            Self::persist_installation(&mut settings, game_id, installed_game)
                    {
                        eprintln!("Failed to persist installation: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to install game: {e}");
                }
            }
        });
    }

    #[must_use] 
    pub fn is_game_installed(
        settings: &GlobalSettings,
        game_id: &str,
        biz: &str,
        temp_dir: PathBuf,
        components_dir: &Path,
    ) -> bool {
        if !settings.installed_games.contains_key(game_id) {
            return false;
        }

        if let Some(installer) = Self::create_installer(game_id, biz, temp_dir, components_dir) {
            let install_path = installer.get_install_path();
            Self::check_marker_file(&install_path, game_id)
        } else {
            false
        }
    }

    fn check_marker_file(install_dir: &Path, game_id: &str) -> bool {
        let marker_path = install_dir.join(".elysia_installed");

        if !marker_path.exists() {
            return false;
        }

        if let Ok(data) = std::fs::read_to_string(&marker_path)
            && let Ok(manifest) = serde_json::from_str::<InstallationManifest>(&data)
        {
            return manifest.game_id == game_id;
        }

        false
    }

    /// # Errors
    /// Returns an error if the installation cannot be persisted.
    pub fn persist_installation(
        settings: &mut GlobalSettings,
        game_id: String,
        installed_game: InstalledGame,
    ) -> Result<(), String> {
        settings.installed_games.insert(game_id, installed_game);

        settings
            .save()
            .map_err(|e| format!("Failed to save settings: {e}"))?;

        Ok(())
    }
}
