use freya::prelude::*;
use reqwest::Url;
use backend::game_providers::hoyoplay::get_game_content;
use crate::{
    components::{MyAnimatedCarousel, MyNetworkImage},
    context::Context,
};

#[component]
pub fn MyNewsWidget(game_id: String) -> Element {
    let ctx = &use_context::<Context>();
    let Some(content) = ctx.api_news.get(&game_id) else {
        return rsx!({});
    };
    
    // Don't show widget if there are no banners
    if content.banners.is_empty() {
        return rsx!({});
    }
    
    let selected = use_signal(|| 0);
    let len = content.banners.len();

    let preload_signals: Vec<_> = content.banners.iter().map(|banner| {
        use_signal(|| banner.image.url.parse::<Url>().ok())
    }).collect();
    
    rsx! {
        rect {
            direction: "vertical",
            main_align: "start",
            spacing: "12",
            width: "100%",
            
            // try to preload all widget images (?)
            // to-do: do this a better way
            rect {
                width: "0",
                height: "0",
                overflow: "clip",
                for (i, url_sig) in preload_signals.iter().enumerate() {
                    if let Some(url) = url_sig.read().clone() {
                        MyNetworkImage {
                            key: "{i}",
                            url: url,
                            width: "1",
                            height: "1",
                        }
                    }
                }
            }
            
            rect {
                direction: "vertical",
                width: "100%",
                spacing: "8",
                rect { // Image Carousel
                    direction: "horizontal",
                    spacing: "0",
                    padding: "0",
                    corner_radius: "12",
                    overflow: "clip",
                    shadow: "0 12 48 0 rgb(0, 0, 0, 80), 0 4 16 0 rgb(0, 0, 0, 60)",
                    background: "rgb(16,16,16,0.4)",
                    backdrop_blur: "24",
                    MyAnimatedCarousel {
                        items: content.banners.iter().map(|url| {
                            rsx!{
                                MyNetworkImage {
                                    url: url.image.url.parse::<Url>().unwrap(),
                                    aspect_ratio: "min",
                                    sampling: "catmull-rom"
                                }
                            }
                        }).collect(),
                        selected: selected,
                    }
                },
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
                                background: if i == selected() {
                                    "rgb(255, 149, 0)"
                                } else {
                                    "rgb(255, 255, 255, 0.5)"
                                },
                                shadow: "0 2 4 0 rgb(0, 0, 0, 60)",
                            }
                        }
                    }
                }
            },
            rect { // Event List
                // TODO: Add news/events list
            }
        }
    }
}