use anyhow::{Context, Result};
use common::HTTP_CLIENT;
use serde::Deserialize;

const COMPONENTS_BASE_URL: &str = "https://dawn.wine/elysia/components/raw/branch/main";
const INDEX_URL: &str = "https://dawn.wine/elysia/components/raw/branch/main/index.json";

#[derive(Deserialize, Debug)]
pub struct ComponentConfig {
    pub id: String,
    pub name: String,
    pub config: String,
}

#[derive(Deserialize, Debug)]
pub struct ComponentsIndex {
    pub components: ComponentsMap,
}

#[derive(Deserialize, Debug)]
pub struct ComponentsMap {
    #[serde(default)]
    pub wine: Vec<ComponentConfig>,
    #[serde(default)]
    pub proton: Vec<ComponentConfig>,
    #[serde(default)]
    pub dxvk: Vec<ComponentConfig>,
}

/// Fetch the components index from the remote repository
/// # Errors
/// Returns an error if the index cannot be fetched or parsed
pub async fn fetch_components_index() -> Result<ComponentsIndex> {
    eprintln!("[fetch_components_index] Fetching index from: {INDEX_URL}");
    
    let response = HTTP_CLIENT.get(INDEX_URL).send().await?;
    
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch components index: HTTP {}", response.status());
    }
    
    let content = response.text().await?;
    let index: ComponentsIndex = serde_json::from_str(&content)
        .context("Failed to parse components index")?;
    
    eprintln!("[fetch_components_index] Successfully fetched index");
    eprintln!("  - Wine components: {}", index.components.wine.len());
    eprintln!("  - Proton components: {}", index.components.proton.len());
    eprintln!("  - DXVK components: {}", index.components.dxvk.len());
    
    Ok(index)
}

/// Fetch a component JSON file from the remote repository
/// # Errors
/// Returns an error if the file cannot be fetched
pub async fn fetch_component_json(config_path: &str) -> Result<String> {
    // Validate the path to prevent directory traversal attacks
    if config_path.contains("..") || config_path.starts_with('/') {
        anyhow::bail!("Invalid component config path: {config_path}");
    }
    
    let url = format!("{COMPONENTS_BASE_URL}/{config_path}");
    eprintln!("[fetch_component_json] Fetching: {url}");
    
    let response = HTTP_CLIENT.get(&url).send().await?;
    
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch component JSON {}: HTTP {}", config_path, response.status());
    }
    
    let content = response.text().await
        .context(format!("Failed to read component file '{config_path}'"))?;
    
    Ok(content)
}
