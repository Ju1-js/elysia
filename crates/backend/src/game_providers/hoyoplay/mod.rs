pub mod proto;

use std::{
    collections::{HashMap, HashSet},
    fs::read_dir,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use proto::{ApiResponse, Background, GetAllGameBasicInfo, GetGameContent, GetGames};
use reqwest::{self, Url};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{
    game_providers::{
        self, GameProvider,
        game_info::{GameBackground, GameEdition, GameVisuals},
        hoyoplay::proto::{GetGameConfigs, GetGameScanInfo},
    },
    settings::GlobalSettings,
};

// FIXME: add all query params that should be there
// TODO: language selection
const API_URL: &str = "https://sg-hyp-api.hoyoverse.com/hyp/hyp-connect/api";
// TODO: other launchers (global, china, 3x bilibili?)
const LAUNCHER_ID: &str = "VYTpXlbWo8"; // Global

pub async fn get_games(settings: &GlobalSettings) -> Result<GetGames, String> {
    let url = format!("{API_URL}/getGames?launcher_id={LAUNCHER_ID}&language=en-us");

    return cached_request(settings, &url).await;
}

pub async fn get_game_content(
    settings: &GlobalSettings,
    game_id: &str,
) -> Result<GetGameContent, String> {
    let url = format!(
        "{API_URL}/getGameContent?game_id={}&launcher_id={LAUNCHER_ID}&language=en-us",
        game_id
    );

    return cached_request(settings, &url).await;
}

pub async fn get_game_scan_info(settings: &GlobalSettings) -> Result<GetGameScanInfo, String> {
    let url = format!("{API_URL}/getGameScanInfo?launcher_id={LAUNCHER_ID}&language=en-us");

    return cached_request(settings, &url).await;
}

pub async fn get_game_configs(settings: &GlobalSettings) -> Result<GetGameConfigs, String> {
    let url = format!("{API_URL}/getGameConfigs?launcher_id={LAUNCHER_ID}&language=en-us");

    return cached_request(settings, &url).await;
}

/// Get all game basic info including video backgrounds
/// If game_id is provided, filters for that specific game
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
pub async fn get_game_video_backgrounds(
    settings: &GlobalSettings,
    game_id: &str,
) -> Result<Vec<String>, String> {
    let basic_info = get_all_game_basic_info(settings, Some(game_id)).await?;

    let mut video_urls = Vec::new();
    for game_info in basic_info.game_info_list {
        if game_info.game.id == game_id {
            for background in game_info.backgrounds {
                if background.background_type == "BACKGROUND_TYPE_VIDEO" && !background.video.url.is_empty() {
                    video_urls.push(background.video.url);
                }
            }
            break;
        }
    }

    Ok(video_urls)
}

pub async fn scan_dir(
    settings: &GlobalSettings,
    path: &Path,
) -> Result<Vec<(String, String, String)>, String> {
    let configs = get_game_configs(settings).await?;
    let mut exe_set = HashSet::new();
    for cfg in configs.launch_configs {
        exe_set.insert(cfg.exe_file_name.clone());
    }

    let mut found_exes = Vec::new();
    scan(path, &exe_set, 0, &mut found_exes);

    let scan_info = get_game_scan_info(settings).await?;

    let mut out = Vec::new();
    for exe in &found_exes {
        let hash = md5(exe).await?;
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

async fn md5(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .await
        .map_err(|e| format!("Failed to open file: {}", e))?;
    let mut hasher = md5::Context::new();
    let mut buffer = vec![0; 1024];

    loop {
        let size = file
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        if size == 0 {
            break; // EOF
        }

        hasher.consume(&buffer[..size]);
    }

    let result = hasher.finalize();
    let hash_hex = format!("{:x}", result);

    Ok(hash_hex)
}

fn scan(dir: &Path, exe_set: &HashSet<String>, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 1 {
        return;
    }

    if let Ok(entries) = read_dir(dir) {
        for entry in entries.into_iter().flatten() {
            let path = entry.path();
            if depth == 0 && path.is_dir() {
                scan(&path, exe_set, depth + 1, out);
            } else if let Some(file_name) = path.file_name()
                && let Some(name_str) = file_name.to_str()
                && exe_set.contains(&name_str.to_string())
            {
                out.push(path);
            }
        }
    }
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

pub fn get_video_url(backgrounds: &[Background]) -> Option<String> {
    backgrounds
        .iter()
        .find_map(|bg| {
            if !bg.video.url.is_empty() {
                Some(bg.video.url.clone())
            } else {
                None
            }
        })
}

pub fn get_theme_url(backgrounds: &[Background]) -> Option<String> {
    backgrounds
        .first()
        .map(|bg| bg.theme.url.clone())
        .filter(|url| !url.is_empty())
}

pub struct HoyoplayProvider {}

#[async_trait]
impl GameProvider for HoyoplayProvider {
    async fn fetch_games(
        &self,
        settings: &GlobalSettings,
    ) -> anyhow::Result<Vec<game_providers::GameInfo>> {


        let games =  get_games(settings).await.map_err(|e| anyhow::anyhow!(e))?;
        let basic_info = get_all_game_basic_info(settings, None).await.map_err(|e| anyhow::anyhow!(e))?;

        let mut background_map: HashMap<String, GameBackground> = basic_info.game_info_list.iter().map(|info| {
            let id = info.game.id.to_owned();
            let mut backgrounds = info.backgrounds.iter().map(|b| {
                if b.background_type == "BACKGROUND_TYPE_VIDEO" {
                    let url = Url::parse(&b.video.url).unwrap();
                    let overlay = Url::parse(&b.theme.url).unwrap();
                    let fallback = Url::parse(&b.background.url).unwrap();
                    GameBackground::Video(game_providers::game_info::Video {
                        url,
                        overlay: Some(overlay),
                        fallback,
                        size: None,
                        framerate: None })
                } else {
                    let url = Url::parse(&b.background.url).unwrap();
                    GameBackground::Image(game_providers::game_info::Image {
                        url,
                        size: None })
                }
            });

            let bg = backgrounds.next().unwrap();
            (id, bg)
        }).collect();

        let infos = games.games.iter().map(|game| {
            let background = background_map.remove(&game.id).unwrap();

            game_providers::GameInfo {
                id: game.id.to_owned(),
                name: game.display.name.to_owned(),
                editions: game.game_server_configs.iter().map(|cfg| GameEdition {
                    name: cfg.i18n_name.to_owned(),
                    id: cfg.game_id.to_owned(),
                }).collect(),
                visuals: GameVisuals {
                    background,
                    icon: game_providers::game_info::Image {
                        url: Url::parse(&game.display.icon.url).unwrap(),
                        size: None,
                    },
                },
            }
        }).collect();
        Ok(infos)
    }
}
