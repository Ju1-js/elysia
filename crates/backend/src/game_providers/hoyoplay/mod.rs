pub mod proto;

use std::{
    collections::{HashMap, HashSet},
    fs::read_dir,
    io::Read,
    path::{Path, PathBuf},
};

use md5::Digest;
use proto::{ApiResponse, Background, Game, GetAllGameBasicInfo, GetGameContent, GetGames};
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{
    game_providers::{hoyoplay::proto::{GameExe, GameInfo, GetGameConfigs, GetGameScanInfo}, scanner},
    settings::GlobalSettings,
};

// FIXME: add all query params that should be there
// TODO: language selection
const API_URL: &str = "https://sg-hyp-api.hoyoverse.com/hyp/hyp-connect/api";
// TODO: other launchers (global, china, 3x bilibili?)
const LAUNCHER_ID: &str = "VYTpXlbWo8"; // Global

/// # Errors
/// Returns an error if the API request fails.
pub async fn get_games(settings: &GlobalSettings) -> Result<GetGames, String> {
    let url = format!("{API_URL}/getGames?launcher_id={LAUNCHER_ID}&language=en-us");

    return cached_request(settings, &url).await;
}

/// # Errors
/// Returns an error if the API request fails.
pub async fn get_game_content(
    settings: &GlobalSettings,
    game_id: &str,
) -> Result<GetGameContent, String> {
    let url = format!(
        "{API_URL}/getGameContent?game_id={game_id}&launcher_id={LAUNCHER_ID}&language=en-us"
    );

    return cached_request(settings, &url).await;
}

/// # Errors
/// Returns an error if the API request fails.
pub async fn get_game_scan_info(settings: &GlobalSettings) -> Result<GetGameScanInfo, String> {
    let url = format!("{API_URL}/getGameScanInfo?launcher_id={LAUNCHER_ID}&language=en-us");

    return cached_request(settings, &url).await;
}

/// # Errors
/// Returns an error if the API request fails.
pub async fn get_game_configs(settings: &GlobalSettings) -> Result<GetGameConfigs, String> {
    let url = format!("{API_URL}/getGameConfigs?launcher_id={LAUNCHER_ID}&language=en-us");

    return cached_request(settings, &url).await;
}

/// Get all game basic info including video backgrounds
/// If `game_id` is provided, filters for that specific game
/// # Errors
/// Returns an error if the API request fails.
pub async fn get_all_game_basic_info(
    settings: &GlobalSettings,
    game_id: Option<&str>,
) -> Result<GetAllGameBasicInfo, String> {
    let game_id_param = game_id.unwrap_or("");
    let url = format!(
        "{API_URL}/getAllGameBasicInfo?launcher_id={LAUNCHER_ID}&language=en-us&game_id={game_id_param}"
    );

    return cached_request(settings, &url).await;
}

/// Get video backgrounds for a specific game
/// # Errors
/// Returns an error if the API request fails.
pub async fn get_game_video_backgrounds(
    settings: &GlobalSettings,
    game_id: &str,
) -> Result<Vec<String>, String> {
    let basic_info = get_all_game_basic_info(settings, Some(game_id)).await?;

    let mut video_urls = Vec::new();
    for game_info in basic_info.game_info_list {
        if game_info.game.id == game_id {
            for background in game_info.backgrounds {
                if background.background_type == "BACKGROUND_TYPE_VIDEO"
                    && !background.video.url.is_empty()
                {
                    video_urls.push(background.video.url);
                }
            }
            break;
        }
    }

    Ok(video_urls)
}

/// # Errors
/// Returns an error if scanning fails.
pub async fn scan_dir(
    settings: &GlobalSettings,
    path: &Path,
) -> Result<Vec<(String, String, String)>, String> {
    let configs = get_game_configs(settings).await?;
    let mut exe_set = HashSet::new();
    for cfg in configs.launch_configs {
        exe_set.insert(cfg.exe_file_name.clone());
    }

    // Use the general scanner with depth of 1 (root + immediate subdirectories)
    let found_exes = scanner::scan_for_executables(path, 1, Some(&exe_set));

    let scan_info = get_game_scan_info(settings).await?;

    let mut out = Vec::new();
    for exe in &found_exes {
        let hash = scanner::calculate_md5(exe).await?;
        for game in &scan_info.game_scan_info {
            let version = game.game_exe_list.iter().find(|v| v.md5 == hash);

            if let Some(version) = version {
                out.push((
                    exe.to_string_lossy().to_string(),
                    game.game_id.clone(),
                    version.version.clone(),
                ));
                break;
            }
        }
    }

    Ok(out)
}

// TODO: cache invalidation on demand
async fn cached_request<Type>(settings: &GlobalSettings, url: &str) -> Result<Type, String>
where
    Type: for<'a> Deserialize<'a> + Serialize,
{
    let cache_path = &settings.cache_directory;

    if let Ok(asset) = cacache::read_sync(cache_path, url) {
        let parsed = serde_json::from_slice(&asset).map_err(|e| format!("Cache parse error: {e}"));
        if let Ok(games) = parsed {
            return Ok(games);
        }
    }

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Api request error: {e}"))?;
    let response: ApiResponse<Type> = response
        .json()
        .await
        .map_err(|e| format!("Api parse error: {e}"))?;

    let _ = cacache::write_sync(cache_path, url, serde_json::to_vec(&response.data).unwrap());

    Ok(response.data)
}

#[must_use] 
pub fn get_video_url(backgrounds: &[Background]) -> Option<String> {
    backgrounds.iter().find_map(|bg| {
        if bg.video.url.is_empty() {
            None
        } else {
            Some(bg.video.url.clone())
        }
    })
}

#[must_use] 
pub fn get_theme_url(backgrounds: &[Background]) -> Option<String> {
    backgrounds
        .first()
        .map(|bg| bg.theme.url.clone())
        .filter(|url| !url.is_empty())
}
