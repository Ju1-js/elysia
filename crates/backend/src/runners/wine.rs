use crate::{
    components::tweaks::TweakManifest,
    runners::{Runner, kill_wineserver, shell_escape},
    settings::{GlobalSettings, InstalledGame, RuntimeComponents},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wine {
    pub version: String,
}

impl Runner for Wine {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child, String> {
        self.run_game_internal(settings, game)
            .map_err(|e| e.to_string())
    }
}

impl Wine {
    /// Kill a Wine process using wineserver
    /// # Errors
    /// Returns an error if wineserver cannot be executed.
    pub fn kill_wine_process(wine_path: &str, prefix_path: &str) -> Result<()> {
        // For system wine (indicated by /usr/bin path), use wineserver from PATH directly
        // This is consistent with version_loader.rs which detects system wine at /usr/bin/wine
        // This allows the system's wineserver to be found via PATH lookup
        if wine_path == "/usr/bin" || wine_path.ends_with("/usr/bin") {
            let status = std::process::Command::new("wineserver")
                .arg("-k")
                .env("WINEPREFIX", prefix_path)
                .status()
                .context("Failed to execute wineserver")?;

            if !status.success() {
                return Err(anyhow::anyhow!("wineserver -k failed"));
            }
            Ok(())
        } else {
            let wineserver_path = std::path::Path::new(wine_path).join("bin/wineserver");
            kill_wineserver(&wineserver_path, prefix_path)
        }
    }

    /// Resolve the Wine version to use
    /// If version is empty, returns the latest installed version
    fn resolve_version(&self, settings: &GlobalSettings) -> Result<String> {
        if !self.version.is_empty() {
            return Ok(self.version.clone());
        }

        // Version is empty, find installed versions and pick the latest one
        let base_dir = settings.components_directory.join("wine");
        
        let mut versions: Vec<String> = std::fs::read_dir(&base_dir)
            .context("Failed to read wine directory")?
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().to_str().map(String::from))
            .collect();
        
        versions.sort();
        
        versions.into_iter().next_back()
            .context("No Wine version installed. Please install Wine from settings.")
    }

    /// Setup DXVK by copying DLLs to the Wine prefix
    #[allow(clippy::unused_self)]
    fn setup_dxvk(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<Vec<String>> {
        let dxvk_dir = settings.components_directory.join("dxvk");
        
        // Find the DXVK version to use from runtime_components
        let dxvk_version = game.runtime_components.iter()
            .find_map(|component| {
                if let RuntimeComponents::Dxvk(version) = component {
                    Some(version.clone())
                } else {
                    None
                }
            });

        // Determine the DXVK path: use specified version or fall back to any available
        let dxvk_path = if let Some(ref version) = dxvk_version {
            println!("Using DXVK version: {version} (configured)");
            let version_path = dxvk_dir.join(version);
            if version_path.exists() && version_path.is_dir() {
                Some(version_path)
            } else {
                None
            }
        } else {
            println!("Using DXVK version: auto (any available)");
            // No version specified, find any DXVK version directory
            std::fs::read_dir(&dxvk_dir)
                .ok()
                .and_then(|entries| {
                    entries
                        .filter_map(std::result::Result::ok)
                        .find(|e| e.path().is_dir())
                        .map(|e| e.path())
                })
        };

        let Some(dxvk_path) = dxvk_path else {
            return Ok(Vec::new());
        };

        let prefix_dir = settings.wineprefixes_directory.join(&game.biz_name);
        let system32 = prefix_dir.join("drive_c/windows/system32");
        let syswow64 = prefix_dir.join("drive_c/windows/syswow64");

        std::fs::create_dir_all(&system32)?;
        std::fs::create_dir_all(&syswow64)?;

        let dxvk_dlls = ["dxgi", "d3d11", "d3d10core", "d3d9"];
        let mut dll_overrides = Vec::new();

        for dll_name in &dxvk_dlls {
            let dll_file = format!("{dll_name}.dll");

            // Copy 64-bit DLL
            let src_x64 = dxvk_path.join("x64").join(&dll_file);
            let dst_x64 = system32.join(&dll_file);
            if src_x64.exists() {
                std::fs::copy(&src_x64, &dst_x64)
                    .with_context(|| format!("Failed to copy {dll_file} to system32"))?;
            }

            // Copy 32-bit DLL
            let src_x32 = dxvk_path.join("x32").join(&dll_file);
            let dst_x32 = syswow64.join(&dll_file);
            if src_x32.exists() {
                std::fs::copy(&src_x32, &dst_x32)
                    .with_context(|| format!("Failed to copy {dll_file} to syswow64"))?;
            }

            dll_overrides.push(format!("{dll_name}=n"));
        }

        Ok(dll_overrides)
    }

    /// Build the wine command with optional Jadeite injection
    #[allow(clippy::unused_self)]
    fn build_wine_command(
        &self,
        settings: &GlobalSettings,
        game: &InstalledGame,
        wine_bin: &std::path::Path,
    ) -> Result<Vec<String>> {
        let manifest = TweakManifest::new();
        let needs_jadeite = manifest.needs_jadeite(&game.id);
        
        let mut wine_args = vec![wine_bin.to_string_lossy().to_string()];

        if needs_jadeite {
            let jadeite_dir = settings.components_directory.join("jadeite");
            
            // Look for Jadeite in version subdirectories first, then fall back to base directory
            let jade = std::fs::read_dir(&jadeite_dir)
                .context("Failed to read jadeite directory")?
                .filter_map(std::result::Result::ok)
                .find(|entry| {
                    let path = entry.path();
                    // Check if this is a version directory with jadeite.exe
                    path.is_dir() && path.join("jadeite.exe").exists()
                })
                .map(|entry| entry.path().join("jadeite.exe"))
                // Fallback: check base directory
                .or_else(|| {
                    let base_jade = jadeite_dir.join("jadeite.exe");
                    base_jade.exists().then_some(base_jade)
                })
                .context("Jadeite executable not found")?;

            wine_args.push(jade.to_string_lossy().to_string());
        }

        wine_args.push(
            game.install_path
                .join(&game.executable_path)
                .to_string_lossy()
                .to_string(),
        );

        if let Some(ref args) = game.command_arguments {
            wine_args.extend(args.iter().cloned());
        }

        Ok(wine_args)
    }

    /// Apply command wrapper if specified
    fn apply_command_wrapper(
        wine_args: &[String],
        wrapper: Option<&String>,
    ) -> (String, Vec<String>) {
        if let Some(wrapper) = wrapper {
            let command_str = wine_args
                .iter()
                .map(|arg| shell_escape(arg))
                .collect::<Vec<_>>()
                .join(" ");
            
            let wrapper_expanded = wrapper.replace("%command%", &command_str);
            
            ("sh".to_string(), vec!["-c".to_string(), wrapper_expanded])
        } else {
            let program = wine_args[0].clone();
            let args = wine_args[1..].to_vec();
            (program, args)
        }
    }

    fn run_game_internal(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child> {
        // Resolve version (use first installed version if empty)
        let resolved_version = self.resolve_version(settings)?;
        
        println!("Using Wine version: {} (configured: {})", 
            resolved_version,
            if self.version.is_empty() { "auto" } else { &self.version }
        );
        
        // Handle "system" wine version - use wine from PATH instead of components directory
        let wine_bin = if resolved_version == "system" {
            std::path::PathBuf::from("wine")
        } else {
            let components_path = settings.components_directory.join("wine");
            let wine_path = components_path.join(&resolved_version);
            wine_path.join("bin/wine")
        };

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

        // Setup DXVK
        let mut dll_overrides = self.setup_dxvk(settings, game)?;

        // Build wine command with optional Jadeite
        let wine_args = self.build_wine_command(settings, game, &wine_bin)?;

        // Apply command wrapper if specified
        let (final_program, final_args) =
            Self::apply_command_wrapper(&wine_args, game.command_wrapper.as_ref());

        // Build the command
        let mut cmd = Command::new(&final_program);
        cmd.args(&final_args);

        cmd.env("WINEPREFIX", &prefix).env("WINEDEBUG", "");

        // Merge with existing WINEDLLOVERRIDES
        if let Some(existing) = game.environment.get("WINEDLLOVERRIDES") {
            dll_overrides.push(existing.clone());
        }

        if !dll_overrides.is_empty() {
            cmd.env("WINEDLLOVERRIDES", dll_overrides.join(";"));
        }

        // Check if Jadeite is needed
        let manifest = TweakManifest::new();
        let needs_jadeite = manifest.needs_jadeite(&game.id);
        
        if needs_jadeite {
            cmd.env("JADEITE_ALLOW_UNKNOWN", "1");
        }

        // Apply user environment variables
        for (key, value) in &game.environment {
            if key != "WINEDLLOVERRIDES" {
                cmd.env(key, value);
            }
        }

        // Apply game specific environmental variables
        let tweak_env_vars = manifest.get_environment_vars(&game.id);
        for (key, value) in &tweak_env_vars {
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

        // Redirect stdout and stderr to log file
        cmd.stdout(log_file.try_clone()?);
        cmd.stderr(log_file);

        println!(
            "Running: WINEPREFIX=\"{prefix}\" {final_program} {final_args:?}"
        );
        println!("Logging to: {}", log_path.display());

        // If using Jadeite, we need to monitor the Wine prefix instead of the launcher PID
        if needs_jadeite {
            cmd.spawn()
                .with_context(|| format!("Failed to launch game: wine={}, exe={}", 
                    final_program, 
                    game.install_path.join(&game.executable_path).display()
                ))?;
            
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            let wineserver = if resolved_version == "system" {
                std::path::PathBuf::from("wineserver")
            } else {
                wine_bin.parent()
                    .context("Failed to get wine bin parent directory")?
                    .join("wineserver")
            };
            
            let monitor = std::process::Command::new(wineserver)
                .arg("-w")
                .env("WINEPREFIX", &prefix)
                .spawn()
                .context("Failed to spawn wineserver monitor")?;
            
            Ok(monitor)
        } else {
            let child = cmd.spawn()
                .with_context(|| format!("Failed to launch game: wine={}, exe={}", 
                    final_program, 
                    game.install_path.join(&game.executable_path).display()
                ))?;
            
            Ok(child)
        }
    }
}
