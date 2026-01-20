use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProxyResponse {
    pub proxy_rsps: Vec<ProxyRsp>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub seq: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRsp {
    pub kind: String,
    #[serde(default)]
    pub get_latest_game_rsp: Option<GetLatestGameRsp>,
    #[serde(default)]
    pub get_main_bg_image_rsp: Option<GetMainBgImageRsp>,
    #[serde(default)]
    pub get_single_ent_rsp: Option<GetSingleEntRsp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLatestGameRsp {
    pub action: Option<i32>,
    pub version: Option<String>,
    #[serde(default)]
    pub request_version: Option<String>,
    pub pkg: Option<Pkg>,
    #[serde(default)]
    pub patch: Option<serde_json::Value>,
    pub state: Option<i32>,
    #[serde(default)]
    pub launcher_action: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pkg {
    pub packs: Vec<Pack>,
    #[serde(default)]
    pub total_size: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub package_size: Option<String>,
    #[serde(default)]
    pub file_id: Option<String>,
    #[serde(default)]
    pub sub_channel: Option<String>,
    #[serde(default)]
    pub game_files_md5: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub url: String,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub package_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMainBgImageRsp {
    #[serde(default)]
    pub data_version: Option<String>,
    pub main_bg_image: Option<MainBgImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainBgImage {
    pub url: String,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub video_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSingleEntRsp {
    pub single_ent: Option<SingleEnt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleEnt {
    pub version_url: String,
    #[serde(default)]
    pub version_md5: Option<String>,
    #[serde(default)]
    pub jump_url: Option<String>,
    #[serde(default)]
    pub button_url: Option<String>,
    #[serde(default)]
    pub button_md5: Option<String>,
    #[serde(default)]
    pub button_hover_url: Option<String>,
    #[serde(default)]
    pub button_hover_md5: Option<String>,
    #[serde(default)]
    pub need_token: Option<bool>,
}
