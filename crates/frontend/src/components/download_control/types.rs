use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed_mb_s: f64,
    pub status: String,
    pub is_busy: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SetupStep {
    CheckDependencies,
    DownloadWine,
    DownloadDxvk,
    DownloadUmu,
    DownloadSteamRuntime,
    DownloadProton,
    DownloadJadeite,
    #[allow(dead_code)]
    Complete,
}

impl SetupStep {
    pub fn description(&self) -> &'static str {
        match self {
            SetupStep::CheckDependencies => "Checking dependencies",
            SetupStep::DownloadWine => "Downloading Wine",
            SetupStep::DownloadDxvk => "Downloading DXVK",
            SetupStep::DownloadUmu => "Downloading UMU Launcher",
            SetupStep::DownloadSteamRuntime => "Downloading Steam Runtime",
            SetupStep::DownloadProton => "Downloading Proton",
            SetupStep::DownloadJadeite => "Downloading Jadeite",
            SetupStep::Complete => "Setup complete",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetupProgress {
    pub current_step: SetupStep,
    pub total_steps: usize,
    pub current_step_index: usize,
    pub step_progress: Option<DownloadProgress>,
}
