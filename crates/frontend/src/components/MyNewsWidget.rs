use std::sync::{Arc, RwLock};
use freya::prelude::*;
use reqwest::Url;
use backend::settings::GlobalSettings;
use crate::{
    components::{MyAnimatedCarousel, MyNetworkImage, preload_images, fetch_image},  // Added fetch_image
    context::Context,
};

#[component]
pub fn MyNewsWidget(game_id: String) -> Element {
    let ctx = use_context::<Context>();
    let settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    
    let mut carousel_index = use_signal(|| 0usize);
    let mut last_interaction = use_signal(|| std::time::Instant::now());
    
    let Some(content) = ctx.api_news.get(&game_id).cloned() else {
        return rsx!({});
    };

    if content.banners.is_empty() {
        return rsx!({});
    }

    let banner_count = content.banners.len();

    let game_id_for_effect = game_id.clone();
    let banners_for_effect = content.banners.clone();
    #[allow(unused_variables)]
    use_effect(use_reactive!(|game_id_for_effect| {
        carousel_index.set(0);
        
        let urls: Vec<Url> = banners_for_effect.iter()
            .filter_map(|b| b.image.url.parse().ok())
            .collect();
        
        let cache_path = settings_sig.read()
            .read()
            .ok()
            .map(|s| s.cache_directory.display().to_string())
            .unwrap_or_default();

        preload_images(urls, cache_path, |url| Box::pin(fetch_image(url)));
    }));

    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                let elapsed = last_interaction().elapsed();
                if elapsed >= std::time::Duration::from_secs(4) {
                    let current = carousel_index();
                    let next = (current + 1) % banner_count;
                    carousel_index.set(next);
                    last_interaction.set(std::time::Instant::now());
                }
            }
        });
    });

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
                
                rect {
                    direction: "horizontal",
                    spacing: "0",
                    padding: "0",
                    corner_radius: "16",
                    overflow: "clip",
                    shadow: "0 8 32 0 rgb(0, 0, 0, 100), 0 2 8 0 rgb(0, 0, 0, 80)",
                    
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
                        on_manual_change: move |_| {
                            last_interaction.set(std::time::Instant::now());
                        }
                    }
                }
                
                rect {
                    width: "100%",
                    height: "28",
                    direction: "horizontal",
                    main_align: "center",
                    cross_align: "center",
                    
                    rect {
                        direction: "horizontal",
                        spacing: "7",
                        padding: "6 14",
                        background: "rgb(0, 0, 0, 0.7)",
                        backdrop_blur: "16",
                        corner_radius: "16",
                        shadow: "0 4 12 0 rgb(0, 0, 0, 80)",
                        
                        for i in 0..banner_count {
                            rect {
                                key: "{i}",
                                width: if i == carousel_index() { "20" } else { "7" },
                                height: "7",
                                corner_radius: "3.5",
                                background: if i == carousel_index() {
                                    "rgb(255, 255, 255)"
                                } else {
                                    "rgb(255, 255, 255, 0.5)"
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
