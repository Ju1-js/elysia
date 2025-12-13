#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProxyResponse {
    pub proxy_rsps: Vec<ProxyRsp>,
    pub timestamp: String,
    pub seq: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyRsp {
    GetLatestGame(ProxyRspGetLatestGame),
    GetMainBgImage(ProxyRspGetMainBgImageRsp),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRspGetLatestGame {
    pub kind: String,
    pub get_latest_game_rsp: GetLatestGameRsp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRspGetMainBgImageRsp {
    pub kind: String,
    pub get_main_bg_image_rsp: GetMainBgImageRsp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLatestGameRsp {
    pub action: i32,
    pub version: String,
    pub request_version: String,
    pub pkg: Pkg,
    pub patch: Option<serde_json::Value>, // wtf is this
    pub state: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pkg {
    pub packs: Vec<Pack>,
    pub total_size: String,
    pub file_path: String,
    pub url: String,
    pub md5: String,
    pub package_size: String,
    pub file_id: String,
    pub sub_channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub url: String,
    pub md5: String,
    pub package_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMainBgImageRsp {
    pub main_bg_image: MainBgImage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainBgImage {
    pub url: String,
    pub md5: String,
    pub size: i32,
}
