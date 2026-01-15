use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TweakType {
    Jadeite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SetupCommandType {
    Winetricks(Vec<String>),
    CustomScript(String),
    RegistryEdit(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupCommand {
    pub command_type: SetupCommandType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfiguration {
    pub tweaks: Vec<TweakType>,
    pub setup_commands: Vec<SetupCommand>,
}

pub struct TweakManifest {
    game_configs: HashMap<String, GameConfiguration>,
}

impl TweakManifest {
    #[must_use] 
    pub fn new() -> Self {
        let mut game_configs = HashMap::new();

        game_configs.insert(
            "zePXHT2t4L2tKR4m".to_string(),
            GameConfiguration {
                tweaks: vec![TweakType::Jadeite],
                setup_commands: vec![],
            },
        );

        Self { game_configs }
    }

    #[must_use] 
    pub fn get_config(&self, game_id: &str) -> Option<&GameConfiguration> {
        self.game_configs.get(game_id)
    }

    #[must_use] 
    pub fn get_tweaks(&self, game_id: &str) -> Vec<TweakType> {
        self.game_configs
            .get(game_id)
            .map(|config| config.tweaks.clone())
            .unwrap_or_default()
    }

    #[must_use] 
    pub fn get_setup_commands(&self, game_id: &str) -> Vec<SetupCommand> {
        self.game_configs
            .get(game_id)
            .map(|config| config.setup_commands.clone())
            .unwrap_or_default()
    }

    #[must_use] 
    pub fn needs_tweak(&self, game_id: &str, tweak_type: &TweakType) -> bool {
        self.game_configs
            .get(game_id)
            .is_some_and(|config| config.tweaks.contains(tweak_type))
    }

    #[must_use] 
    pub fn needs_jadeite(&self, game_id: &str) -> bool {
        self.needs_tweak(game_id, &TweakType::Jadeite)
    }

    #[must_use] 
    pub fn needs_first_time_setup(&self, game_id: &str) -> bool {
        !self.get_setup_commands(game_id).is_empty()
    }
}

impl Default for TweakManifest {
    fn default() -> Self {
        Self::new()
    }
}
