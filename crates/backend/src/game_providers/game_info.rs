use reqwest::Url;

use crate::game_providers::version::Version;

#[derive(Debug)]
pub struct GameInfo {
    pub id: String,
    pub name: String,
    pub editions: Vec<GameEdition>,
    pub visuals: GameVisuals,
}

#[derive(Debug)]
pub struct GameEdition {
    pub name: String,
    pub id: String,
    pub description: String,
    pub version_info: EditionVersionInfo,
}

#[derive(Debug)]
pub struct GameVisuals {
    pub background: GameBackground,
    pub icon: Image,
}

#[derive(Debug)]
pub enum GameBackground {
    Image(Image),
    Video(Video),
}

#[derive(Debug)]
pub struct Image {
    pub url: Url,
    pub size: Option<(u32, u32)>,
}

#[derive(Debug)]
pub struct Video {
    pub url: Url,
    pub overlay: Option<Url>,
    pub fallback: Url,
    pub size: Option<(u32, u32)>,
    pub framerate: Option<f32>,
}

#[derive(Debug)]
pub struct EditionVersionInfo {
    pub latest: Version,
    pub patches: Vec<Version>,
    pub pre_download: Option<Version>,
}
