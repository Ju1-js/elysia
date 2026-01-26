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
    pub fn kill_proton_process(proton_path: &str, prefix_path: &str) -> Result<()> {
        let wineserver_path = std::path::Path::new(proton_path).join("files/bin/wineserver");
        let actual_prefix = format!("{prefix_path}/pfx");
        kill_wineserver(&wineserver_path, &actual_prefix)
    }

    pub fn launch_winecfg(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<()> {
        self.launch_utility(settings, game, "winecfg")
    }

    pub fn launch_regedit(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<()> {
        self.launch_utility(settings, game, "regedit")
    }

    fn launch_utility(&self, settings: &GlobalSettings, game: &InstalledGame, utility: &str) -> Result<()> {
        let resolved_version = self.resolve_version(settings)?;
        
        let proton_path = settings.components_directory.join("proton").join(&resolved_version);
        
        if !proton_path.exists() {
            return Err(anyhow::anyhow!("Proton version {resolved_version} not found"));
        }

        let umu_run = Self::find_umu_runtime(settings)?;
        let steam_runtime = Self::find_steam_runtime(settings)?;

        let prefix_path = settings
            .wineprefixes_directory
            .join(&game.biz_name);

        std::fs::create_dir_all(&prefix_path)
            .with_context(|| format!("Failed to create wineprefix directory: {}", prefix_path.display()))?;

        let prefix = prefix_path
            .to_string_lossy()
            .into_owned();

        let mut cmd = Command::new(&umu_run);
        cmd.arg(utility);

        cmd.env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "")
            .env("RUNTIMEPATH", &steam_runtime)
            .env("PROTONPATH", &proton_path);

        for (key, value) in &game.environment {
            cmd.env(key, value);
        }

        if game.enable_winewayland {
            cmd.env("PROTON_ENABLE_WAYLAND", "1");
        }

        println!(
            "Launching {utility}: WINEPREFIX=\"{prefix}\" RUNTIMEPATH=\"{}\" PROTONPATH=\"{}\" {} {utility}",
            steam_runtime.display(),
            proton_path.display(),
            umu_run.display()
        );

        cmd.spawn()
            .with_context(|| format!("Failed to spawn {utility}"))?;

        Ok(())
    }

    pub fn resolve_version(&self, settings: &GlobalSettings) -> Result<String> {
        if !self.version.is_empty() {
            return Ok(self.version.clone());
        }

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

        if game.use_directx11 {
            args.push("-force-d3d11".to_string());
        }

        Ok(args)
    }

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

    fn apply_gamemode(
        program: String,
        args: Vec<String>,
        enable_gamemode: bool,
    ) -> (String, Vec<String>) {
        if enable_gamemode && which::which("gamemoderun").is_ok() {
            let mut gamemode_args = vec![program];
            gamemode_args.extend(args);
            ("gamemoderun".to_string(), gamemode_args)
        } else {
            (program, args)
        }
    }

    fn run_game_internal(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child> {
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

        let umu_run = Self::find_umu_runtime(settings)?;
        let steam_runtime = Self::find_steam_runtime(settings)?;
        let game_args = self.build_game_command(settings, game)?;

        let mut umu_args = vec![umu_run.to_string_lossy().to_string()];
        umu_args.extend(game_args);

        let (mut final_program, mut final_args) =
            Self::apply_command_wrapper(&umu_args, game.command_wrapper.as_ref());

        (final_program, final_args) = Self::apply_gamemode(
            final_program,
            final_args,
            game.enable_gamemode,
        );

        let mut cmd = Command::new(&final_program);
        cmd.args(&final_args);

        cmd.env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "")
            .env("RUNTIMEPATH", &steam_runtime)
            .env("PROTONPATH", &proton_path);

        let manifest = TweakManifest::new();
        if manifest.needs_jadeite(&game.id) {
            cmd.env("JADEITE_ALLOW_UNKNOWN", "1");
        }

        for (key, value) in &game.environment {
            cmd.env(key, value);
        }

        let tweak_env_vars = manifest.get_environment_vars(&game.id);
        for (key, value) in &tweak_env_vars {
            cmd.env(key, value);
        }

        if game.enable_winewayland {
            cmd.env("PROTON_ENABLE_WAYLAND", "1");
        }

        if game.enable_mangohud {
            cmd.env("MANGOHUD", "1");
        }

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
