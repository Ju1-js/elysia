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
        // Proton uses a /pfx subdirectory for the actual Wine prefix
        let actual_prefix = format!("{prefix_path}/pfx");
        kill_wineserver(&wineserver_path, &actual_prefix)
    }

    /// Launch winecfg utility with proper environment settings
    /// # Errors
    /// Returns an error if winecfg cannot be launched.
    pub fn launch_winecfg(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<()> {
        self.launch_utility(settings, game, "winecfg")
    }

    /// Launch regedit utility with proper environment settings
    /// # Errors
    /// Returns an error if regedit cannot be launched.
    pub fn launch_regedit(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<()> {
        self.launch_utility(settings, game, "regedit")
    }

    /// Launch a Wine utility with proper environment settings
    /// # Errors
    /// Returns an error if the utility cannot be launched.
    fn launch_utility(&self, settings: &GlobalSettings, game: &InstalledGame, utility: &str) -> Result<()> {
        // Resolve version
        let resolved_version = self.resolve_version(settings)?;
        
        let proton_path = settings.components_directory.join("proton").join(&resolved_version);
        
        if !proton_path.exists() {
            return Err(anyhow::anyhow!("Proton version {resolved_version} not found"));
        }

        // Find UMU runtime
        let umu_run = Self::find_umu_runtime(settings)?;

        let prefix_path = settings
            .wineprefixes_directory
            .join(&game.biz_name);

        // Ensure the wineprefix directory exists before running utilities
        std::fs::create_dir_all(&prefix_path)
            .with_context(|| format!("Failed to create wineprefix directory: {}", prefix_path.display()))?;

        let prefix = prefix_path
            .to_string_lossy()
            .into_owned();

        // Build the command
        let mut cmd = Command::new(&umu_run);
        cmd.arg(utility);

        cmd.env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "")
            .env("PROTONPATH", &proton_path);

        // Apply user environment variables from game settings
        for (key, value) in &game.environment {
            cmd.env(key, value);
        }

        // Handle Wayland
        if game.enable_winewayland {
            cmd.env("PROTON_ENABLE_WAYLAND", "1");
        }

        println!(
            "Launching {utility}: WINEPREFIX=\"{prefix}\" PROTONPATH=\"{}\" {} {utility}",
            proton_path.display(),
            umu_run.display()
        );

        cmd.spawn()
            .with_context(|| format!("Failed to spawn {utility}"))?;

        Ok(())
    }

    /// Resolve the Proton version to use
    /// If version is empty, returns the latest installed version
    /// # Errors
    /// Returns an error if the proton directory cannot be read or if no Proton versions are installed.
    pub fn resolve_version(&self, settings: &GlobalSettings) -> Result<String> {
        if !self.version.is_empty() {
            return Ok(self.version.clone());
        }

        // Version is empty, find installed versions and pick the latest one
        let base_dir = settings.components_directory.join("proton");
        
        let mut versions: Vec<String> = std::fs::read_dir(&base_dir)
            .context("Failed to read proton directory")?
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().to_str().map(String::from))
            .collect();
        
        versions.sort();
        
        versions.into_iter().next_back()
            .context("No Proton version installed. Please install Proton from settings.")
    }

    /// Find the UMU runtime path
    fn find_umu_runtime(settings: &GlobalSettings) -> Result<std::path::PathBuf> {
        let umu_dir = settings.components_directory.join("umu");
        
        std::fs::read_dir(&umu_dir)
            .context("Failed to read umu directory")?
            .filter_map(std::result::Result::ok)
            .find(|entry| {
                let path = entry.path();
                path.is_dir() && path.join("umu-run").exists()
            })
            .map(|entry| entry.path().join("umu-run"))
            .context("umu-run executable not found")
    }

    /// Find the Steam Runtime path
    fn find_steam_runtime(settings: &GlobalSettings) -> Result<std::path::PathBuf> {
        let steamrt_dir = settings.components_directory.join("steamrt");
        
        std::fs::read_dir(&steamrt_dir)
            .context("Failed to read steamrt directory")?
            .filter_map(std::result::Result::ok)
            .find(|entry| {
                let path = entry.path();
                path.is_dir() && path.join("SteamLinuxRuntime_sniper").exists()
            })
            .map(|entry| entry.path().join("SteamLinuxRuntime_sniper"))
            .context("SteamLinuxRuntime_sniper not found")
    }

    /// Build the game command with optional Jadeite injection
    #[allow(clippy::unused_self)]
    fn build_game_command(
        &self,
        settings: &GlobalSettings,
        game: &InstalledGame,
    ) -> Result<Vec<String>> {
        let manifest = TweakManifest::new();
        let needs_jadeite = manifest.needs_jadeite(&game.id);
        
        let mut args = Vec::new();

        if needs_jadeite {
            let jadeite_dir = settings.components_directory.join("jadeite");
            
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
        // Resolve version (use first installed version if empty)
        let resolved_version = self.resolve_version(settings)?;
        
        println!("Using Proton version: {} (configured: {})", 
            resolved_version,
            if self.version.is_empty() { "auto" } else { &self.version }
        );
        
        let components_path = settings.components_directory.join("proton");
        let proton_path = components_path.join(&resolved_version);

        let prefix = settings
            .wineprefixes_directory
            .join(&game.biz_name)
            .to_string_lossy()
            .into_owned();

        // Create log file path
        let log_path = settings.components_directory
            .parent()
            .unwrap_or(&settings.components_directory)
            .join("game.log");
        
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
            .context("Failed to create log file")?;

        // Find UMU and Steam Runtime
        let umu_run = Self::find_umu_runtime(settings)?;
        let steam_runtime = Self::find_steam_runtime(settings)?;

        // Build game command with optional Jadeite
        let game_args = self.build_game_command(settings, game)?;

        // Build umu-run command: umu-run <game_exe> <game_args>
        let mut umu_args = vec![umu_run.to_string_lossy().to_string()];
        umu_args.extend(game_args);

        // Apply command wrapper if specified
        let (final_program, final_args) =
            Self::apply_command_wrapper(&umu_args, game.command_wrapper.as_ref());

        // Build the command
        let mut cmd = Command::new(&final_program);
        cmd.args(&final_args);

        cmd.env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "")
            .env("RUNTIMEPATH", &steam_runtime)
            .env("PROTONPATH", &proton_path);

        // Check if Jadeite is needed
        let manifest = TweakManifest::new();
        if manifest.needs_jadeite(&game.id) {
            cmd.env("JADEITE_ALLOW_UNKNOWN", "1");
        }

        // Apply user environment variables
        for (key, value) in &game.environment {
            cmd.env(key, value);
        }

        // Apply game specific environmental variables
        let tweak_env_vars = manifest.get_environment_vars(&game.id);
        for (key, value) in &tweak_env_vars {
            cmd.env(key, value);
        }

        // Handle Wayland
        if game.enable_winewayland {
            cmd.env("PROTON_ENABLE_WAYLAND", "1");
        }

        // Handle MangoHud
        if game.enable_mangohud {
            cmd.env("MANGOHUD", "1");
        }

        // Redirect stdout and stderr to log file
        cmd.stdout(log_file.try_clone()?);
        cmd.stderr(log_file);

        println!(
            "Running: WINEPREFIX=\"{prefix}\" RUNTIMEPATH=\"{}\" PROTONPATH=\"{}\" {final_program} {final_args:?}",
            steam_runtime.display(),
            proton_path.display()
        );
        println!("Logging to: {}", log_path.display());

        let child = cmd.spawn().context("Failed to launch game")?;

        Ok(child)
    }
}
