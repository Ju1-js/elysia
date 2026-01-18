use crate::components::{ComponentVersion, repository};
use anyhow::Result;
use reqwest::Url;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct UmuEntry {
    version: String,
    url: String,
}

/// # Errors
/// Returns an error if fetching or parsing component versions fails.
pub async fn fetch_versions(_components_dir: &PathBuf) -> Result<Vec<ComponentVersion>> {
    // Fetch the components index
    let index = repository::fetch_components_index().await?;
    
    if index.components.runtime.is_empty() {
        eprintln!("[UMU] No runtime configs found in index");
        return Ok(Vec::new());
    }
    
    let mut all_versions = Vec::new();
    
    // Fetch and parse each runtime component JSON file listed in the index
    for component_config in index.components.runtime {
        // Only process umu-elysia entries
        if component_config.id != "umu-elysia" {
            continue;
        }
        
        eprintln!("[UMU] Loading component: {} from {}", component_config.name, component_config.config);
        
        match repository::fetch_component_json(&component_config.config).await {
            Ok(content) => {
                match serde_json::from_str::<UmuEntry>(&content) {
                    Ok(entry) => {
                        if let Ok(url) = Url::parse(&entry.url) {
                            let version = ComponentVersion {
                                version: entry.version.clone(),
                                download_url: url,
                                display_name: format!("UMU Launcher {}", entry.version),
                                source: Some(component_config.id.clone()),
                            };
                            eprintln!("[UMU] Loaded version: {}", entry.version);
                            all_versions.push(version);
                        } else {
                            eprintln!("[UMU] Failed to parse URL: {}", entry.url);
                        }
                    }
                    Err(err) => {
                        eprintln!("[UMU] Failed to parse JSON from {}: {err}", component_config.config);
                    }
                }
            }
            Err(err) => {
                eprintln!("[UMU] Failed to fetch {}: {err}", component_config.config);
            }
        }
    }

    Ok(all_versions)
}
