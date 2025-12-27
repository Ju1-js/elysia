use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

use crate::settings::GlobalSettings;

pub const STEAMRT_VERSION: &str = "3.0.20251216.191774";
const STEAMRT_BASE_URL: &str = "https://repo.steampowered.com/steamrt3/images";
const STEAMRT_TARBALL: &str = "SteamLinuxRuntime_sniper.tar.xz";

#[derive(Debug)]
pub struct SteamRtSetup {
    pub runtime_path: Option<PathBuf>,
    pub version: String,
    pub needs_download: bool,
}

fn is_steamrt_complete(version_dir: &Path) -> bool {
    version_dir.join(".elysia_steamrt_installed").exists()
}

pub async fn prepare_steamrt(settings: &GlobalSettings) -> Result<SteamRtSetup> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    let version_dir = steamrt_dir.join(STEAMRT_VERSION);
    
    let is_complete = version_dir.exists() && is_steamrt_complete(&version_dir);
    
    let needs_download = !is_complete;

    let runtime_path = if is_complete {
        Some(version_dir)
    } else {
        None
    };

    Ok(SteamRtSetup {
        runtime_path,
        version: STEAMRT_VERSION.to_string(),
        needs_download,
    })
}

pub fn get_download_url() -> String {
    format!("{}/{}/{}", STEAMRT_BASE_URL, STEAMRT_VERSION, STEAMRT_TARBALL)
}

pub fn get_checksum_url() -> String {
    format!("{}/{}/SHA256SUMS", STEAMRT_BASE_URL, STEAMRT_VERSION)
}

pub async fn download_steamrt(
    settings: &GlobalSettings,
    progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
) -> Result<PathBuf> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    let version_dir = steamrt_dir.join(STEAMRT_VERSION);
    
    if version_dir.exists() && is_steamrt_complete(&version_dir) {
        println!("Steam Runtime {} already exists", STEAMRT_VERSION);
        return Ok(version_dir);
    }

    println!("Downloading Steam Runtime {}...", STEAMRT_VERSION);
    
    fs::create_dir_all(&steamrt_dir)?;
    
    let download_url = get_download_url();
    let tarball_path = steamrt_dir.join(STEAMRT_TARBALL);
    let resume_marker = steamrt_dir.join(".steamrt_download_progress");
    
    let start_byte = if tarball_path.exists() && resume_marker.exists() {
        match fs::metadata(&tarball_path) {
            Ok(metadata) => {
                let size = metadata.len();
                println!("Resuming Steam Runtime download from {} bytes", size);
                size
            }
            Err(_) => 0,
        }
    } else {
        let _ = fs::remove_file(&tarball_path);
        let _ = fs::remove_file(&resume_marker);
        0
    };

    let client = reqwest::Client::new();
    let mut request = client.get(&download_url);
    
    if start_byte > 0 {
        request = request.header("Range", format!("bytes={}-", start_byte));
    }
    
    let response = request.send().await
        .context("Failed to download Steam Runtime")?;
    
    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
    }
    
    let total_size = if start_byte > 0 {
        response.content_length().unwrap_or(0) + start_byte
    } else {
        response.content_length().unwrap_or(0)
    };
    
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&tarball_path)?;
    
    fs::write(&resume_marker, "")?;
    
    let mut downloaded = start_byte;
    let mut stream = response.bytes_stream();
    let mut last_update = Instant::now();
    
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        
        let now = Instant::now();
        if now.duration_since(last_update) >= Duration::from_millis(100) {
            if let Some(ref callback) = progress_callback {
                callback(downloaded, total_size);
            }
            last_update = now;
        }
    }
    
    if let Some(ref callback) = progress_callback {
        callback(downloaded, total_size);
    }
    
    drop(file);
    
    let _ = fs::remove_file(&resume_marker);
    
    println!("Download complete. Extracting...");
    
    if version_dir.exists() {
        let _ = fs::remove_dir_all(&version_dir);
    }
    
    let tarball_clone = tarball_path.clone();
    let version_clone = version_dir.clone();
    tokio::task::spawn_blocking(move || {
        extract_tarball(&tarball_clone, &version_clone)
    }).await??;
    
    let marker_path = version_dir.join(".elysia_steamrt_installed");
    fs::write(&marker_path, STEAMRT_VERSION)?;
    
    fs::remove_file(&tarball_path)?;
    
    if let Some(ref callback) = progress_callback {
        callback(total_size, total_size);
    }
    
    println!("Steam Runtime {} installed to {:?}", STEAMRT_VERSION, version_dir);
    
    Ok(version_dir)
}

fn extract_tarball(tarball_path: &Path, dest_dir: &Path) -> Result<()> {
    fs::create_dir_all(dest_dir)?;
    
    let file = fs::File::open(tarball_path)?;
    let decompressor = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressor);
    
    archive.unpack(dest_dir)?;
    
    Ok(())
}

pub async fn verify_checksum(settings: &GlobalSettings) -> Result<bool> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    let tarball_path = steamrt_dir.join(STEAMRT_TARBALL);
    
    if !tarball_path.exists() {
        return Ok(false);
    }

    let checksum_url = get_checksum_url();
    let checksums = reqwest::get(&checksum_url)
        .await?
        .text()
        .await?;
    
    let expected_hash = checksums
        .lines()
        .find(|line| line.contains(STEAMRT_TARBALL))
        .and_then(|line| line.split_whitespace().next())
        .context("Checksum not found in SHA256SUMS")?;
    
    use sha2::{Sha256, Digest};
    let mut file = fs::File::open(&tarball_path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let actual_hash = format!("{:x}", hasher.finalize());
    
    Ok(actual_hash == expected_hash)
}

pub fn cleanup_old_versions(settings: &GlobalSettings) -> Result<()> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    
    if !steamrt_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&steamrt_dir)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy();
                if name.starts_with("3.0.") && name != STEAMRT_VERSION {
                    println!("Removing old Steam Runtime version: {}", name);
                    fs::remove_dir_all(&path)?;
                }
            }
        }
    }

    Ok(())
}
