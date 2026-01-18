use anyhow::{Context, Result};
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

use crate::components::{ComponentVersion, repository};
use crate::settings::GlobalSettings;

// Legacy constants for backward compatibility with existing download functions
// These are used by prepare_steamrt, download_steamrt, and related legacy functions
// New code should use fetch_versions() to get current version from repository
pub const STEAMRT_VERSION: &str = "3.0.20251216.191774";
const STEAMRT_BASE_URL: &str = "https://repo.steampowered.com/steamrt3/images";
const STEAMRT_TARBALL: &str = "SteamLinuxRuntime_sniper.tar.xz";

#[derive(Deserialize)]
struct SteamRtEntry {
    name: String,
    version: String,
    url: String,
}

/// # Errors
/// Returns an error if fetching or parsing component versions fails.
pub async fn fetch_versions(_components_dir: &PathBuf) -> Result<Vec<ComponentVersion>> {
    // Fetch the components index
    let index = repository::fetch_components_index().await?;
    
    if index.components.runtime.is_empty() {
        eprintln!("[SteamRT] No runtime configs found in index");
        return Ok(Vec::new());
    }
    
    let mut all_versions = Vec::new();
    
    // Fetch and parse each runtime component JSON file listed in the index
    for component_config in index.components.runtime {
        // Only process steamrt entries
        if component_config.id != "steamrt" {
            continue;
        }
        
        eprintln!("[SteamRT] Loading component: {} from {}", component_config.name, component_config.config);
        
        match repository::fetch_component_json(&component_config.config).await {
            Ok(content) => {
                match serde_json::from_str::<SteamRtEntry>(&content) {
                    Ok(entry) => {
                        // Construct the full download URL
                        let download_url = format!("{}/{}/{}", entry.url, entry.version, entry.name);
                        
                        if let Ok(url) = Url::parse(&download_url) {
                            let version = ComponentVersion {
                                version: entry.version.clone(),
                                download_url: url,
                                display_name: format!("Steam Runtime {}", entry.version),
                                source: Some(component_config.id.clone()),
                            };
                            eprintln!("[SteamRT] Loaded version: {}", entry.version);
                            all_versions.push(version);
                        } else {
                            eprintln!("[SteamRT] Failed to parse URL: {download_url}");
                        }
                    }
                    Err(err) => {
                        eprintln!("[SteamRT] Failed to parse JSON from {}: {err}", component_config.config);
                    }
                }
            }
            Err(err) => {
                eprintln!("[SteamRT] Failed to fetch {}: {err}", component_config.config);
            }
        }
    }

    Ok(all_versions)
}

#[derive(Debug)]
pub struct SteamRtSetup {
    pub runtime_path: Option<PathBuf>,
    pub version: String,
    pub needs_download: bool,
}

fn is_steamrt_complete(version_dir: &Path) -> bool {
    version_dir.join(".elysia_steamrt_installed").exists()
}

/// # Errors
/// Returns an error if the Steam Runtime setup cannot be prepared.
#[allow(clippy::unused_async)]
pub async fn prepare_steamrt(settings: &GlobalSettings) -> Result<SteamRtSetup> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    let version_dir = steamrt_dir.join(STEAMRT_VERSION);

    let is_complete = version_dir.exists() && is_steamrt_complete(&version_dir);

    // Clean up interrupted download markers if installation is incomplete
    if !is_complete {
        let tarball_path = steamrt_dir.join(STEAMRT_TARBALL);
        let resume_marker = steamrt_dir.join(".steamrt_download_progress");

        // If we have a partial download but no complete installation, clean up
        if resume_marker.exists() {
            println!("Detected interrupted Steam Runtime download, cleaning up stale markers...");
            let _ = fs::remove_file(&resume_marker);
            // Also remove partial tarball to ensure fresh download
            if tarball_path.exists() {
                let _ = fs::remove_file(&tarball_path);
            }
        }
    }

    let needs_download = !is_complete;

    let runtime_path = if is_complete { Some(version_dir) } else { None };

    Ok(SteamRtSetup {
        runtime_path,
        version: STEAMRT_VERSION.to_string(),
        needs_download,
    })
}

