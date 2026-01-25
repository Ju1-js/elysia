use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};

use super::api;

const APP_CODE: &str = "YDUTE5gscDZ229CW";
const DATA_FOLDER: &str = "Endfield_Data";

#[derive(Debug, Clone)]
pub struct Game {
    path: PathBuf,
}

impl Game {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.path
            .join(DATA_FOLDER)
            .join("globalgamemanagers")
            .exists()
    }

    /// Get latest version from API
    /// 
    /// # Errors
    /// Returns an error if the API request fails or version is not found in response.
    pub async fn get_latest_version() -> Result<String> {
        let body = serde_json::json!({
            "proxy_reqs": [{
                "kind": "get_latest_game",
                "get_latest_game_req": {
                    "appcode": APP_CODE,
                    "channel": "6",
                    "sub_channel": "6",
                    "version": ""
                }
            }]
        });

        let resp_json = super::batch_proxy_post(&body).await
            .map_err(|e| anyhow::anyhow!("Failed to get latest version: {e}"))?;

        let typed: api::BatchProxyResponse = serde_json::from_value(resp_json)
            .context("Failed to deserialize batch response")?;

        for proxy in typed.proxy_rsps {
            if let Some(get_latest) = proxy.get_latest_game_rsp
                && let Some(version) = get_latest.version {
                return Ok(version);
            }
        }

        Err(anyhow::anyhow!("No version found in API response"))
    }

    /// Get installed game version
    /// 
    /// Reads from .version file if available, otherwise parses globalgamemanagers binary.
    /// 
    /// # Errors
    /// Returns an error if version cannot be determined from either source.
    pub fn get_version(&self) -> Result<String> {
        let version_file = self.path.join(".version");
        if let Ok(stored_version) = std::fs::read_to_string(&version_file) {
            let trimmed = stored_version.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        let path = self.path.join(DATA_FOLDER).join("globalgamemanagers");
        let mut file = File::open(&path)
            .with_context(|| format!("Failed to open {}", path.display()))?;

        file.seek(SeekFrom::Start(4000))?;
        
        let mut buffer = vec![0u8; 10000];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        let mut version_parts: Vec<String> = Vec::new();
        let mut current_num = String::new();
        
        for byte in buffer {
            match byte {
                b'0'..=b'9' => {
                    current_num.push(byte as char);
                }
                b'.' => {
                    if current_num.is_empty() {
                        version_parts.clear();
                    } else {
                        version_parts.push(current_num.clone());
                        current_num.clear();
                    }
                }
                0 | b'_' => {
                    if !current_num.is_empty() {
                        version_parts.push(current_num.clone());
                    }
                    
                    if version_parts.len() == 3 {
                        let version = version_parts.join(".");
                        eprintln!("[INFO] Found version in globalgamemanagers: {version}");
                        return Ok(version);
                    }
                    
                    version_parts.clear();
                    current_num.clear();
                }
                _ => {
                    version_parts.clear();
                    current_num.clear();
                }
            }
        }

        Err(anyhow::anyhow!(
            "Version string not found in globalgamemanagers. The game may not be installed correctly."
        ))
    }

    /// Check if an update is available
    /// 
    /// # Errors
    /// Returns an error if the API request fails.
    pub async fn needs_update(&self) -> Result<bool> {
        if !self.is_installed() {
            return Ok(true);
        }

        let Ok(current) = self.get_version() else {
            return Ok(true);
        };

        let latest = Self::get_latest_version().await?;

        Ok(current != latest)
    }
}
