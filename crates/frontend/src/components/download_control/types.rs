use freya::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed_mb_s: f32,
    pub status: String,
    pub is_busy: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SetupStep {
    CheckDependencies,
    DownloadUmu,
    DownloadDxvk,
    DownloadJadeite,
    DownloadSteamRuntime,
}

impl SetupStep {
    pub fn description(&self) -> &str {
        match self {
            SetupStep::CheckDependencies => "Checking dependencies",
            SetupStep::DownloadUmu => "Downloading UMU launcher",
            SetupStep::DownloadDxvk => "Downloading DXVK",
            SetupStep::DownloadJadeite => "Downloading Jadeite",
            SetupStep::DownloadSteamRuntime => "Downloading Steam Runtime",
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
