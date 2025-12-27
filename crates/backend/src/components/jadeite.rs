use anyhow::Result;
use reqwest::Url;
use crate::components::ComponentVersion;
use common::git;

pub async fn fetch_versions() -> Result<Vec<ComponentVersion>> {
    let repo = "mkrsym1/jadeite";
    let releases = git::codeberg_releases(repo).await?;
    
    let versions = releases
        .into_iter()
        .filter_map(|rel| {
            rel.assets
                .iter()
                .find(|asset| asset.name == format!("{}.zip", &rel.tag_name))
                .and_then(|asset| {
                    Some(ComponentVersion {
                        version: rel.tag_name.clone(),
                        download_url: Url::parse(&asset.browser_download_url).ok()?,
                    })
                })
        })
        .collect();
    
    Ok(versions)
}
