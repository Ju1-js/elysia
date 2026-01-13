use crate::components::ComponentVersion;
use anyhow::Result;
use reqwest::Url;
use serde::Deserialize;

#[derive(Deserialize)]
struct DxvkEntry {
    name: String,
    title: String,
    uri: String,
}

pub async fn fetch_versions() -> Result<Vec<ComponentVersion>> {
    // Use include_str! to embed the JSON file at compile time
    let content = include_str!("dxvk/vanilla.json");
    let entries: Vec<DxvkEntry> = serde_json::from_str(content)?;

    let versions = entries
        .into_iter()
        .filter_map(|entry| {
            Some(ComponentVersion {
                version: entry.name.clone(), // Use name for folder structure
                download_url: Url::parse(&entry.uri).ok()?,
                display_name: entry.title.clone(),
            })
        })
        .collect();

    Ok(versions)
}
