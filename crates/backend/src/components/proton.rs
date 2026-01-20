use crate::components::{ComponentVersion, repository};
use anyhow::Result;
use reqwest::Url;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct ProtonEntry {
    name: String,
    title: String,
    uri: String,
}

pub async fn fetch_versions(_components_dir: &PathBuf) -> Result<Vec<ComponentVersion>> {
    // Fetch the components index
    let index = repository::fetch_components_index().await?;
    
    if index.components.proton.is_empty() {
        eprintln!("[Proton] No component configs found in index");
        return Ok(Vec::new());
    }
    
    let mut all_versions = Vec::new();
    
    // Fetch and parse each component JSON file listed in the index
    for component_config in index.components.proton {
        eprintln!("[Proton] Loading component: {} from {}", component_config.name, component_config.config);
        
        match repository::fetch_component_json(&component_config.config).await {
            Ok(content) => {
                match serde_json::from_str::<Vec<ProtonEntry>>(&content) {
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
                        eprintln!("[Proton] Loaded {} versions from {}", versions.len(), component_config.name);
                        all_versions.extend(versions);
                    }
                    Err(err) => {
                        eprintln!("[Proton] Failed to parse JSON from {}: {err}", component_config.config);
                    }
                }
            }
            Err(err) => {
                eprintln!("[Proton] Failed to fetch {}: {err}", component_config.config);
            }
        }
    }

    // Sort versions to prioritize dwproton first, then by source name
    all_versions.sort_by(|a, b| {
        let a_source = a.source.as_deref().unwrap_or("");
        let b_source = b.source.as_deref().unwrap_or("");
        
        // dwproton should come first
        let a_is_dwproton = a_source.eq_ignore_ascii_case("dwproton");
        let b_is_dwproton = b_source.eq_ignore_ascii_case("dwproton");
        
        match (a_is_dwproton, b_is_dwproton) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a_source.cmp(b_source),
        }
    });

    Ok(all_versions)
}
