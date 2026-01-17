use crate::components::ComponentVersion;
use anyhow::Result;
use common::HTTP_CLIENT;
use reqwest::Url;
use serde::Deserialize;

#[derive(Deserialize)]
struct JadeiteMetadata {
    jadeite: JadeiteInfo,
}

#[derive(Deserialize)]
struct JadeiteInfo {
    version: String,
}

pub async fn fetch_versions() -> Result<Vec<ComponentVersion>> {
    let metadata_url = "https://codeberg.org/mkrsym1/jadeite/raw/branch/master/metadata.json";
    
    let response = HTTP_CLIENT.get(metadata_url).send().await?;
    let metadata: JadeiteMetadata = response.json().await?;
    
    let version = metadata.jadeite.version;
    
    // Construct the download URL based on the version
    let version_tag = format!("v{version}");
    let download_url = format!(
        "https://codeberg.org/mkrsym1/jadeite/releases/download/{version_tag}/{version_tag}.zip"
    );
    
    let component_version = ComponentVersion {
        version: version.clone(),
        download_url: Url::parse(&download_url)?,
        display_name: version,
    };

    Ok(vec![component_version])
}
