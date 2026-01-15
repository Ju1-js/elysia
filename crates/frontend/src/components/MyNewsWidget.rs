use crate::{components::MyAnimatedCarousel, components::MyNetworkImage, context::Context};
use backend::settings::GlobalSettings;
use freya::prelude::*;
use reqwest::Url;
use std::sync::{Arc, RwLock};

/// News widget displaying a carousel of banners for a game
#[component]
pub fn MyNewsWidget(game_id: String) -> Element {
    let ctx = use_context::<Context>();
    let game_state = use_context::<crate::pages::game::state::GlobalGameStateSignal>();

    let mut carousel_index = use_signal(|| 0usize);
    let mut last_interaction = use_signal(std::time::Instant::now);

    let Some(content) = ctx.api_news.get(&game_id).cloned() else {
        return rsx!({});
    };

    if content.banners.is_empty() {
        return rsx!({});
    }

    let banner_count = content.banners.len();

    let game_id_for_effect = game_id.clone();
    #[allow(unused_variables)]
    use_effect(use_reactive!(|game_id_for_effect| {
        carousel_index.set(0);
    }));

    // Auto-scroll carousel, but only when game is not running
    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                // Check if game is running - pause auto-scroll if it is
                let game_running = game_state.read().is_game_running();
                if game_running {
                    continue; // Skip auto-scroll when game is running
                }

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
                        on_manual_change: move |()| {
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
                        corner_radius: "16",

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
