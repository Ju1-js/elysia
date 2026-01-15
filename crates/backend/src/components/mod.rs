use anyhow::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tokio_stream::StreamExt;

mod dxvk;
pub mod installer;
mod jadeite;
mod proton;
pub mod steamrt;
pub mod tweaks;
mod umu;
mod wine;

pub use installer::{ComponentRequirement, install_components, install_proton_runtime};

use crate::settings::GlobalSettings;

#[derive(Serialize, PartialEq, Eq, Hash, Deserialize, Debug, Clone, Copy)]
pub enum ComponentType {
    Dxvk,
    Jadeite,
    Umu,
    SteamRuntime,
    Wine,
    Proton,
}

impl ComponentType {
    fn name(self) -> &'static str {
        match self {
            ComponentType::Dxvk => "dxvk",
            ComponentType::Jadeite => "jadeite",
            ComponentType::Umu => "umu",
            ComponentType::SteamRuntime => "steamrt",
            ComponentType::Wine => "wine",
            ComponentType::Proton => "proton",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            ComponentType::Dxvk => "DXVK",
            ComponentType::Jadeite => "Jadeite",
            ComponentType::Umu => "UMU Launcher",
            ComponentType::SteamRuntime => "Steam Runtime",
            ComponentType::Wine => "Wine",
            ComponentType::Proton => "Proton",
        }
    }

    async fn fetch_versions(&self) -> Result<Vec<ComponentVersion>> {
        match self {
            ComponentType::Dxvk => dxvk::fetch_versions().await,
            ComponentType::Jadeite => jadeite::fetch_versions().await,
            ComponentType::Umu => umu::fetch_versions().await,
            ComponentType::SteamRuntime => Ok(vec![]),
            ComponentType::Wine => wine::fetch_versions().await,
            ComponentType::Proton => proton::fetch_versions().await,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ComponentVersion {
    pub version: String,
    pub download_url: Url,
    pub display_name: String,
}

#[derive(Serialize, Default, Deserialize)]
pub struct VersionCache {
    pub entries: HashMap<ComponentType, Vec<ComponentVersion>>,
}

pub struct ComponentManager {
    pub cache: VersionCache,
}

impl ComponentManager {
    #[allow(clippy::unused_async)]
    pub async fn new() -> Self {
        Self {
            cache: VersionCache::default(),
        }
    }

    /// # Errors
    /// Returns an error if fetching versions fails.
    pub async fn refresh_index(&mut self) -> Result<()> {
        let types = [
            ComponentType::Dxvk,
            ComponentType::Jadeite,
            ComponentType::Umu,
            ComponentType::Wine,
            ComponentType::Proton,
        ];
        let handles = types
            .iter()
            .map(|&t| tokio::spawn(async move { (t, t.fetch_versions().await) }))
            .collect::<Vec<_>>();

        for handle in handles {
            let (component_type, versions) = handle.await?;
            match versions {
                Ok(versions) => {
                    self.cache.entries.insert(component_type, versions);
                }
                Err(err) => {
                    eprintln!("Failed to fetch versions for {component_type:?}: {err}");
                }
            }
        }
        Ok(())
    }

    /// # Errors
    /// Returns an error if fetching versions fails.
    pub async fn refresh_component(&mut self, component_type: ComponentType) -> Result<()> {
        let versions = component_type.fetch_versions().await?;
        self.cache.entries.insert(component_type, versions);
        Ok(())
    }

    #[must_use] 
    pub fn is_installed(&self, settings: &GlobalSettings, component_type: ComponentType) -> bool {
        let base_dir = settings.components_directory.join(component_type.name());

        eprintln!("[ComponentManager::is_installed] Checking {component_type:?} at path: {}", base_dir.display());

        if !base_dir.exists() {
            eprintln!("[ComponentManager::is_installed] Base directory does not exist");
            return false;
        }

        // Check for version subdirectories (Wine, DXVK, Proton, UMU, Jadeite)
        let result = std::fs::read_dir(&base_dir)
            .ok()
            .and_then(|entries| {
                let dirs: Vec<_> = entries
                    .filter_map(std::result::Result::ok)
                    .filter(|e| e.path().is_dir())
                    .collect();
                
                eprintln!("[ComponentManager::is_installed] Found {} subdirectories:", dirs.len());
                for dir in &dirs {
                    eprintln!("  - {}", dir.file_name().to_string_lossy());
                }
                
                dirs.into_iter().next()
            })
            .is_some();
        
        eprintln!("[ComponentManager::is_installed] Result: {result}");
        result
    }

    #[must_use] 
    pub fn get_installed_version(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
    ) -> Option<String> {
        let base_dir = settings.components_directory.join(component_type.name());

        std::fs::read_dir(&base_dir)
            .ok()?
            .flatten()
            .find(|e| e.path().is_dir())
            .and_then(|e| e.file_name().to_str().map(String::from))
    }

    #[must_use] 
    pub fn get_latest_version(&self, component_type: ComponentType) -> Option<&ComponentVersion> {
        self.cache.entries.get(&component_type)?.first()
    }

    #[must_use] 
    pub fn needs_update(&self, settings: &GlobalSettings, component_type: ComponentType) -> bool {
        let Some(installed) = self.get_installed_version(settings, component_type) else { return false };

        let latest = match self.get_latest_version(component_type) {
            Some(v) => &v.version,
            None => return false,
        };

        installed != *latest
    }

    /// # Errors
    /// Returns an error if download or installation fails.
    /// # Panics
    /// Panics if the destination directory has no parent directory.
    pub async fn download_component(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
        version: Option<&str>,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> Result<PathBuf> {
        let component_version = if let Some(ver) = version {
            self.cache
                .entries
                .get(&component_type)
                .and_then(|versions| versions.iter().find(|v| v.version == ver))
                .ok_or_else(|| anyhow::anyhow!("Version {ver} not found in cache"))?
        } else {
            self.get_latest_version(component_type)
                .ok_or_else(|| anyhow::anyhow!("No versions available for {component_type:?}"))?
        };

        println!(
            "Downloading: {} ({})",
            component_version.display_name, component_version.version
        );

        let temp_dir = std::env::temp_dir();
        let filename = component_version
            .download_url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .ok_or_else(|| anyhow::anyhow!("Invalid download URL"))?;
        let archive_path = temp_dir.join(filename);

        // Download file
        let response = reqwest::get(component_version.download_url.clone()).await?;
        let total_size = response.content_length().unwrap_or(0);

        let mut file = std::fs::File::create(&archive_path)?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            if let Some(ref callback) = progress_callback {
                callback(downloaded, total_size);
            }
        }

        drop(file);

        // Extract to versioned directory
        let base_dir = settings.components_directory.join(component_type.name());
        let dest_dir = base_dir.join(&component_version.version);
        
        std::fs::create_dir_all(&dest_dir)?;

        let archive_path_clone = archive_path.clone();
        let dest_dir_clone = dest_dir.clone();

        tokio::task::spawn_blocking(move || {
            extract_archive(&archive_path_clone, &dest_dir_clone)
        })
        .await??;

        // Fix double-nesting: if dest_dir only contains a single directory, flatten it
        let entries: Vec<_> = std::fs::read_dir(&dest_dir)?
            .filter_map(std::result::Result::ok)
            .collect();
        
        if entries.len() == 1 && entries[0].path().is_dir() {
            let inner_dir = entries[0].path();
            let temp_dir = dest_dir.parent().unwrap().join(format!("{}_temp", component_version.version));
            
            // Move inner directory to temp location
            std::fs::rename(&inner_dir, &temp_dir)?;
            
            // Remove outer directory
            std::fs::remove_dir(&dest_dir)?;
            
            // Rename temp to correct location
            std::fs::rename(&temp_dir, &dest_dir)?;
            
            println!("   Flattened nested directory structure");
        }

        std::fs::remove_file(&archive_path)?;

        if let Some(ref callback) = progress_callback {
            callback(total_size, total_size);
        }

        println!(
            "✅ {} version {} installed to {}",
            component_type.display_name(),
            component_version.version,
            dest_dir.display()
        );

        Ok(dest_dir)
    }

    /// # Errors
    /// Returns an error if cleanup fails.
    pub fn cleanup_old_versions(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
    ) -> Result<()> {
        // Skip cleanup for UMU - it has special structure
        if component_type == ComponentType::Umu {
            return Ok(());
        }

        let base_dir = settings.components_directory.join(component_type.name());

        if !base_dir.exists() {
            return Ok(());
        }

        let latest = match self.get_latest_version(component_type) {
            Some(v) => &v.version,
            None => return Ok(()),
        };

        for entry in std::fs::read_dir(&base_dir)?.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(version) = path.file_name().and_then(|s| s.to_str())
                && version != latest {
                    println!(
                        "Removing old {} version: {}",
                        component_type.display_name(),
                        version
                    );
                    std::fs::remove_dir_all(&path)?;
                }
        }

        Ok(())
    }

    /// Download Jadeite (tweaks/anti-cheat compatibility layer)
    /// # Errors
    /// Returns an error if download fails.
    pub async fn download_jadeite(
        &self,
        settings: &GlobalSettings,
        version: Option<&str>,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> Result<PathBuf> {
        self.download_component(settings, ComponentType::Jadeite, version, progress_callback)
            .await
    }
}

fn extract_archive(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let extension = archive_path.extension().and_then(|s| s.to_str());

    match extension {
        Some("tar") => tar::Archive::new(file).unpack(dest_dir)?,
        Some("xz") => {
            let decoder = xz2::read::XzDecoder::new(file);
            tar::Archive::new(decoder).unpack(dest_dir)?;
        }
        Some("gz") => {
            let decoder = flate2::read::GzDecoder::new(file);
            tar::Archive::new(decoder).unpack(dest_dir)?;
        }
        Some("zip") => zip::ZipArchive::new(file)?.extract(dest_dir)?,
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported archive format: {extension:?}"
            ));
        }
    }

    Ok(())
}
