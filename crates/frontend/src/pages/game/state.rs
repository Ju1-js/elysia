use std::collections::HashMap;
use freya::prelude::*;
use crate::components::DownloadProgress;

#[derive(Clone, Debug)]
pub struct GameDownloadState {
    pub active: bool,
    pub progress: Option<DownloadProgress>,
    pub installed: bool,
}

impl Default for GameDownloadState {
    fn default() -> Self {
        Self {
            active: false,
            progress: None,
            installed: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SetupState {
    pub active: bool,
    pub ready: bool,
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            active: false,
            ready: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GlobalGameState {
    pub downloads: HashMap<String, GameDownloadState>,
    pub runtime_setup: SetupState,
    pub tweaks_setup: HashMap<String, SetupState>,
}

impl GlobalGameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_download_state(&self, game_id: &str) -> GameDownloadState {
        self.downloads.get(game_id).cloned().unwrap_or_default()
    }

    pub fn set_download_active(&mut self, game_id: &str, active: bool) {
        self.downloads.entry(game_id.to_string())
            .or_default()
            .active = active;
    }

    pub fn set_download_progress(&mut self, game_id: &str, progress: Option<DownloadProgress>) {
        self.downloads.entry(game_id.to_string())
            .or_default()
            .progress = progress;
    }

    pub fn set_download_installed(&mut self, game_id: &str, installed: bool) {
        self.downloads.entry(game_id.to_string())
            .or_default()
            .installed = installed;
    }

    pub fn set_runtime_active(&mut self, active: bool) {
        self.runtime_setup.active = active;
    }

    pub fn set_runtime_ready(&mut self, ready: bool) {
        self.runtime_setup.ready = ready;
    }

    pub fn get_tweaks_state(&self, game_id: &str) -> SetupState {
        self.tweaks_setup.get(game_id).cloned().unwrap_or_default()
    }

    pub fn set_tweaks_active(&mut self, game_id: &str, active: bool) {
        self.tweaks_setup.entry(game_id.to_string())
            .or_default()
            .active = active;
    }

    pub fn set_tweaks_ready(&mut self, game_id: &str, ready: bool) {
        self.tweaks_setup.entry(game_id.to_string())
            .or_default()
            .ready = ready;
    }
}

pub type GlobalGameStateSignal = Signal<GlobalGameState>;
