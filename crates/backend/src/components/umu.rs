use anyhow::Result;
use async_trait::async_trait;
use reqwest::Url;

use crate::components::{Component, ComponentVersion};

pub struct Umu {}

#[async_trait]
impl Component for Umu {
    fn name(&self) -> &'static str {
        "umu"
    }

    fn display_name(&self) -> &'static str {
        "UMU Launcher"
    }

    async fn fetch_versions(&self) -> Result<Vec<ComponentVersion>> {
        let repo = "Open-Wine-Components/umu-launcher";
        let releases = common::git::github_releases(repo).await?;

        let versions = releases.into_iter().filter_map(|rel| {
            let version = &rel.tag_name;
            
            rel.assets
                .iter()
                .filter_map(|asset| {
                    if asset.name == format!("umu-launcher-{}-zipapp.tar", version) {
                        Some(ComponentVersion {
                            version: rel.tag_name.clone(),
                            download_url: Url::parse(&asset.browser_download_url).ok()?,
                        })
                    } else {
                        None
                    }
                })
                .next()
        });

        Ok(versions.collect())
    }
}