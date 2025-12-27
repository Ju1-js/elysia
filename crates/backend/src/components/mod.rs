use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use std::io::Write;

mod dxvk;
mod jadeite;
mod umu;
pub mod installer; 
pub mod steamrt;
pub mod tweaks;

pub use installer::{ComponentRequirement, install_components, install_proton_runtime, install_tweaks};

use crate::settings::GlobalSettings;

#[derive(Serialize, PartialEq, Eq, Hash, Deserialize, Debug, Clone, Copy)]
pub enum ComponentType {
    Dxvk,
    Jadeite,
    Umu,
    SteamRuntime,
}

impl ComponentType {
    fn name(&self) -> &'static str {
        match self {
            ComponentType::Dxvk => "dxvk",
            ComponentType::Jadeite => "jadeite",
            ComponentType::Umu => "umu",
            ComponentType::SteamRuntime => "steamrt",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            ComponentType::Dxvk => "DXVK",
            ComponentType::Jadeite => "Jadeite",
            ComponentType::Umu => "UMU Launcher",
            ComponentType::SteamRuntime => "Steam Runtime",
        }
    }

    async fn fetch_versions(&self) -> Result<Vec<ComponentVersion>> {
        match self {
            ComponentType::Dxvk => dxvk::fetch_versions().await,
            ComponentType::Jadeite => jadeite::fetch_versions().await,
            ComponentType::Umu => umu::fetch_versions().await,
            ComponentType::SteamRuntime => Ok(vec![]),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ComponentVersion {
    pub version: String,
    pub download_url: Url,
}

#[derive(Serialize, Default, Deserialize)]
pub struct VersionCache {
    pub entries: HashMap<ComponentType, Vec<ComponentVersion>>,
}

pub struct ComponentManager {
    pub cache: VersionCache,
}

impl ComponentManager {
    pub async fn new() -> Self {
        Self {
            cache: VersionCache::default(),
        }
    }

    pub async fn refresh_index(&mut self) -> Result<()> {
        let types = [ComponentType::Dxvk, ComponentType::Jadeite, ComponentType::Umu];
        let handles = types.iter().map(|&t| {
            tokio::spawn(async move { (t, t.fetch_versions().await) })
        }).collect::<Vec<_>>();

        for handle in handles {
            let (component_type, versions) = handle.await?;
            match versions {
                Ok(versions) => {
                    self.cache.entries.insert(component_type, versions);
                }
                Err(err) => {
                    eprintln!("Failed to fetch versions for {:?}: {}", component_type, err);
                }
            }
        }
        Ok(())
    }

    pub async fn refresh_component(&mut self, component_type: ComponentType) -> Result<()> {
        let versions = component_type.fetch_versions().await?;
        self.cache.entries.insert(component_type, versions);
        Ok(())
    }

    pub fn is_installed(&self, settings: &GlobalSettings, component_type: ComponentType) -> bool {
        let base_dir = settings.components_directory.join(component_type.name());
        
        if !base_dir.exists() {
            return false;
        }

        std::fs::read_dir(&base_dir)
            .ok()
            .and_then(|mut entries| entries.find(|e| e.as_ref().ok().map_or(false, |e| e.path().is_dir())))
            .is_some()
    }

    pub fn get_installed_version(&self, settings: &GlobalSettings, component_type: ComponentType) -> Option<String> {
        let base_dir = settings.components_directory.join(component_type.name());
        
        std::fs::read_dir(&base_dir).ok()?
            .flatten()
            .find(|e| e.path().is_dir())
            .and_then(|e| e.file_name().to_str().map(String::from))
    }

    pub fn get_latest_version(&self, component_type: ComponentType) -> Option<&ComponentVersion> {
        self.cache.entries.get(&component_type)?.first()
    }

    pub fn needs_update(&self, settings: &GlobalSettings, component_type: ComponentType) -> bool {
        let installed = match self.get_installed_version(settings, component_type) {
            Some(v) => v,
            None => return false,
        };
        
        let latest = match self.get_latest_version(component_type) {
            Some(v) => &v.version,
            None => return false,
        };
        
        installed != *latest
    }

    pub async fn download_component(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
        version: Option<&str>,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> Result<PathBuf> {
        let component_version = if let Some(ver) = version {
            self.cache.entries
                .get(&component_type)
                .and_then(|versions| versions.iter().find(|v| v.version == ver))
                .ok_or_else(|| anyhow::anyhow!("Version {} not found", ver))?
        } else {
            self.get_latest_version(component_type)
                .ok_or_else(|| anyhow::anyhow!("No versions available for {:?}", component_type))?
        };

        println!("Downloading {} version {}...", component_type.display_name(), component_version.version);

        let temp_dir = std::env::temp_dir();
        let filename = component_version.download_url
            .path_segments()
            .and_then(|segments| segments.last())
            .ok_or_else(|| anyhow::anyhow!("Invalid download URL"))?;
        let archive_path = temp_dir.join(filename);

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

        println!("Download complete. Extracting...");

        let dest_dir = settings.components_directory
            .join(component_type.name())
            .join(&component_version.version);
        
        let archive_path_clone = archive_path.clone();
        let dest_dir_clone = dest_dir.clone();
        
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&dest_dir_clone)?;
            extract_archive(&archive_path_clone, &dest_dir_clone)
        }).await??;

        std::fs::remove_file(&archive_path)?;

        if let Some(ref callback) = progress_callback {
            callback(total_size, total_size);
        }

        println!("{} version {} installed to {:?}", component_type.display_name(), component_version.version, dest_dir);

        Ok(dest_dir)
    }

    pub fn cleanup_old_versions(&self, settings: &GlobalSettings, component_type: ComponentType) -> Result<()> {
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
            if path.is_dir() {
                if let Some(version) = path.file_name().and_then(|s| s.to_str()) {
                    if version != latest {
                        println!("Removing old {} version: {}", component_type.display_name(), version);
                        std::fs::remove_dir_all(&path)?;
                    }
                }
            }
        }

        Ok(())
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
        _ => return Err(anyhow::anyhow!("Unsupported archive format: {:?}", extension)),
    }
    
    Ok(())
}
