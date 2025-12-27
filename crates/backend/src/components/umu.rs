use anyhow::Result;
use reqwest::Url;
use crate::components::ComponentVersion;
use common::git;

pub async fn fetch_versions() -> Result<Vec<ComponentVersion>> {
    let repo = "Open-Wine-Components/umu-launcher";
    let releases = git::github_releases(repo).await?;
    
    let versions = releases
        .into_iter()
        .filter_map(|rel| {
            let version = &rel.tag_name;
            rel.assets
                .iter()
                .find(|asset| asset.name == format!("umu-launcher-{}-zipapp.tar", version))
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
