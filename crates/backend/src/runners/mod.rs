mod proton;
mod wine;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::runners::{proton::Proton, wine::Wine};
use crate::settings::{GlobalSettings, InstalledGame};
use crate::components::{
    steamrt, 
    tweaks::TweakManifest, 
    ComponentManager, 
    ComponentType,
    installer::{ComponentRequirement, install_components},
};
use crate::progress::ProgressTracker;

pub trait Runner {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Runners {
    Native,
    Wine(Wine),
    Proton(Proton),
}

impl Runner for Runners {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<(), String> {
        match self {
            Runners::Native => Ok(()),
            Runners::Wine(wine) => wine.run_game(settings, game),
            Runners::Proton(proton) => proton.run_game(settings, game),
        }
    }
}

impl Runners {
    pub async fn ensure_runtime(
        &self,
        settings: &GlobalSettings,
        game: &InstalledGame,
        component_manager: &ComponentManager,
        tweak_manifest: &TweakManifest,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<HashMap<String, String>> {
        let mut requirements = Vec::new();
        
        if tweak_manifest.needs_jadeite(&game.id) {
            if !component_manager.is_installed(settings, ComponentType::Jadeite) {
                requirements.push(ComponentRequirement {
                    component_type: ComponentType::Jadeite,
                    display_name: "Jadeite".to_string(),
                });
            }
        }

        match self {
            Runners::Native => {}
            Runners::Wine(_) => {
                if !component_manager.is_installed(settings, ComponentType::Dxvk) {
                    requirements.push(ComponentRequirement {
                        component_type: ComponentType::Dxvk,
                        display_name: "DXVK".to_string(),
                    });
                }
            }
            Runners::Proton(_) => {
                if !component_manager.is_installed(settings, ComponentType::Umu) {
                    requirements.push(ComponentRequirement {
                        component_type: ComponentType::Umu,
                        display_name: "umu-launcher".to_string(),
                    });
                }
                
                let steamrt_setup = steamrt::prepare_steamrt(settings).await?;
                if steamrt_setup.needs_download {
                    requirements.push(ComponentRequirement {
                        component_type: ComponentType::SteamRuntime,
                        display_name: "Steam Runtime".to_string(),
                    });
                }
            }
        }
        
        install_components(
            settings,
            component_manager,
            requirements,
            progress_tracker,
            progress_key,
        ).await
    }

    pub async fn is_proton_runtime_installed(
        settings: &GlobalSettings,
        component_manager: &ComponentManager,
    ) -> bool {
        let umu_installed = component_manager.is_installed(settings, ComponentType::Umu);
        let steamrt_setup = steamrt::prepare_steamrt(settings).await.ok();
        let steamrt_installed = steamrt_setup.map_or(false, |s| !s.needs_download);
        
        umu_installed && steamrt_installed
    }

    pub async fn are_tweaks_installed(
        settings: &GlobalSettings,
        _game_id: &str,
        component_manager: &ComponentManager,
    ) -> bool {
        component_manager.is_installed(settings, ComponentType::Jadeite)
    }

    pub async fn download_proton_runtime(
        settings: &GlobalSettings,
        component_manager: &ComponentManager,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<()> {
        crate::components::install_proton_runtime(
            settings,
            component_manager,
            progress_tracker,
            progress_key,
        ).await
    }

    pub async fn download_tweaks(
        settings: &GlobalSettings,
        game_id: &str,
        component_manager: &ComponentManager,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<()> {
        crate::components::install_tweaks(
            settings,
            game_id,
            component_manager,
            progress_tracker,
            progress_key,
        ).await
    }
}
