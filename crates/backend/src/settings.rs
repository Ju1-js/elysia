#![allow(dead_code)]
use crate::{
    globals::{CONFIG_PATH, DATA_PATH},
    runners::Runners,
};
use common::utils::filesystem::ensure_or_default;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalSettings {
    pub wineprefixes_directory: PathBuf,
    pub components_directory: PathBuf,
    pub temp_directory: PathBuf,
    pub cache_directory: PathBuf,
    pub games_directory: PathBuf,
    pub installed_games: HashMap<String, InstalledGame>,
    pub game_preferences: HashMap<String, GamePreferences>,
    pub default_preferences: GamePreferences,
    pub disable_videos: bool,
    pub last_announcement_hash: Option<String>,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        let data_path = &*DATA_PATH;
        let wineprefixes_directory = data_path.join("wineprefixes/");
        let components_directory = data_path.join("components/");
        let temp_directory = data_path.join("temp/");
        let cache_directory = data_path.join("cache/");
        let games_directory = data_path.join("games/");
        Self {
            wineprefixes_directory,
            components_directory,
            temp_directory,
            cache_directory,
            games_directory,
            installed_games: HashMap::new(),
            game_preferences: HashMap::new(),
            default_preferences: GamePreferences::default(),
            disable_videos: false,
            last_announcement_hash: None,
        }
    }
}

impl GlobalSettings {
    /// # Errors
    /// Returns an error if the config cannot be loaded.
    pub fn load() -> Result<GlobalSettings, String> {
        let config_path = &*CONFIG_PATH;
        let exists =
            fs::exists(config_path).map_err(|e| format!("Cannot check if config exists: {e}"))?;
        if !exists {
            return Err("Config does not exist".to_string());
        }
        let data = fs::read(config_path).map_err(|e| format!("Cannot read config file: {e}"))?;
        let mut settings = serde_json::from_slice::<GlobalSettings>(&data)
            .map_err(|e| format!("Cannot deserialize saved config: {e}"))?;
        settings.validate();
        Ok(settings)
    }

    /// # Errors
    /// Returns an error if the config cannot be saved.
    pub fn save(&self) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {e}"))?;
        fs::write(&*CONFIG_PATH, data)
            .map_err(|e| format!("Failed to write settings file: {e}"))?;
        Ok(())
    }

    /// # Panics
    /// Panics if the paths cannot be validated or created.
    pub fn validate(&mut self) {
        let mut check_fn = || -> Result<(), String> {
            self.wineprefixes_directory = ensure_or_default(
                &self.wineprefixes_directory,
                &DATA_PATH.join("wineprefixes"),
            )?
            .to_path_buf();
            self.components_directory =
                ensure_or_default(&self.components_directory, &DATA_PATH.join("components"))?
                    .to_path_buf();
            self.temp_directory =
                ensure_or_default(&self.temp_directory, &DATA_PATH.join("temp"))?.to_path_buf();
            self.cache_directory =
                ensure_or_default(&self.cache_directory, &DATA_PATH.join("cache"))?.to_path_buf();
            self.games_directory =
                ensure_or_default(&self.games_directory, &DATA_PATH.join("games"))?.to_path_buf();
            Ok(())
        };
        if let Err(e) = check_fn() {
            panic!("Cannot use current or default path: {e}");
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct InstalledGame {
    pub id: String,
    pub biz_name: String,
    pub install_path: PathBuf,
    pub executable_path: PathBuf,
    pub command_wrapper: Option<String>,
    pub command_arguments: Option<Vec<String>>,
    pub environment: HashMap<String, String>,
    pub runner: Runners,
    pub runtime_components: Vec<RuntimeComponents>,
    pub enable_winewayland: bool,
    pub enable_mangohud: bool,
    pub enable_gamemode: bool,
    #[serde(default)]
    pub use_directx11: bool,
}

impl Default for InstalledGame {
    fn default() -> Self {
        Self {
            id: String::new(),
            biz_name: String::new(),
            install_path: PathBuf::new(),
            executable_path: PathBuf::new(),
            command_wrapper: None,
            command_arguments: None,
            environment: HashMap::new(),
            runner: Runners::Proton(crate::runners::Proton {
                version: String::new(),
            }),
            runtime_components: Vec::new(),
            enable_winewayland: false,
            enable_mangohud: false,
            enable_gamemode: false,
            use_directx11: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeComponents {
    Dxvk(ComponentVersion),
    Vkd3dProton(ComponentVersion),
    DxvkNvApi(ComponentVersion),
}

type ComponentVersion = String;

/// Game preferences stored even before a game is installed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct GamePreferences {
    pub runner: Runners,
    pub runtime_components: Vec<RuntimeComponents>,
    pub command_wrapper: Option<String>,
    pub enable_winewayland: bool,
    pub enable_mangohud: bool,
    pub enable_gamemode: bool,
    #[serde(default)]
    pub playtime_seconds: u64,
    #[serde(default)]
    pub use_directx11: bool,
}

impl Default for GamePreferences {
    fn default() -> Self {
        Self {
            runner: Runners::Proton(crate::runners::Proton {
                version: String::new(), // Empty version will be resolved to first available Proton (typically the latest)
            }),
            runtime_components: Vec::new(),
            command_wrapper: None,
            enable_winewayland: false,
            enable_mangohud: false,
            enable_gamemode: false,
            playtime_seconds: 0,
            use_directx11: false,
        }
    }
}
