use std::path::Path;
use anyhow::Result;
use serde_json::json;

use super::api::{self, BatchProxyResponse};
use super::download;
use super::game::Game;

const APP_CODE: &str = "YDUTE5gscDZ229CW";

pub struct Repairer {
    game: Game,
    #[allow(dead_code)]
    temp_dir: std::path::PathBuf,
}

impl Repairer {
    #[must_use]
    pub fn new(game: Game, temp_dir: std::path::PathBuf) -> Self {
        Self { game, temp_dir }
    }

    /// Update the game to the latest version
    /// Attempts to use patch update if available, falls back to full download
    /// # Errors
    /// Returns an error if update fails.
    #[allow(clippy::cast_precision_loss)]
    pub async fn update(&self, progress_key: &str) -> Result<(), String> {
        eprintln!("[INFO] Starting Endfield update");

        // Get current version to request patch
        let current_version = self.game.get_version().unwrap_or_default();
        
        eprintln!("[INFO] Current version: {current_version}");

        let body = json!({
            "proxy_reqs": [{
                "kind": "get_latest_game",
                "get_latest_game_req": {
                    "appcode": APP_CODE,
                    "channel": "6",
                    "sub_channel": "6",
                    "version": current_version
                }
            }]
        });

        let resp_json = super::batch_proxy_post(&body).await?;
        let typed: BatchProxyResponse = serde_json::from_value(resp_json)
            .map_err(|e| format!("Failed to deserialize batch response: {e}"))?;

        for proxy in typed.proxy_rsps {
            if let Some(get_latest) = proxy.get_latest_game_rsp {
                let latest_version = get_latest.version.clone();

                if let Some(patch) = get_latest.patch
                    && !patch.patches.is_empty() {
                        let patch_packs: Vec<api::Pack> = patch
                            .patches
                            .into_iter()
                            .filter(|p| !p.url.is_empty())
                            .map(std::convert::Into::into)
                            .collect();

                        if !patch_packs.is_empty() {
                            let patch_size: u64 = patch_packs
                                .iter()
                                .filter_map(|p| p.package_size.as_ref())
                                .filter_map(|s| s.parse::<u64>().ok())
                                .sum();

                            eprintln!("[INFO] Using differential patch update");
                            eprintln!("[INFO]   Patch size: {:.2} MB", patch_size as f64 / 1024.0 / 1024.0);
                            eprintln!("[INFO]   Parts: {}", patch_packs.len());
                            eprintln!("[INFO]   Destination: {}", self.game.path().display());

                            download::download_and_extract_streaming(
                                patch_packs,
                                self.game.path(),
                                progress_key,
                                APP_CODE,
                            )
                            .await?;

                            // Write .version file
                            if let Some(version_str) = latest_version {
                                std::fs::write(self.game.path().join(".version"), &version_str)
                                    .map_err(|e| format!("Failed to write .version file: {e}"))?;
                                eprintln!("[INFO] Updated to version: {version_str}");
                            }

                            eprintln!("[INFO] Patch update complete!");
                            return Ok(());
                        }
                    }

                // fallback to full downloads (?) hopefully :xdd:
                if let Some(pkg) = get_latest.pkg {
                    let packs: Vec<api::Pack> = pkg
                        .packs
                        .into_iter()
                        .filter(|p| !p.url.is_empty())
                        .collect();

                    if packs.is_empty() {
                        return Err("No pack URLs found in batch response".to_string());
                    }

                    let full_size: u64 = packs
                        .iter()
                        .filter_map(|p| p.package_size.as_ref())
                        .filter_map(|s| s.parse::<u64>().ok())
                        .sum();

                    eprintln!("[WARN] No patch available, using full download");
                    eprintln!("[INFO]   Full size: {:.2} GB", full_size as f64 / 1024.0 / 1024.0 / 1024.0);
                    eprintln!("[INFO]   Parts: {}", packs.len());
                    eprintln!("[INFO]   Destination: {}", self.game.path().display());

                    download::download_and_extract_streaming(
                        packs,
                        self.game.path(),
                        progress_key,
                        APP_CODE,
                    )
                    .await?;

                    // Write .version file
                    if let Some(version_str) = latest_version {
                        std::fs::write(self.game.path().join(".version"), &version_str)
                            .map_err(|e| format!("Failed to write .version file: {e}"))?;
                        eprintln!("[INFO] Updated to version: {version_str}");
                    }

                    eprintln!("[INFO] Full update complete!");
                    return Ok(());
                }
            }
        }

        Err("No get_latest_game response found".to_string())
    }
}
