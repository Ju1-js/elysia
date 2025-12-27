use std::process::Command;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::{
    components::tweaks::TweakManifest,
    runners::Runner,
    settings::{GlobalSettings, InstalledGame}
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wine {
    pub version: String,
}

impl Runner for Wine {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<(), String> {
        self.run_game_internal(settings, game)
            .map_err(|e| e.to_string())
    }
}

impl Wine {
    fn run_game_internal(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<()> {
        let components_path = settings.components_directory.join("wine");
        let wine_path = components_path.join(&self.version);
        let wine_bin = wine_path.join("bin/wine");

        let prefix = settings
            .wineprefixes_directory
            .join(&game.biz_name)
            .to_string_lossy()
            .into_owned();

        let game_executable = game.install_path.join(&game.executable_path);

        let manifest = TweakManifest::new();
        let needs_jadeite = manifest.needs_jadeite(&game.id);

        let mut cmd = Command::new(&wine_bin);
        
        cmd.env("WINEPREFIX", &prefix)
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
                "Running with Jadeite: WINEPREFIX=\"{}\" {} {} {}",
                prefix,
                wine_bin.display(),
                jade.display(),
                game_executable.display()
            );

            cmd.arg(&jade);
        } else {
            println!(
                "Running: WINEPREFIX=\"{}\" {} {}",
                prefix,
                wine_bin.display(),
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
