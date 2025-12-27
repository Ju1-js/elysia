use anyhow::{Result, Context};
use reqwest::Client;
use crate::{HTTP_CLIENT, git::proto::ReleasesResponse};

pub async fn github_releases(repo: &str) -> Result<ReleasesResponse> {
    let url = format!("https://api.github.com/repos/{}/releases", repo);
    
    let response = HTTP_CLIENT
        .get(&url)
        .send()
        .await?;
    
    let status = response.status();
    
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("[GITHUB] Error for {}: {} - {}", repo, status, error_text);
        return Err(anyhow::anyhow!("GitHub API error {}: {}", status, error_text));
    }
    
    let text = response.text().await?;
    
    let releases = serde_json::from_str::<ReleasesResponse>(&text)
        .context(format!("Failed to parse GitHub releases for {}", repo))?;
    
    Ok(releases)
}

pub async fn codeberg_releases(repo: &str) -> Result<ReleasesResponse> {
    let url = format!("https://codeberg.org/api/v1/repos/{}/releases", repo);
    
    let response = HTTP_CLIENT
        .get(&url)
        .send()
        .await?;
    
    let status = response.status();
    
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("[CODEBERG] Error for {}: {} - {}", repo, status, error_text);
        return Err(anyhow::anyhow!("Codeberg API error {}: {}", status, error_text));
    }
    
    let text = response.text().await?;
    
    let releases = serde_json::from_str::<ReleasesResponse>(&text)
        .context(format!("Failed to parse Codeberg releases for {}", repo))?;
    
    Ok(releases)
}
