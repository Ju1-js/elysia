use freya::{
    core::custom_attributes::NodeReferenceLayout,
    prelude::*,
};
use freya_router::prelude::*;
use reqwest::Url;

use crate::{
    Context,
    components::{Expand, MyNetworkImage, MySidebarItem},
    pages::{ErrorPage, Game, Home, Settings},
};

#[derive(Clone, Debug, PartialEq)]
pub struct GamePageState {
    pub prev_game_id: Option<String>,
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Home,
        #[route("/games")]
        Game,
        #[route("/settings")]
        Settings,
    #[end_layout]
    #[route("/..route")]
    ErrorPage {},
}

/// Main application layout component
#[component]
pub fn app() -> Element {
    rsx! { Router::<Route> {} }
}

/// Animated route transition component
#[component]
fn FromRouteToCurrent(
    from: Element,
    upwards: bool,
    node_size: ReadOnlySignal<NodeReferenceLayout>,
) -> Element {
    let mut animated_router = use_animated_router::<Route>();
    let animations = use_animation_with_dependencies(&upwards, move |_conf, _upwards| {
        AnimNum::new(1.0, 0.96)
            .time(300)
            .ease(Ease::Out)
            .function(Function::Cubic)
    });

    use_memo(use_reactive((&upwards, &from), move |_| {
        animations.run(AnimDirection::Forward)
    }));

    use_effect(move || {
        if !animations.is_running() && animations.has_run_yet() {
            animated_router.write().settle();
        }
    });

    let scale_from = animations.get().read().read();
    let scale_to = 0.96 + (1.0 - scale_from);
    let to = rsx!(Outlet::<Route> {});

    rsx!(
        rect {
            height: "fill",
            width: "fill",
            rect {
                height: "fill",
                width: "fill",
                position: "absolute",
                position_top: "0",
                position_left: "0",
                rotate: "0deg",
                scale: "{scale_from}",
                Expand { {from} }
            }
            rect {
                height: "fill",
                width: "fill",
                rotate: "0deg",
                scale: "{scale_to}",
                Expand { {to} }
            }
        }
    )
}

#[component]
fn AnimatedOutlet(children: Element) -> Element {
    let (reference, node_size) = use_node_signal();
    let animated_router = use_context::<Signal<AnimatedRouterContext<Route>>>();

    let from_route = match *animated_router.peek() {
        AnimatedRouterContext::FromTo(Route::Home, Route::Game) => Some((
            rsx!(Home {
                key: "home-transition"
            }),
            true,
        )),
        _ => None,
    };

    rsx!(
        rect {
            reference,
            if let Some((from, upwards)) = from_route {
                FromRouteToCurrent { upwards, from, node_size }
            } else {
                Expand {
                    Outlet::<Route> {}
                }
            }
        }
    )
}

fn make_links(
    games: &[backend::game_providers::hoyoplay::proto::Game],
    selected_game_id: Signal<Option<String>>,
    navigator: Navigator,
) -> Vec<Element> {
    games
        .iter()
        .map(|game| {
            let is_active = selected_game_id.read().as_ref() == Some(&game.id);
            let mut selected_game_id_mut = selected_game_id;
            let game_id = game.id.clone();

            rsx!(
                rect {
                    key: game.id.clone(),
                    width: "100%",
                    main_align: "center",
                    cross_align: "center",
                    onclick: move |_| {
                        selected_game_id_mut.write().replace(game_id.clone());
                        navigator.push(Route::Game);
                    },
                    MySidebarItem {
                        is_active: is_active,
                        {
                            match game.display.icon.url.parse::<Url>() {
                                Ok(url) => rsx!(
                                    rect {
                                        width: "48",
                                        height: "48",
                                        corner_radius: "10",
                                        overflow: "clip",
                                        padding: "2",
                                        MyNetworkImage {
                                            url: url,
                                            width: "100%",
                                            height: "100%",
                                            aspect_ratio: "max",
                                            cover: "center",
                                            sampling: "trilinear"
                                        }
                                    }
                                ),
                                Err(_) => rsx!(
                                    rect {
                                        width: "48",
                                        height: "48",
                                        corner_radius: "10",
                                        padding: "2",
                                        main_align: "center",
                                        cross_align: "center",
                                        label {
                                            font_size: "9",
                                            {game.display.name.clone()}
                                        }
                                    }
                                )
                            }
                        }
                    }
                }
            )
        })
        .collect::<Vec<_>>()
}

#[allow(non_snake_case)]
fn AppLayout() -> Element {
    let ctx_resource = &use_context::<Resource<Context>>();
    let selected_game_id = use_signal(|| None::<String>);
    let game_page_state = use_signal(|| GamePageState { prev_game_id: None });
    let navigator = use_navigator();

    use_context_provider(|| selected_game_id);
    use_context_provider(|| game_page_state);

    let ctx_option = ctx_resource.read_unchecked().clone();
    if let Some(ref ctx) = ctx_option {
        use_context_provider(|| ctx.clone());
    }

    rsx! {
        NativeRouter {
            AnimatedRouter::<Route> {
                rect {
                    width: "100%",
                    height: "100%",
                    direction: "horizontal",

                    rect {
                        position: "absolute",
                        position_top: "0",
                        position_left: "0",
                        height: "100%",
                        width: "80",
                        overflow: "clip",
                        background: "rgb(30,30,35,0.7)",
                        shadow: "4 0 12 0 rgb(0, 0, 0, 50)",
                        layer: "-10",
                        backdrop_blur: "16",
                        ScrollView {
                            padding: "16",
                            spacing: "12",
                            height: "80%",
                            if let Some(ctx) = &ctx_option {
                                for link in make_links(&ctx.api_games, selected_game_id, navigator.clone()) {
                                    {link}
                                }
                            } else {
                                label {
                                    "Loading..."
                                }
                            }
                        }
                        rect {
                            height: "20%",
                            width: "100%",
                            direction: "vertical",
                            main_align: "center",
                            cross_align: "center",
                            spacing: "20",
                            padding: "14",

                            rect {
                                key: "settings",
                                onclick: move |_| {
                                    navigator.push(Route::Settings);
                                },
                                MySidebarItem {
                                    svg {
                                        width: "32",
                                        height: "32",
                                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white" xmlns="http://www.w3.org/2000/svg">
                                            <path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z"/>
                                        </svg>"#
                                    }
                                }
                            }

                            rect {
                                onclick: move |_| {
                                    println!("Elysia logo clicked");
                                },
                                MySidebarItem {
                                    image {
                                        width: "32",
                                        height: "32",
                                        sampling: "trilinear",
                                        image_data: static_bytes(include_bytes!("../../../assets/elysia.png"))
                                    }
                                }
                            }

                        }

                    }
                    rect {
                        overflow: "clip",
                        width: "fill",
                        height: "100%",

                        Body {
                            AnimatedOutlet { }
                        }
                    }
                }

            }
        }
    }
}
