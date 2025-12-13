#![allow(non_snake_case)]
#![allow(unused_imports)]

mod DownloadControl;
mod Expand;
mod MyAnimatedCarousel;
mod MyButton;
mod MyNetworkImage;
mod MyNewsWidget;
mod MySidebarItem;
mod VideoBackgroundPlayer;

pub use DownloadControl::*;
pub use Expand::*;
pub use MyAnimatedCarousel::*;
pub use MyButton::*;
pub use MyNetworkImage::*;
pub use MyNewsWidget::*;
pub use MySidebarItem::*;
pub use VideoBackgroundPlayer::*;

// Helper functions for extracting video/theme URLs from backgrounds
use backend::game_providers::hoyoplay::proto::Background;

/// Extract the first video URL from backgrounds list
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

/// Extract the first theme URL from backgrounds list
pub fn get_theme_url(backgrounds: &[Background]) -> Option<String> {
    backgrounds
        .first()
        .map(|bg| bg.theme.url.clone())
        .filter(|url| !url.is_empty())
}

/// Get static background as fallback
pub fn get_background_url(backgrounds: &[Background]) -> Option<String> {
    backgrounds
        .first()
        .map(|bg| bg.background.url.clone())
        .filter(|url| !url.is_empty())
}