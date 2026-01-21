use anyhow::Result;

const ANNOUNCEMENT_URL: &str = "https://dawn.wine/elysia/components/raw/branch/main/announcements.txt";

/// Fetches the current announcement text from the remote server
///
/// # Errors
///
/// Returns an error if the HTTP request fails or if the response cannot be parsed as text
pub async fn fetch_announcement() -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get(ANNOUNCEMENT_URL)
        .send()
        .await?;
    
    let text = response.text().await?;
    Ok(text)
}

/// Computes a simple hash of the announcement text
#[must_use]
pub fn hash_announcement(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
