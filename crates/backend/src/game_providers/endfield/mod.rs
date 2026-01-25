pub mod api;
mod download;
mod game;
mod installer;
mod repairer;

pub use download::{Progress, clear_progress, get_progress, set_progress};
pub use game::Game;
pub use installer::EndfieldInstaller;
pub use repairer::Repairer;

use crate::game_providers::hoyoplay::proto::{
    Background, Banner, Content, Display, Game as ProtoGame, GameBasicInfo, GameIdentifier,
    GameInfo, GetGameContent, GetGames, IconAsset, Image, ImageLink, MediaAsset, Post,
    SocialMedia, VideoAsset,
};
use api::BatchProxyResponse;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;
use std::path::{Path, PathBuf};

const BASE_URL: &str = "https://launcher.gryphline.com";
const APP_CODE: &str = "YDUTE5gscDZ229CW";

/// # Errors
/// Returns an error if the API request fails.
pub async fn batch_proxy_post(body: &Value) -> Result<Value, String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("Accept", HeaderValue::from_static("application/json"));

    let url = format!("{BASE_URL}/api/proxy/batch_proxy");

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

/// # Errors
/// Returns an error if the API request fails.
pub async fn batch_proxy_web_post(body: &Value) -> Result<Value, String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("Accept", HeaderValue::from_static("application/json"));

    let url = format!("{BASE_URL}/api/proxy/web/batch_proxy");

    let resp = client
        .post(&url)
        .headers(headers)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("batch_proxy_web request error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "batch_proxy_web returned status: {}",
            resp.status()
        ));
    }

    let json: Value = resp.json().await.map_err(|e| format!("parse json: {e}"))?;
    Ok(json)
}

/// # Errors
/// Returns an error if the image cannot be fetched.
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
        .map_err(|e| format!("Failed to deserialize batch response: {e}"))?;

    for proxy in typed.proxy_rsps {
        if let Some(bg_rsp) = proxy.get_main_bg_image_rsp
            && let Some(bg_image) = bg_rsp.main_bg_image
        {
            return Ok(bg_image.url);
        }
    }

    Err("No background image found in response".to_string())
}

/// # Errors
/// Returns an error if the theme overlay cannot be fetched.
pub async fn get_single_ent(app_code: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "proxy_reqs": [{
            "kind": "get_single_ent",
            "get_single_ent_req": {
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
        .map_err(|e| format!("Failed to deserialize batch response: {e}"))?;

    for proxy in typed.proxy_rsps {
        if let Some(ent_rsp) = proxy.get_single_ent_rsp
            && let Some(ent) = ent_rsp.single_ent
        {
            return Ok(ent.version_url);
        }
    }

    Err("No theme overlay found in response".to_string())
}

/// # Errors
/// Returns an error if game data cannot be fetched.
pub async fn get_games() -> Result<GetGames, String> {
    let placeholder_icon = Image {
        url: "https://play-lh.googleusercontent.com/l6FVNa293RykBWy88TqEhUakIcGSC8bRygSnKOBgztln48JX-WzMWnrBAETrKZsxDNC4HhwCsvfle_UI7rBE=w960-h1920-rw".to_string(),
        hover_url: String::new(),
        link: String::new(),
        login_state_in_link: false,
        md5: String::new(),
        size: 0,
    };

    let background_url = get_main_bg_image(APP_CODE)
        .await
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

    let game = ProtoGame {
        id: APP_CODE.to_string(),
        biz: "endfield".to_string(),
        display,
        reservation: None,
        display_status: "online".to_string(),
        game_server_configs: Vec::new(),
    };

    Ok(GetGames { games: vec![game] })
}

/// # Errors
/// Returns an error if game content cannot be fetched.
#[allow(clippy::unused_async)]
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

/// Get game basic info for endfield including theme overlay
/// # Errors
/// Returns an error if the API request fails.
pub async fn get_game_basic_info() -> Result<Vec<GameBasicInfo>, String> {
    let background_url = get_main_bg_image(APP_CODE)
        .await
        .unwrap_or_else(|err| {
            eprintln!("[WARNING] Failed to get endfield background URL: {err}");
            String::new()
        });

    let theme_url = get_single_ent(APP_CODE)
        .await
        .unwrap_or_else(|err| {
            eprintln!("[WARNING] Failed to get endfield theme URL: {err}");
            String::new()
        });

    let background = Background {
        id: "endfield_bg_1".to_string(),
        background: MediaAsset {
            url: background_url,
            link: String::new(),
            login_state_in_link: false,
        },
        icon: IconAsset {
            url: String::new(),
            hover_url: String::new(),
            link: String::new(),
            login_state_in_link: false,
            md5: String::new(),
            size: 0,
        },
        video: VideoAsset {
            url: String::new(),
            size: 0,
        },
        theme: MediaAsset {
            url: theme_url,
            link: String::new(),
            login_state_in_link: false,
        },
        background_type: "default".to_string(),
    };

    let game_info = GameBasicInfo {
        game: GameIdentifier {
            id: APP_CODE.to_string(),
            biz: "endfield".to_string(),
        },
        backgrounds: vec![background],
    };

    Ok(vec![game_info])
}
