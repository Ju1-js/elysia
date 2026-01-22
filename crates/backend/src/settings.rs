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
    pub audio_directory: PathBuf,
    pub installed_games: HashMap<String, InstalledGame>,
    pub game_preferences: HashMap<String, GamePreferences>,
    pub default_preferences: DefaultGamePreferences,
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
        let audio_directory = data_path.join("audio/");
        Self {
            wineprefixes_directory,
            components_directory,
            temp_directory,
            cache_directory,
            games_directory,
            audio_directory,
            installed_games: HashMap::new(),
            game_preferences: HashMap::new(),
            default_preferences: DefaultGamePreferences::default(),
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
            self.audio_directory = 
                ensure_or_default(&self.audio_directory, &DATA_PATH.join("audio"))?.to_path_buf();
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeComponents {
    Dxvk(ComponentVersion),
    Vkd3dProton(ComponentVersion),
    DxvkNvApi(ComponentVersion),
}

type ComponentVersion = String;

/// Default game preferences - all fields are concrete values used as the base
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct DefaultGamePreferences {
    pub runner: Runners,
    pub runtime_components: Vec<RuntimeComponents>,
    pub command_wrapper: Option<String>,
    pub enable_winewayland: bool,
    pub enable_mangohud: bool,
    pub enable_gamemode: bool,
    #[serde(default)]
    pub use_directx11: bool,
}

impl Default for DefaultGamePreferences {
    fn default() -> Self {
        Self {
            runner: Runners::Proton(crate::runners::Proton {
                version: String::new(),
            }),
            runtime_components: Vec::new(),
            command_wrapper: None,
            enable_winewayland: false,
            enable_mangohud: false,
            enable_gamemode: false,
            use_directx11: false,
        }
    }
}

/// Per-game preferences
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct GamePreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<Runners>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_components: Option<Vec<RuntimeComponents>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_wrapper: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_winewayland: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_mangohud: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_gamemode: Option<bool>,
    /// Playtime tracking (never inherited, always stored)
    #[serde(default)]
    pub playtime_seconds: u64,
    /// fixme: this should be in tweaks, its not for every game
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_directx11: Option<bool>,
}

impl GamePreferences {
    /// Merge with defaults to get resolved values for all fields
    #[must_use]
    pub fn merge_with_defaults(&self, defaults: &DefaultGamePreferences) -> ResolvedGamePreferences {
        ResolvedGamePreferences {
            runner: self.runner.clone().unwrap_or_else(|| defaults.runner.clone()),
            runtime_components: self.runtime_components.clone().unwrap_or_else(|| defaults.runtime_components.clone()),
            command_wrapper: self.command_wrapper.clone().unwrap_or(defaults.command_wrapper.clone()),
            enable_winewayland: self.enable_winewayland.unwrap_or(defaults.enable_winewayland),
            enable_mangohud: self.enable_mangohud.unwrap_or(defaults.enable_mangohud),
            enable_gamemode: self.enable_gamemode.unwrap_or(defaults.enable_gamemode),
            playtime_seconds: self.playtime_seconds,
            use_directx11: self.use_directx11.unwrap_or(defaults.use_directx11),
        }
    }
    
    /// Create `GamePreferences` from old format for backward compatibility
    pub(crate) fn from_legacy(legacy: LegacyGamePreferences) -> Self {
        Self {
            runner: Some(legacy.runner),
            runtime_components: Some(legacy.runtime_components),
            command_wrapper: Some(legacy.command_wrapper),
            enable_winewayland: Some(legacy.enable_winewayland),
            enable_mangohud: Some(legacy.enable_mangohud),
            enable_gamemode: Some(legacy.enable_gamemode),
            playtime_seconds: legacy.playtime_seconds,
            use_directx11: Some(legacy.use_directx11),
        }
    }
}

/// Resolved preferences with all fields having concrete values
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ResolvedGamePreferences {
    pub runner: Runners,
    pub runtime_components: Vec<RuntimeComponents>,
    pub command_wrapper: Option<String>,
    pub enable_winewayland: bool,
    pub enable_mangohud: bool,
    pub enable_gamemode: bool,
    pub playtime_seconds: u64,
    pub use_directx11: bool,
}

/// Old format for backward compatibility during migration
#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct LegacyGamePreferences {
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
