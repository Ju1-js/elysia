use std::sync::{Arc, RwLock};
use freya::prelude::*;
use reqwest::Url;
use backend::game_providers::hoyoplay::get_game_content;
use backend::settings::GlobalSettings;
use crate::{
    components::{MyAnimatedCarousel, MyNetworkImage, preload_images},
    context::Context,
};

#[component]
pub fn MyNewsWidget(game_id: String, carousel_index: Signal<usize>) -> Element {
    let ctx = use_context::<Context>();
    let settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    
    let Some(content) = ctx.api_news.get(&game_id).cloned() else {
        return rsx!({});
    };

    // Don't show widget if there are no banners
    if content.banners.is_empty() {
        return rsx!({});
    }

    let len = content.banners.len();

    // Preload all banner images in the background
    let game_id_for_effect = game_id.clone();
    let banners_for_effect = content.banners.clone();
    use_effect(use_reactive!(|game_id_for_effect| {
        let _ = game_id_for_effect; // React to game_id changes
        
        // Collect all banner URLs
        let urls: Vec<Url> = banners_for_effect.iter()
            .filter_map(|b| b.image.url.parse().ok())
            .collect();
        
        // Get cache path and convert PathBuf to String
        let cache_path = settings_sig.read()
            .read()
            .ok()
            .map(|s| s.cache_directory.display().to_string())
            .unwrap_or_default();
        
        // Preload images asynchronously
        preload_images(urls, cache_path);
    }));

    rsx! {
        rect {
            direction: "vertical",
            main_align: "start",
            spacing: "12",
            width: "100%",
            
            rect {
                direction: "vertical",
                width: "100%",
                spacing: "8",
                
                // Image Carousel
                rect {
                    direction: "horizontal",
                    spacing: "0",
                    padding: "0",
                    corner_radius: "12",
                    overflow: "clip",
                    shadow: "0 12 48 0 rgb(0, 0, 0, 80), 0 4 16 0 rgb(0, 0, 0, 60)",
                    background: "rgb(16,16,16,0.4)",
                    backdrop_blur: "24",
                    
                    MyAnimatedCarousel {
                        key: "{game_id}",
                        items: content.banners.iter().map(|banner| {
                            rsx! {
                                MyNetworkImage {
                                    url: banner.image.url.parse::<Url>().unwrap(),
                                    aspect_ratio: "min",
                                    sampling: "catmull-rom"
                                }
                            }
                        }).collect(),
                        selected: carousel_index,
                    }
                }
                
                // Pagination dots
                rect {
                    width: "100%",
                    height: "28",
                    direction: "horizontal",
                    main_align: "center",
                    cross_align: "center",
                    
                    rect {
                        direction: "horizontal",
                        spacing: "8",
                        padding: "6 12",
                        background: "rgb(0, 0, 0, 0.5)",
                        corner_radius: "8",
                        
                        for i in 0..len {
                            rect {
                                key: "{i}",
                                width: "8",
                                height: "8",
                                corner_radius: "4",
                                background: if i == carousel_index() {
                                    "rgb(255, 149, 0)"
                                } else {
                                    "rgb(255, 255, 255, 0.5)"
                                },
                                shadow: "0 2 4 0 rgb(0, 0, 0, 60)",
                            }
                        }
                    }
                }
            }
            
            // Event List (TODO)
            rect {
                // TODO: Add news/events list
            }
        }
    }
}