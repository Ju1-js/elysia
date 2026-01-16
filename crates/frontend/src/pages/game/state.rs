use crate::components::DownloadProgress;
use backend::runners::Runners;
use backend::settings::InstalledGame;
use freya::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct GameDownloadState {
    pub active: bool,
    pub progress: Option<DownloadProgress>,
    pub installed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum RunnerType {
    Wine,
    #[default]
    Proton,
}

#[derive(Clone, Debug, Default)]
pub struct ComponentSetupState {
    pub wine_ready: bool,
    pub dxvk_ready: bool,
    pub proton_ready: bool,
}

impl ComponentSetupState {
    pub fn is_ready_for(&self, runner_type: &RunnerType) -> bool {
        match runner_type {
            RunnerType::Wine => self.wine_ready && self.dxvk_ready,
            RunnerType::Proton => self.proton_ready,
        }
    }

    pub fn get_missing_components(&self, runner_type: &RunnerType) -> Vec<&'static str> {
        match runner_type {
            RunnerType::Wine => {
                let mut missing = Vec::new();
                if !self.wine_ready {
                    missing.push("Wine");
                }
                if !self.dxvk_ready {
                    missing.push("DXVK");
                }
                missing
            }
            RunnerType::Proton => {
                if self.proton_ready {
                    vec![]
                } else {
                    vec!["Proton"]
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SetupState {
    pub active: bool,
    pub ready: bool,
}

#[derive(Clone, Debug, Default)]
pub struct GlobalGameState {
    pub downloads: HashMap<String, GameDownloadState>,
    pub component_setup: ComponentSetupState,
    pub tweaks_setup: HashMap<String, SetupState>,
    pub current_runner_type: RunnerType,
    pub runtime_setup_active: bool,
    pub game_running: bool,
    pub game_child_pid: Option<u32>,
    pub runner_path: Option<String>,    // Path to wine/proton directory
    pub wine_prefix: Option<String>,    // WINEPREFIX path
    pub game_start_time: Option<std::time::Instant>,  // When the game started
}

impl GlobalGameState {
    pub fn new() -> Self {
        Self::default()
    }

    // Download state methods
    pub fn get_download_state(&self, game_id: &str) -> GameDownloadState {
        self.downloads.get(game_id).cloned().unwrap_or_default()
    }

    pub fn set_download_active(&mut self, game_id: &str, active: bool) {
        self.downloads
            .entry(game_id.to_string())
            .or_default()
            .active = active;
    }

    pub fn set_download_progress(&mut self, game_id: &str, progress: Option<DownloadProgress>) {
        self.downloads
            .entry(game_id.to_string())
            .or_default()
            .progress = progress;
    }

    pub fn set_download_installed(&mut self, game_id: &str, installed: bool) {
        self.downloads
            .entry(game_id.to_string())
            .or_default()
            .installed = installed;
    }

    // Component setup methods
    pub fn set_wine_ready(&mut self, ready: bool) {
        self.component_setup.wine_ready = ready;
    }

    pub fn set_dxvk_ready(&mut self, ready: bool) {
        self.component_setup.dxvk_ready = ready;
    }

    pub fn set_proton_ready(&mut self, ready: bool) {
        self.component_setup.proton_ready = ready;
    }

    pub fn is_runtime_ready(&self) -> bool {
        self.component_setup.is_ready_for(&self.current_runner_type)
    }

    pub fn get_missing_components(&self) -> Vec<&'static str> {
        self.component_setup
            .get_missing_components(&self.current_runner_type)
    }

    pub fn set_runner_type(&mut self, runner_type: RunnerType) {
        self.current_runner_type = runner_type;
    }

    pub fn set_runtime_active(&mut self, active: bool) {
        self.runtime_setup_active = active;
    }

    pub fn is_runtime_setup_active(&self) -> bool {
        self.runtime_setup_active
    }

    #[allow(dead_code)]
    pub fn set_runtime_ready(&mut self, ready: bool) {
        match self.current_runner_type {
            RunnerType::Wine => {
                self.component_setup.wine_ready = ready;
                self.component_setup.dxvk_ready = ready;
            }
            RunnerType::Proton => {
                self.component_setup.proton_ready = ready;
            }
        }
    }

    #[allow(dead_code)]
    pub fn can_use_dxvk(game: &InstalledGame) -> bool {
        matches!(game.runner, Runners::Wine(_))
    }

    pub fn get_tweaks_state(&self, game_id: &str) -> SetupState {
        self.tweaks_setup.get(game_id).cloned().unwrap_or_default()
    }

    pub fn set_tweaks_active(&mut self, game_id: &str, active: bool) {
        self.tweaks_setup
            .entry(game_id.to_string())
            .or_default()
            .active = active;
    }

    pub fn set_tweaks_ready(&mut self, game_id: &str, ready: bool) {
        self.tweaks_setup
            .entry(game_id.to_string())
            .or_default()
            .ready = ready;
    }

    // Game running state methods
    pub fn set_game_running(
        &mut self,
        running: bool,
        pid: Option<u32>,
        runner_path: Option<String>,
        wine_prefix: Option<String>,
    ) {
        self.game_running = running;
        self.game_child_pid = pid;
        self.runner_path = runner_path;
        self.wine_prefix = wine_prefix;
        
        if running {
            // Record start time when game starts
            self.game_start_time = Some(std::time::Instant::now());
        } else {
            // Clear start time when game stops
            self.game_start_time = None;
        }
    }

    pub fn is_game_running(&self) -> bool {
        self.game_running
    }

    pub fn get_elapsed_playtime(&self) -> u64 {
        if let Some(start_time) = self.game_start_time {
            start_time.elapsed().as_secs()
        } else {
            0
        }
    }

    #[allow(dead_code)]
    pub fn get_game_pid(&self) -> Option<u32> {
        self.game_child_pid
    }

    pub fn kill_game(&mut self) -> Result<(), String> {
        // Try wineserver -k if we have runner info
        if let (Some(runner_path), Some(wine_prefix)) = (&self.runner_path, &self.wine_prefix) {
            eprintln!("[GameState] Attempting to kill via wineserver");
            
            let result = match self.current_runner_type {
                RunnerType::Wine => {
                    backend::runners::Wine::kill_wine_process(runner_path, wine_prefix)
                }
                RunnerType::Proton => {
                    backend::runners::Proton::kill_proton_process(runner_path, wine_prefix)
                }
            };
            
            match result {
                Ok(()) => {
                    eprintln!("[GameState] Successfully killed via wineserver");
                    self.set_game_running(false, None, None, None);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[GameState] wineserver kill failed: {e}");
                }
            }
        }

        // Fallback to SIGKILL
        if let Some(pid) = self.game_child_pid {
            eprintln!("[GameState] Fallback: killing process PID {pid} with SIGKILL");
            let result = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .status();

            match result {
                Ok(status) => {
                    if status.success() {
                        eprintln!("[GameState] Successfully killed process PID {pid}");
                        self.set_game_running(false, None, None, None);
                        Ok(())
                    } else {
                        self.set_game_running(false, None, None, None);
                        Err("Kill command returned non-zero status".to_string())
                    }
                }
                Err(e) => {
                    eprintln!("[GameState] Failed to execute kill: {e}");
                    Err(format!("Failed to execute kill command: {e}"))
                }
            }
        } else {
            Err("No game process running".to_string())
        }
    }
}

pub type GlobalGameStateSignal = Signal<GlobalGameState>;
