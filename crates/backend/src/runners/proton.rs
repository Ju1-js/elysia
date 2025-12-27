use std::process::Command;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::{components::tweaks::TweakManifest, runners::Runner, settings::{GlobalSettings, InstalledGame}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proton {
    pub version: String,
}

impl Runner for Proton {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<(), String> {
        self.run_game_internal(settings, game)
            .map_err(|e| e.to_string())
    }
}

impl Proton {
    fn run_game_internal(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<()> {
        let components_path = settings.components_directory.join("proton");
        let proton_path = components_path.join(&self.version);
        
        let umu_base_dir = settings.components_directory.join("umu");
        let umu_version_dir = std::fs::read_dir(&umu_base_dir)
            .context("Failed to read umu directory")?
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.path().is_dir())
            .context("No umu version found")?
            .path();
        let umu_run = umu_version_dir.join("umu").join("umu-run");

        anyhow::ensure!(
            umu_run.exists(),
            "umu-run not found at {}",
            umu_run.display()
        );

        let prefix = settings
            .wineprefixes_directory
            .join(&game.biz_name)
            .to_string_lossy()
            .into_owned();

        let game_executable = game.install_path.join(&game.executable_path);

        let manifest = TweakManifest::new();
        let needs_jadeite = manifest.needs_jadeite(&game.id);

        let steamrt_dir = settings.components_directory.join("steamrt");
        let runtime_path = std::fs::read_dir(&steamrt_dir)
            .context("Failed to read steamrt directory")?
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.path().is_dir())
            .context("No steamrt version found")?
            .path()
            .join("SteamLinuxRuntime_sniper");

        anyhow::ensure!(
            runtime_path.exists(),
            "Steam runtime not found at {}",
            runtime_path.display()
        );

        let mut cmd = Command::new(&umu_run);
        
        cmd.env("PROTONPATH", &proton_path)
            .env("WINEPREFIX", &prefix)
            .env("RUNTIMEPATH", &runtime_path)
            .env("PROTONFIXES_DISABLE", "1")
            .env("UMU_RUNTIME_UPDATE", "0")
            .env("WINEDEBUG", "");

        if needs_jadeite {
            let jadeite_dir = settings.components_directory.join("jadeite");
            
            let jade = std::fs::read_dir(&jadeite_dir)
                .context("Failed to read jadeite directory")?
                .filter_map(|entry| entry.ok())
                .find(|entry| entry.path().is_dir())
                .context("No jadeite version found")?
                .path()
                .join("jadeite.exe");

            anyhow::ensure!(
                jade.exists(),
                "Jadeite executable not found at {}",
                jade.display()
            );

            cmd.env("JADEITE_ALLOW_UNKNOWN", "1");

            println!(
                "Running with Jadeite: PROTONPATH=\"{}\" WINEPREFIX=\"{}\" RUNTIMEPATH=\"{}\" {} {} {}",
                proton_path.display(),
                prefix,
                runtime_path.display(),
                umu_run.display(),
                jade.display(),
                game_executable.display()
            );

            cmd.arg(&jade);
        } else {
            println!(
                "Running: PROTONPATH=\"{}\" WINEPREFIX=\"{}\" RUNTIMEPATH=\"{}\" {} {}",
                proton_path.display(),
                prefix,
                runtime_path.display(),
                umu_run.display(),
                game_executable.display()
            );
        }

        cmd.arg(&game_executable);

        if let Some(ref args) = game.command_arguments {
            cmd.args(args);
        }

        for (key, value) in &game.environment {
            cmd.env(key, value);
        }

        cmd.spawn()
            .context("Failed to launch game")?;

        Ok(())
    }
}
