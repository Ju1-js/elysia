use crate::components::{ComponentVersion, repository};
use anyhow::Result;
use reqwest::Url;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct DxvkEntry {
    name: String,
    title: String,
    uri: String,
}

pub async fn fetch_versions(_components_dir: &PathBuf) -> Result<Vec<ComponentVersion>> {
    // Fetch the components index
    let index = repository::fetch_components_index().await?;
    
    if index.components.dxvk.is_empty() {
        eprintln!("[DXVK] No component configs found in index");
        return Ok(Vec::new());
    }
    
    let mut all_versions = Vec::new();
    
    // Fetch and parse each component JSON file listed in the index
    for component_config in index.components.dxvk {
        eprintln!("[DXVK] Loading component: {} from {}", component_config.name, component_config.config);
        
        match repository::fetch_component_json(&component_config.config).await {
            Ok(content) => {
                match serde_json::from_str::<Vec<DxvkEntry>>(&content) {
                    Ok(entries) => {
                        let versions: Vec<ComponentVersion> = entries
                            .into_iter()
                            .filter_map(|entry| {
                                Some(ComponentVersion {
                                    version: entry.name.clone(),
                                    download_url: Url::parse(&entry.uri).ok()?,
                                    display_name: entry.title.clone(),
                                    source: Some(component_config.id.clone()),
                                })
                            })
                            .collect();
                        eprintln!("[DXVK] Loaded {} versions from {}", versions.len(), component_config.name);
                        all_versions.extend(versions);
                    }
                    Err(err) => {
                        eprintln!("[DXVK] Failed to parse JSON from {}: {err}", component_config.config);
                    }
                }
            }
            Err(err) => {
                eprintln!("[DXVK] Failed to fetch {}: {err}", component_config.config);
            }
        }
    }

    Ok(all_versions)
}
