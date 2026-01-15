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
    fn get_executable_name(&self) -> &str;
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
        games_dir: PathBuf,
    ) -> Option<Box<dyn GameInstaller>> {
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
        games_dir: PathBuf,
    ) -> bool {
        if !settings.installed_games.contains_key(game_id) {
            return false;
        }

        if let Some(installer) = Self::create_installer(game_id, biz, temp_dir, games_dir) {
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

    /// Import an existing game installation from a specified directory
    /// 
    /// This function:
    /// 1. Verifies the game executable exists in the specified path
    /// 2. Creates a marker file (`.elysia_installed`) to track the installation
    /// 3. Persists the installation to settings
    /// 
    /// # Errors
    /// Returns an error if:
    /// - The installer is not available for this game
    /// - The executable doesn't exist in the specified path
    /// - The marker file cannot be created
    /// - The installation cannot be persisted
    pub fn import_game(
        settings: &mut GlobalSettings,
        game_id: &str,
        biz: &str,
        import_path: PathBuf,
        temp_dir: PathBuf,
        games_dir: PathBuf,
    ) -> Result<InstalledGame, String> {
        // Create installer to get expected executable name
        let installer = Self::create_installer(game_id, biz, temp_dir, games_dir)
            .ok_or_else(|| format!("No installer available for game: {game_id} (biz: {biz})"))?;

        let executable_name = installer.get_executable_name();
        let executable_path = import_path.join(executable_name);

        // Verify executable exists
        if !executable_path.exists() {
            return Err(format!(
                "Game executable '{}' not found in '{}'",
                executable_name,
                import_path.display()
            ));
        }

        // Create marker file
        let marker_path = import_path.join(".elysia_installed");
        let manifest = InstallationManifest {
            game_id: game_id.to_string(),
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
        std::fs::write(&marker_path, manifest_json)
            .map_err(|e| format!("Failed to write marker file: {e}"))?;

        // Create InstalledGame struct
        let installed_game = InstalledGame {
            id: game_id.to_string(),
            biz_name: biz.to_string(),
            executable_path,
            install_path: import_path,
            ..Default::default()
        };

        // Persist to settings
        Self::persist_installation(settings, game_id.to_string(), installed_game.clone())?;

        Ok(installed_game)
    }

    /// Import a game by scanning for executables in the directory
    /// Scans for the expected executable name from the installer
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The expected executable is not found
    /// - MD5 verification fails
    /// - File system operations fail
    #[allow(clippy::needless_pass_by_value)]
    pub fn import_game_by_scan(
        settings: &mut GlobalSettings,
        game_id: &str,
        biz: &str,
        import_path: PathBuf,
        temp_dir: PathBuf,
        games_dir: PathBuf,
    ) -> Result<InstalledGame, String> {
        use crate::game_providers::scanner;
        use std::collections::HashSet;

        // Get the expected executable name from the installer
        let installer = Self::create_installer(game_id, biz, temp_dir.clone(), games_dir.clone());
        let expected_name = if let Some(ref installer) = installer {
            installer.get_executable_name()
        } else {
            return Err(format!("No installer available for game: {game_id} (biz: {biz})"));
        };

        // Create a filter with just the expected executable name
        let mut exe_filter = HashSet::new();
        exe_filter.insert(expected_name.to_string());

        // Scan for the specific executable in the directory (max depth 1)
        let found_exes = scanner::scan_for_executables(&import_path, 1, Some(&exe_filter));

        if found_exes.is_empty() {
            return Err(format!(
                "Game executable '{}' not found in '{}'",
                expected_name,
                import_path.display()
            ));
        }

        let executable_path = found_exes[0].clone();

        // Create marker file
        let marker_path = import_path.join(".elysia_installed");
        let manifest = InstallationManifest {
            game_id: game_id.to_string(),
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
        std::fs::write(&marker_path, manifest_json)
            .map_err(|e| format!("Failed to write marker file: {e}"))?;

        // Create InstalledGame struct
        let installed_game = InstalledGame {
            id: game_id.to_string(),
            biz_name: biz.to_string(),
            executable_path,
            install_path: import_path,
            ..Default::default()
        };

        // Persist to settings
        Self::persist_installation(settings, game_id.to_string(), installed_game.clone())?;

        Ok(installed_game)
    }
}
