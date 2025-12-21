mod installer;
mod download;
pub mod api;

pub use installer::EndfieldInstaller;
pub use download::{Progress, get_progress, set_progress, clear_progress};

use std::path::{Path, PathBuf};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::Value;
use crate::game_providers::hoyoplay::proto::{
    Banner, Content, Display, Game, GameInfo, GetGameContent, GetGames, Image, ImageLink,
    Post, SocialMedia,
};
use api::BatchProxyResponse;

const BASE_URL: &str = "https://launcher.gryphline.com";

pub async fn batch_proxy_post(body: &Value) -> Result<Value, String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("Accept", HeaderValue::from_static("application/json"));

    let url = format!("{}/api/proxy/batch_proxy", BASE_URL);

    let resp = client
        .post(&url)
        .headers(headers)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("batch_proxy request error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("batch_proxy returned status: {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("parse json: {e}"))?;
    Ok(json)
}

pub async fn batch_proxy_web_post(body: &Value) -> Result<Value, String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("Accept", HeaderValue::from_static("application/json"));

    let url = format!("{}/api/proxy/web/batch_proxy", BASE_URL);

    let resp = client
        .post(&url)
        .headers(headers)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("batch_proxy_web request error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("batch_proxy_web returned status: {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("parse json: {e}"))?;
    Ok(json)
}

#[allow(dead_code)]
pub async fn batch_proxy_post_from_file(path: &Path) -> Result<Value, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read batch json {path:?}: {e}"))?;
    let v: Value = serde_json::from_str(&data).map_err(|e| format!("Invalid json file: {e}"))?;
    batch_proxy_post(&v).await
}

pub async fn install_from_batch_body_value(
    resp_json: Value,
    game_name: &str,
    games_dir: &Path,
    temp_dir: &Path,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(games_dir)
        .map_err(|e| format!("Failed to create games directory: {}", e))?;
    
    std::fs::create_dir_all(temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let typed: BatchProxyResponse = match serde_json::from_value(resp_json) {
        Ok(t) => t,
        Err(e) => {
            let key = format!("{}_streaming", game_name);
            download::set_progress(
                &key,
                download::Progress {
                    downloaded: 0,
                    total: 0,
                    part_index: 0,
                    parts_total: 0,
                    status: format!("Error: Failed to deserialize batch response: {}", e),
                    mb_s: 0.0,
                    is_busy: false,
                },
            );
            return Err(format!("Failed to deserialize batch response: {e}"));
        }
    };

    for proxy in typed.proxy_rsps.into_iter() {
        if let Some(get_latest) = proxy.get_latest_game_rsp {
            if let Some(pkg) = get_latest.pkg {

                let packs: Vec<api::Pack> = pkg
                    .packs
                    .into_iter()
                    .filter(|p| !p.url.is_empty())
                    .collect();

                if packs.is_empty() {
                    let key = format!("{}_streaming", game_name);
                    download::set_progress(
                        &key,
                        download::Progress {
                            downloaded: 0,
                            total: 0,
                            part_index: 0,
                            parts_total: 0,
                            status: "Error: No pack URLs found in batch response".to_string(),
                            mb_s: 0.0,
                            is_busy: false,
                        },
                    );
                    return Err("No pack URLs found in batch response".to_string());
                }

                let progress_key = format!("{}_streaming", game_name);
                let dest = games_dir.join("endfield");

                eprintln!("[INFO] Starting streaming download & extraction");
                eprintln!("[INFO]   Parts: {}", packs.len());
                eprintln!("[INFO]   Destination: {:?}", dest);
                eprintln!("[INFO]   Temp directory: {:?}", temp_dir);

                std::fs::create_dir_all(&dest)
                    .map_err(|e| format!("Failed to create game directory: {}", e))?;

                download::set_progress(
                    &progress_key,
                    download::Progress {
                        downloaded: 0,
                        total: 0,
                        part_index: 0,
                        parts_total: packs.len(),
                        status: "Preparing download...".to_string(),
                        mb_s: 0.0,
                        is_busy: true,
                    },
                );

                download::download_and_extract_streaming(
                    packs,
                    &dest,
                    &progress_key,
                    game_name,
                ).await?;

                eprintln!("[INFO] Installation complete: {:?}", dest);
                return Ok(dest);
            }
        }
    }

    let key = format!("{}_streaming", game_name);
    download::set_progress(
        &key,
        download::Progress {
            downloaded: 0,
            total: 0,
            part_index: 0,
            parts_total: 0,
            status: "Error: No get_latest_game response with package info found".to_string(),
            mb_s: 0.0,
            is_busy: false,
        },
    );
    Err("No get_latest_game response with package info found".to_string())
}

pub async fn get_main_bg_image(app_code: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "proxy_reqs": [{
            "kind": "get_main_bg_image",
            "get_main_bg_image_req": {
                "appcode": app_code,
                "language": "en-us",
                "channel": "6",
                "sub_channel": "6",
                "platform": "Windows",
                "source": "launcher"
            }
        }]
    });

    let resp_json = batch_proxy_web_post(&body).await?;
    
    let typed: BatchProxyResponse = serde_json::from_value(resp_json)
        .map_err(|e| format!("Failed to deserialize batch response: {}", e))?;

    for proxy in typed.proxy_rsps {
        if let Some(bg_rsp) = proxy.get_main_bg_image_rsp
            && let Some(bg_image) = bg_rsp.main_bg_image {
                return Ok(bg_image.url);
            }
    }

    Err("No background image found in response".to_string())
}

pub async fn get_games() -> Result<GetGames, String> {
    let app_code = "zePXHT2t4L2tKR4m";
    
    let placeholder_icon = Image {
        url: "https://play-lh.googleusercontent.com/l6FVNa293RykBWy88TqEhUakIcGSC8bRygSnKOBgztln48JX-WzMWnrBAETrKZsxDNC4HhwCsvfle_UI7rBE=w960-h1920-rw".to_string(),
        hover_url: String::new(),
        link: String::new(),
        login_state_in_link: false,
        md5: String::new(),
        size: 0,
    };

    let background_url = get_main_bg_image(app_code).await
        .unwrap_or_else(|_| String::new());

    let placeholder_image = ImageLink {
        url: background_url,
        link: String::new(),
        login_state_in_link: false,
    };

    let display = Display {
        language: "en-us".to_string(),
        name: "Arknights: Endfield".to_string(),
        icon: placeholder_icon.clone(),
        title: "Endfield".to_string(),
        subtitle: String::new(),
        background: placeholder_image.clone(),
        logo: placeholder_image.clone(),
        thumbnail: placeholder_image.clone(),
        korea_rating: None,
        shortcut: placeholder_icon,
        wpf_icon: None,
    };

    let game = Game {
        id: "zePXHT2t4L2tKR4m".to_string(),
        biz: "endfield".to_string(),
        display,
        reservation: None,
        display_status: "online".to_string(),
        game_server_configs: Vec::new(),
    };

    Ok(GetGames { games: vec![game] })
}

pub async fn get_game_content(game_id: &str) -> Result<GetGameContent, String> {
    let content = Content {
        game: GameInfo {
            id: game_id.to_string(),
            biz: "endfield".to_string(),
        },
        language: "en-us".to_string(),
        banners: Vec::<Banner>::new(),
        posts: Vec::<Post>::new(),
        social_media_list: Vec::<SocialMedia>::new(),
    };

    Ok(GetGameContent { content })
}