use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetAllGameBasicInfo {
    pub game_info_list: Vec<GameBasicInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GameBasicInfo {
    pub game: GameIdentifier,
    pub backgrounds: Vec<Background>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GameIdentifier {
    pub id: String,
    pub biz: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Background {
    pub id: String,
    pub background: MediaAsset,
    pub icon: IconAsset,
    pub video: VideoAsset,
    pub theme: MediaAsset,
    #[serde(rename = "type")]
    pub background_type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MediaAsset {
    pub url: String,
    pub link: String,
    pub login_state_in_link: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IconAsset {
    pub url: String,
    pub hover_url: String,
    pub link: String,
    pub login_state_in_link: bool,
    pub md5: String,
    pub size: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoAsset {
    pub url: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<DataType> {
    pub retcode: i32,
    pub message: String,
    pub data: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameConfigs {
    pub launch_configs: Vec<LaunchConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameScanInfo {
    pub game_scan_info: Vec<GameScanInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGames {
    pub games: Vec<Game>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGameContent {
    pub content: Content,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub game: GameInfo,
    pub exe_file_name: String,
    pub installation_dir: String,
    pub audio_pkg_scan_dir: String,
    pub audio_pkg_res_dir: String,
    pub audio_pkg_cache_dir: String,
    pub game_cached_res_dir: String,
    pub game_screenshot_dir: String,
    pub game_log_gen_dir: String,
    pub game_crash_file_gen_dir: String,
    pub default_download_mode: String,
    pub enable_customer_service: bool,
    pub local_res_dir: String,
    pub local_res_cache_dir: String,
    pub res_category_dir: String,
    pub game_res_cut_dir: String,
    pub enable_game_log_export: bool,
    pub game_log_export_config: Option<GameLogExportConfig>,
    pub blacklist_dir: String,
    pub wpf_exe_dir: String,
    pub wpf_pkg_version_dir: String,
    pub enable_audio_pkg_mgmt: bool,
    pub audio_pkg_config_dir: String,
    pub enable_resource_deletion_adapter: bool,
    pub enable_resource_blacklist: bool,
    pub enable_redundant_file_cleanup: bool,
    pub redundant_file_cleanup_paths: Vec<String>,
    pub enable_v2_game_detection: bool,
    pub related_processes: Vec<String>,
    pub enable_ldiff: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLogExportConfig {
    pub file_size_filter: String,
    pub export_timeout: String,
    pub export_files: Vec<ExportFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFile {
    pub file_type: String,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameScanInfo {
    pub game_id: String,
    pub game_exe_list: Vec<GameExe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameExe {
    pub version: String,
    pub md5: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub biz: String,
    pub display: Display,
    pub reservation: Option<serde_json::Value>, // null or future object
    pub display_status: String,
    #[serde(default)]
    pub game_server_configs: Vec<GameServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    pub language: String,
    pub name: String,
    pub icon: Image,
    pub title: String,
    pub subtitle: String,
    pub background: ImageLink,
    pub logo: ImageLink,
    pub thumbnail: ImageLink,
    pub korea_rating: Option<serde_json::Value>,
    pub shortcut: Image,
    pub wpf_icon: Option<Image>, // sometimes null
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub url: String,
    pub hover_url: String,
    pub link: String,
    pub login_state_in_link: bool,
    pub md5: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageLink {
    pub url: String,
    pub link: String,
    pub login_state_in_link: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameServerConfig {
    pub i18n_name: String,
    pub i18n_description: String,
    pub package_name: String,
    pub auto_scan_registry_key: String,
    pub package_detection_info: String,
    pub game_id: String,
    pub reservation: Option<serde_json::Value>,
    pub display_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub game: GameInfo,
    pub language: String,
    pub banners: Vec<Banner>,
    pub posts: Vec<Post>,
    pub social_media_list: Vec<SocialMedia>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: String,
    pub biz: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Banner {
    pub id: String,
    pub image: ImageLink,
    pub i18n_identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    #[serde(rename = "type")]
    pub post_type: String,
    pub title: String,
    pub link: String,
    pub date: String,
    pub login_state_in_link: bool,
    pub i18n_identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMedia {
    pub id: String,
    pub icon: Image,
    pub qr_image: ImageLink,
    pub qr_desc: String,
    #[serde(default)]
    pub links: Vec<serde_json::Value>, // empty array in sample
    pub enable_red_dot: bool,
    pub red_dot_content: String,
}