#[must_use] 
pub fn get_download_url() -> String {
    format!(
        "{STEAMRT_BASE_URL}/{STEAMRT_VERSION}/{STEAMRT_TARBALL}"
    )
}

#[must_use] 
pub fn get_checksum_url() -> String {
    format!("{STEAMRT_BASE_URL}/{STEAMRT_VERSION}/SHA256SUMS")
}

/// # Errors
/// Returns an error if the download or extraction fails.
#[allow(clippy::too_many_lines)]
pub async fn download_steamrt(
    settings: &GlobalSettings,
    progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
) -> Result<PathBuf> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    let version_dir = steamrt_dir.join(STEAMRT_VERSION);

    if version_dir.exists() && is_steamrt_complete(&version_dir) {
        println!("Steam Runtime {STEAMRT_VERSION} already exists");
        return Ok(version_dir);
    }

    // Clean up incomplete installation
    if version_dir.exists() && !is_steamrt_complete(&version_dir) {
        println!("Removing incomplete Steam Runtime installation...");
        let _ = fs::remove_dir_all(&version_dir);
    }

    println!("Downloading Steam Runtime {STEAMRT_VERSION}...");

    fs::create_dir_all(&steamrt_dir)?;

    let download_url = get_download_url();
    let tarball_path = steamrt_dir.join(STEAMRT_TARBALL);
    let resume_marker = steamrt_dir.join(".steamrt_download_progress");

    let start_byte = if tarball_path.exists() && resume_marker.exists() {
        match fs::metadata(&tarball_path) {
            Ok(metadata) => {
                let size = metadata.len();
                println!("Resuming Steam Runtime download from {size} bytes");
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
        request = request.header("Range", format!("bytes={start_byte}-"));
    }

    let response = request
        .send()
        .await
        .context("Failed to download Steam Runtime")?;

    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
    {
        return Err(anyhow::anyhow!(
            "Download failed with status: {}",
            response.status()
        ));
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
    let extraction_result =
        tokio::task::spawn_blocking(move || extract_tarball(&tarball_clone, &version_clone))
            .await?;

    // If extraction fails, clean up the incomplete installation
    if let Err(e) = extraction_result {
        eprintln!(
            "Steam Runtime extraction failed: {e}. Cleaning up partial installation and tarball..."
        );
        let _ = fs::remove_dir_all(&version_dir);
        let _ = fs::remove_file(&tarball_path);
        return Err(e.context("Failed to extract Steam Runtime. The partial download has been cleaned up. Please try downloading again."));
    }

    let marker_path = version_dir.join(".elysia_steamrt_installed");
    fs::write(&marker_path, STEAMRT_VERSION)?;

    fs::remove_file(&tarball_path)?;

    if let Some(ref callback) = progress_callback {
        callback(total_size, total_size);
    }

    println!(
        "Steam Runtime {STEAMRT_VERSION} installed to {}", version_dir.display()
    );

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

/// # Errors
/// Returns an error if checksum verification fails.
pub async fn verify_checksum(settings: &GlobalSettings) -> Result<bool> {
    let steamrt_dir = settings.components_directory.join("steamrt");
    let tarball_path = steamrt_dir.join(STEAMRT_TARBALL);

    if !tarball_path.exists() {
        return Ok(false);
    }

    let checksum_url = get_checksum_url();
    let checksums = reqwest::get(&checksum_url).await?.text().await?;

    let expected_hash = checksums
        .lines()
        .find(|line| line.contains(STEAMRT_TARBALL))
        .and_then(|line| line.split_whitespace().next())
        .context("Checksum not found in SHA256SUMS")?;

    let mut file = fs::File::open(&tarball_path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let actual_hash = format!("{:x}", hasher.finalize());

    Ok(actual_hash == expected_hash)
}

/// # Errors
/// Returns an error if cleanup fails.
pub fn cleanup_old_versions(settings: &GlobalSettings) -> Result<()> {
    let steamrt_dir = settings.components_directory.join("steamrt");

    if !steamrt_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&steamrt_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir()
            && let Some(name) = path.file_name() {
                let name = name.to_string_lossy();
                if name.starts_with("3.0.") && name != STEAMRT_VERSION {
                    println!("Removing old Steam Runtime version: {name}");
                    fs::remove_dir_all(&path)?;
                }
            }
    }

    Ok(())
}
