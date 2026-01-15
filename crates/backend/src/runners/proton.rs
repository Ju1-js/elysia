use crate::{
    components::tweaks::TweakManifest,
    runners::{Runner, kill_wineserver, shell_escape},
    settings::{GlobalSettings, InstalledGame},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proton {
    pub version: String,
}

impl Runner for Proton {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child, String> {
        self.run_game_internal(settings, game)
            .map_err(|e| e.to_string())
    }
}

impl Proton {
    /// Kill a Proton process using wineserver
    /// # Errors
    /// Returns an error if wineserver cannot be executed.
    pub fn kill_proton_process(proton_path: &str, prefix_path: &str) -> Result<()> {
        let wineserver_path = std::path::Path::new(proton_path).join("files/bin/wineserver");
        kill_wineserver(&wineserver_path, prefix_path)
    }

    /// Build the proton command with optional Jadeite injection
    #[allow(clippy::unused_self)]
    fn build_proton_command(
        &self,
        settings: &GlobalSettings,
        game: &InstalledGame,
        proton_bin: &std::path::Path,
    ) -> Result<Vec<String>> {
        let manifest = TweakManifest::new();
        let needs_jadeite = manifest.needs_jadeite(&game.id);
        
        let mut args = vec![proton_bin.to_string_lossy().to_string()];

        if needs_jadeite {
            let jadeite_dir = settings.components_directory.join("jadeite");
            
            // Look for Jadeite in version subdirectories first, then fall back to base directory
            let jade = std::fs::read_dir(&jadeite_dir)
                .context("Failed to read jadeite directory")?
                .filter_map(std::result::Result::ok)
                .find(|entry| {
                    let path = entry.path();
                    path.is_dir() && path.join("jadeite.exe").exists()
                })
                .map(|entry| entry.path().join("jadeite.exe"))
                .or_else(|| {
                    let base_jade = jadeite_dir.join("jadeite.exe");
                    base_jade.exists().then_some(base_jade)
                })
                .context("Jadeite executable not found")?;

            args.push(jade.to_string_lossy().to_string());
        }

        args.push(
            game.install_path
                .join(&game.executable_path)
                .to_string_lossy()
                .to_string(),
        );

        if let Some(ref game_args) = game.command_arguments {
            args.extend(game_args.iter().cloned());
        }

        Ok(args)
    }

    /// Apply command wrapper if specified
    fn apply_command_wrapper(
        args: &[String],
        wrapper: Option<&String>,
    ) -> (String, Vec<String>) {
        if let Some(wrapper) = wrapper {
            let command_str = args
                .iter()
                .map(|arg| shell_escape(arg))
                .collect::<Vec<_>>()
                .join(" ");
            
            let wrapper_expanded = wrapper.replace("%command%", &command_str);
            
            ("sh".to_string(), vec!["-c".to_string(), wrapper_expanded])
        } else {
            let program = args[0].clone();
            let args = args[1..].to_vec();
            (program, args)
        }
    }

    fn run_game_internal(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child> {
        let components_path = settings.components_directory.join("proton");
        let proton_path = components_path.join(&self.version);
        let proton_bin = proton_path.join("proton");

        let prefix = settings
            .wineprefixes_directory
            .join(&game.biz_name)
            .to_string_lossy()
            .into_owned();

        // Build proton command with optional Jadeite
        let proton_args = self.build_proton_command(settings, game, &proton_bin)?;

        // Apply command wrapper if specified
        let (final_program, final_args) =
            Self::apply_command_wrapper(&proton_args, game.command_wrapper.as_ref());

        // Build the command
        let mut cmd = Command::new(&final_program);
        cmd.args(&final_args);

        cmd.env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "")
            .env("STEAM_COMPAT_DATA_PATH", &prefix);

        // Check if Jadeite is needed
        let manifest = TweakManifest::new();
        if manifest.needs_jadeite(&game.id) {
            cmd.env("JADEITE_ALLOW_UNKNOWN", "1");
        }

        // Apply user environment variables
        for (key, value) in &game.environment {
            cmd.env(key, value);
        }

        // Handle Wayland
        if game.enable_winewayland {
            cmd.env("DISPLAY", "");
        }

        // Handle MangoHud
        if game.enable_mangohud {
            cmd.env("MANGOHUD", "1");
        }

        println!(
            "Running: WINEPREFIX=\"{prefix}\" {final_program} {final_args:?}"
        );

        let child = cmd.spawn().context("Failed to launch game")?;

        Ok(child)
    }
}
