pub mod proton;
pub mod wine;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::components::{ComponentManager, ComponentType, ComponentVersion};
use crate::progress::ProgressTracker;
pub use crate::runners::{proton::Proton, wine::Wine};
use crate::settings::{GlobalSettings, InstalledGame};

pub trait Runner {
    /// # Errors
    /// Returns an error if the game cannot be started.
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child, String>;
}

/// Kill a Wine/Proton process using wineserver
/// 
/// This is a common utility used by both Wine and Proton runners
/// # Errors
/// Returns an error if wineserver cannot be executed.
pub fn kill_wineserver(wineserver_path: &std::path::Path, prefix_path: &str) -> Result<()> {
    if !wineserver_path.exists() {
        return Err(anyhow::anyhow!("wineserver not found at {}", wineserver_path.display()));
    }

    let status = Command::new(wineserver_path)
        .arg("-k")
        .env("WINEPREFIX", prefix_path)
        .status()
        .context("Failed to execute wineserver")?;

    if !status.success() {
        return Err(anyhow::anyhow!("wineserver -k failed"));
    }

    Ok(())
}

/// Shell-escape a string for safe use in shell commands
pub(crate) fn shell_escape(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Runners {
    Native,
    Wine(Wine),
    Proton(Proton),
}

impl Runner for Runners {
    fn run_game(&self, settings: &GlobalSettings, game: &InstalledGame) -> Result<std::process::Child, String> {
        match self {
            Runners::Native => Err("Native runner not implemented".to_string()),
            Runners::Wine(wine) => wine.run_game(settings, game),
            Runners::Proton(proton) => proton.run_game(settings, game),
        }
    }
}

impl Runners {
    #[must_use] 
    pub fn is_proton(&self) -> bool {
        matches!(self, Runners::Proton(_))
    }

    #[must_use] 
    pub fn is_wine(&self) -> bool {
        matches!(self, Runners::Wine(_))
    }

    #[must_use] 
    pub fn is_native(&self) -> bool {
        matches!(self, Runners::Native)
    }

    pub fn get_wine_status(
        &self,
        settings: &GlobalSettings,
        component_manager: &mut ComponentManager,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        if !self.is_wine() {
            return None;
        }

        let installed_version = component_manager.get_installed_version(settings, ComponentType::Wine);
        let installed = installed_version.is_some();

        // Get 2 latest versions + installed version (avoiding duplicates)
        let available_versions: Vec<ComponentVersion> = component_manager
            .cache
            .entries
            .get(&ComponentType::Wine)
            .map(|versions| {
                let mut result = Vec::new();
                // Add the two latest versions
                result.extend(versions.iter().take(2).cloned());
                
                // Add installed version if it's not already in the list
                if let Some(ref installed_ver) = installed_version
                    && !result.iter().any(|v| &v.version == installed_ver) {
                    // Find the installed version in the full list
                    if let Some(installed_component) = versions.iter().find(|v| &v.version == installed_ver) {
                        result.push(installed_component.clone());
                    }
                }
                
                result
            })
            .unwrap_or_default();

        Some((installed, installed_version, available_versions))
    }

    pub fn get_proton_status(
        &self,
        settings: &GlobalSettings,
        component_manager: &mut ComponentManager,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        if !self.is_proton() {
            return None;
        }

        let installed_version = component_manager.get_installed_version(settings, ComponentType::Proton);
        let installed = installed_version.is_some();

        // Get 2 latest versions from EACH source + installed version (avoiding duplicates)
        let available_versions: Vec<ComponentVersion> = component_manager
            .cache
            .entries
            .get(&ComponentType::Proton)
            .map(|versions| {
                let mut result = Vec::new();
                
                // Group versions by source
                let mut by_source: std::collections::HashMap<Option<String>, Vec<&ComponentVersion>> = std::collections::HashMap::new();
                for version in versions {
                    by_source.entry(version.source.clone()).or_default().push(version);
                }
                
                // Take 2 from each source
                for (_source, source_versions) in by_source {
                    result.extend(source_versions.into_iter().take(2).cloned());
                }
                
                // Add installed version if it's not already in the list
                if let Some(ref installed_ver) = installed_version
                    && !result.iter().any(|v| &v.version == installed_ver) {
                    // Find the installed version in the full list
                    if let Some(installed_component) = versions.iter().find(|v| &v.version == installed_ver) {
                        result.push(installed_component.clone());
                    }
                }
                
                result
            })
            .unwrap_or_default();

        Some((installed, installed_version, available_versions))
    }

    pub fn get_dxvk_status(
        &self,
        settings: &GlobalSettings,
        component_manager: &mut ComponentManager,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        if !self.is_wine() {
            return None;
        }

        let installed_version = component_manager.get_installed_version(settings, ComponentType::Dxvk);
        let installed = installed_version.is_some();

        // Get 2 latest versions + installed version (avoiding duplicates)
        let available_versions: Vec<ComponentVersion> = component_manager
            .cache
            .entries
            .get(&ComponentType::Dxvk)
            .map(|versions| {
                let mut result = Vec::new();
                // Add the two latest versions
                result.extend(versions.iter().take(2).cloned());
                
                // Add installed version if it's not already in the list
                if let Some(ref installed_ver) = installed_version
                    && !result.iter().any(|v| &v.version == installed_ver) {
                    // Find the installed version in the full list
                    if let Some(installed_component) = versions.iter().find(|v| &v.version == installed_ver) {
                        result.push(installed_component.clone());
                    }
                }
                
                result
            })
            .unwrap_or_default();

        Some((installed, installed_version, available_versions))
    }

    /// # Errors
    /// Returns an error if download fails.
    pub async fn download_proton(
        settings: &GlobalSettings,
        component_manager: &mut ComponentManager,
        version: Option<&str>,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<String> {
        let _ = component_manager.refresh_component(ComponentType::Proton).await;

        let display_name = component_manager
            .get_latest_version(ComponentType::Proton).map_or_else(|| "Proton".to_string(), |v| v.display_name.clone());

        let dest_path = component_manager
            .download_component(
                settings,
                ComponentType::Proton,
                version,
                progress_tracker.map(|pt| {
                    let pt = pt.clone();
                    let key = progress_key.to_string();
                    let name = display_name.clone();
                    Box::new(move |current, total| {
                        pt.report(&key, &name, crate::progress::ReportParams {
                            downloaded: current,
                            total,
                            is_busy: true,
                            step_index: None,
                            total_steps: None,
                        });
                    }) as Box<dyn Fn(u64, u64) + Send>
                }),
            )
            .await?;

        if let Some(pt) = progress_tracker {
            pt.finish(progress_key);
        }

        // Extract the version from the destination path
        let downloaded_version = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract version from path"))?
            .to_string();

        Ok(downloaded_version)
    }

    /// # Errors
    /// Returns an error if download fails.
    pub async fn download_wine(
        settings: &GlobalSettings,
        component_manager: &ComponentManager,
        version: Option<&str>,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<String> {
        let display_name = component_manager
            .get_latest_version(ComponentType::Wine).map_or_else(|| "Wine".to_string(), |v| v.display_name.clone());

        let dest_path = component_manager
            .download_component(
                settings,
                ComponentType::Wine,
                version,
                progress_tracker.map(|pt| {
                    let pt = pt.clone();
                    let key = progress_key.to_string();
                    let name = display_name.clone();
                    Box::new(move |current, total| {
                        pt.report(&key, &name, crate::progress::ReportParams {
                            downloaded: current,
                            total,
                            is_busy: true,
                            step_index: None,
                            total_steps: None,
                        });
                    }) as Box<dyn Fn(u64, u64) + Send>
                }),
            )
            .await?;

        if let Some(pt) = progress_tracker {
            pt.finish(progress_key);
        }

        // Extract the version from the destination path
        let downloaded_version = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract version from path"))?
            .to_string();

        Ok(downloaded_version)
    }

    /// # Errors
    /// Returns an error if download fails.
    pub async fn download_proton_runtime(
        settings: &GlobalSettings,
        component_manager: &mut ComponentManager,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<()> {
        crate::components::install_proton_runtime(
            settings,
            component_manager,
            progress_tracker,
            progress_key,
        )
        .await
    }

    /// # Errors
    /// Returns an error if download fails.
    pub async fn download_dxvk(
        settings: &GlobalSettings,
        component_manager: &ComponentManager,
        version: Option<&str>,
        progress_tracker: Option<&ProgressTracker>,
        progress_key: &str,
    ) -> Result<String> {
        let display_name = component_manager
            .get_latest_version(ComponentType::Dxvk).map_or_else(|| "DXVK".to_string(), |v| v.display_name.clone());

        let dest_path = component_manager
            .download_component(
                settings,
                ComponentType::Dxvk,
                version,
                progress_tracker.map(|pt| {
                    let pt = pt.clone();
                    let key = progress_key.to_string();
                    let name = display_name.clone();
                    Box::new(move |current, total| {
                        pt.report(&key, &name, crate::progress::ReportParams {
                            downloaded: current,
                            total,
                            is_busy: true,
                            step_index: None,
                            total_steps: None,
                        });
                    }) as Box<dyn Fn(u64, u64) + Send>
                }),
            )
            .await?;

        if let Some(pt) = progress_tracker {
            pt.finish(progress_key);
        }

        // Extract the version from the destination path
        let downloaded_version = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract version from path"))?
            .to_string();

        Ok(downloaded_version)
    }

}
