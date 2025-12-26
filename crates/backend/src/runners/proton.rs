use std::process::Command;
use serde::{Deserialize, Serialize};
use crate::{components::tweaks::TweakManifest, runners::Runner, settings::{GlobalSettings, InstalledGame}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proton {
    pub version: String,
}

impl Runner for Proton {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<(), String> {
        let components_path = settings.components_directory.join("proton");
        let proton_path = components_path.join(&self.version);
        let umu_dir = settings.components_directory.join("umu-launcher");
        let umu_run = umu_dir.join("umu-run");
        
        let prefix = settings
            .wineprefixes_directory
            .join(&game.biz_name)
            .to_string_lossy()
            .into_owned();
        
        let game_executable = game.install_path.join(&game.executable_path);
        let manifest = TweakManifest::new();

        let needs_jadeite = manifest.needs_jadeite(&game.id);
        
        let mut cmd = Command::new(&umu_run);
        cmd.env("PROTONPATH", &proton_path)
           .env("WINEPREFIX", &prefix)
           .env("PROTONFIXES_DISABLE", "1")
           .env("WINEDEBUG", "");
        
        if needs_jadeite {
            let jade = settings.components_directory
                .join("tweaks")
                .join("jadeite")
                .join("jadeite.exe");
            
            cmd.env("JADEITE_ALLOW_UNKNOWN", "1");

            println!(
                "Running with Jadeite: PROTONPATH=\"{}\" WINEPREFIX=\"{}\" {} {} {}",
                proton_path.display(),
                prefix,
                umu_run.display(),
                jade.display(),
                game_executable.display()
            );
            
            cmd.arg(&jade);
        } else {
            println!(
                "Running: PROTONPATH=\"{}\" WINEPREFIX=\"{}\" {} {}",
                proton_path.display(),
                prefix,
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
            .map_err(|e| format!("Failed to launch game: {}", e))?;
        
        Ok(())
    }
}