use std::sync::{Arc, RwLock};
use std::rc::Rc;
use freya::prelude::*;
use reqwest::Url;

use crate::{
    components::{
        DownloadControl, 
        DownloadProgress, 
        MyButton, 
        MyNetworkImage, 
        MyNewsWidget,
        VideoBackgroundPlayer,
    },
    pages::GameSettings,
    context::Context,
};
use backend::{
    settings::GlobalSettings,
    game_providers::{
        installer::InstallerManager,
        hoyoplay::{get_video_url, get_theme_url},
    },
    runners::Runner,
};

fn create_fade_animation(duration_ms: u64, ease: Ease, function: Function) -> UseAnimation<AnimNum> {
    use_animation(move |_| {
        AnimNum::new(0.0, 1.0)
            .time(duration_ms)
            .ease(ease)
            .function(function)
    })
}

fn get_animation_progress(anim: &UseAnimation<AnimNum>, default_value: f64) -> f64 {
    if anim.is_running() {
        anim.get().read().read() as f64
    } else {
        default_value
    }
}

struct VideoFadeController {
    loaded: Signal<bool>,
    prev_url: Signal<Option<String>>,
    prev_page: Signal<&'static str>,
    animation: UseAnimation<AnimNum>,
}

impl VideoFadeController {
    fn get_opacity(&self) -> f64 {
        if *self.loaded.read() {
            get_animation_progress(&self.animation, 1.0)
        } else {
            0.0
        }
    }
    
    fn mark_ready(&mut self) {
        self.loaded.set(true);
        self.animation.start();
    }
}

fn use_video_fade(initial_url: Option<String>, initial_page: &'static str) -> VideoFadeController {
    VideoFadeController {
        loaded: use_signal(|| false),
        prev_url: use_signal(|| initial_url),
        prev_page: use_signal(|| initial_page),
        animation: create_fade_animation(800, Ease::InOut, Function::Cubic),
    }
}

struct PageTransitionController {
    current: Signal<&'static str>,
    target: Signal<&'static str>,
    animation: UseAnimation<AnimNum>,
}

impl PageTransitionController {
    fn get_opacity(&self) -> f64 {
        if self.animation.is_running() {
            let progress = self.animation.get().read().read() as f64;
            if progress < 0.5 {
                1.0 - (progress * 2.0)
            } else {
                (progress - 0.5) * 2.0
            }
        } else {
            1.0
        }
    }
    
    fn trigger_transition(&mut self) {
        if *self.target.read() != *self.current.read() && !self.animation.is_running() {
            self.animation.start();
        }
    }
    
    fn update_current_page(&mut self) {
        if self.animation.is_running() {
            let progress = self.animation.get().read().read() as f64;
            if progress >= 0.5 && *self.current.read() != *self.target.read() {
                self.current.set(*self.target.read());
            }
        }
    }
}

fn use_page_transition(initial_page: &'static str) -> PageTransitionController {
    PageTransitionController {
        current: use_signal(|| initial_page),
        target: use_signal(|| initial_page),
        animation: create_fade_animation(400, Ease::InOut, Function::Quad),
    }
}

#[derive(Clone, PartialEq)]
struct CrossfadeState {
    prev_url: Url,
    curr_url: Url,
    fade_progress: f64,
}

fn use_crossfade_background(url: Url) -> CrossfadeState {
    let mut prev = use_signal(|| url.clone());
    let mut curr = use_signal(|| url.clone());
    
    let anim = create_fade_animation(700, Ease::InOut, Function::Cubic);

    let curr_url_str = curr.read().to_string();
    let new_url_str = url.to_string();
    
    if curr_url_str != new_url_str {
        prev.set(curr.read().clone());
        curr.set(url.clone());
        anim.start();
    }

    let fade_progress = get_animation_progress(&anim, 1.0);

    CrossfadeState {
        prev_url: prev.read().clone(),
        curr_url: curr.read().clone(),
        fade_progress,
    }
}

fn create_progress_getter(
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> (String, Rc<dyn Fn(&str) -> Option<DownloadProgress>>) {
    let progress_key = format!("{}_streaming", game_id);
    
    let get_progress_fn = Rc::new(move |key: &str| {
        let settings = settings_sig.read();
        let s = settings.read().ok()?;
        
        let installer = InstallerManager::create_installer(
            &game_id,
            &biz,
            s.temp_directory.clone(),
            s.components_directory.clone(),
        )?;
        
        installer.get_progress(key).map(|p| DownloadProgress {
            downloaded: p.downloaded,
            total: p.total,
            mb_s: p.mb_s,
            part_index: p.part_index,
            parts_total: p.parts_total,
            status: p.status,
            is_busy: p.is_busy,
        })
    });

    (progress_key, get_progress_fn)
}

fn create_game_action_handler(
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: String,
    biz: String,
) -> EventHandler<PressEvent> {
    EventHandler::new(move |_| {
        let settings_arc = settings_sig.read().clone();
        
        let installer = {
            let settings = match settings_arc.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            
            if let Some(installed_game) = settings.installed_games.get(&game_id) {
                if let Err(e) = installed_game.runner.run_game(&settings, installed_game) {
                    eprintln!("Failed to run game: {}", e);
                }
                return;
            }

            let inst = InstallerManager::create_installer(
                &game_id,
                &biz,
                settings.temp_directory.clone(),
                settings.components_directory.clone(),
            );

            if let Some(ref installer) = inst {
                if installer.get_progress(&installer.progress_key()).map_or(false, |p| p.is_busy) {
                    eprintln!("Installation already in progress");
                    return;
                }
            }
            
            inst
        };

        if let Some(inst) = installer {
            let game_id_clone = game_id.clone();
            let mut settings_sig_mut = settings_sig;

            spawn(async move {
                match inst.install().await {
                    Ok(installed_game) => {
                        if let Ok(mut settings) = settings_arc.write() {
                            settings.installed_games.insert(game_id_clone, installed_game);
                            if let Err(e) = settings.save() {
                                eprintln!("Failed to save settings: {}", e);
                                return;
                            }

                            drop(settings);
                            let new_settings = settings_arc.read().unwrap().clone();
                            settings_sig_mut.set(Arc::new(RwLock::new(new_settings)));
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to install game: {}", e);
                    }
                }
            });
        }
    })
}

fn check_game_installed(
    settings_sig: &Signal<Arc<RwLock<GlobalSettings>>>,
    game_id: &str,
    biz: &str,
) -> bool {
    let settings = settings_sig.read();
    if let Ok(s) = settings.read() {
        InstallerManager::is_game_installed(
            &s,
            game_id,
            biz,
            s.temp_directory.clone(),
            s.components_directory.clone(),
        )
    } else {
        false
    }
}

#[component]
fn BackgroundLayers(
    crossfade: CrossfadeState,
    video_url: Option<String>,
    video_opacity: f64,
    theme_url: Option<String>,
    static_bg_url: Url,
    on_video_ready: EventHandler<()>,
) -> Element {
    rsx! {
        // Static background layer
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            main_align: "start",
            cross_align: "start",
            layer: "4",
            MyNetworkImage {
                url: static_bg_url,
                sampling: "trilinear",
            }
        }

        // Previous crossfade image
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            main_align: "end",
            cross_align: "end",
            layer: "3",
            opacity: "{1.0 - crossfade.fade_progress}",
            MyNetworkImage {
                url: crossfade.prev_url,
                sampling: "trilinear",
            }
        }

        // Current crossfade image
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            main_align: "end",
            cross_align: "end",
            layer: "3",
            opacity: "{crossfade.fade_progress * (1.0 - video_opacity)}",
            MyNetworkImage {
                url: crossfade.curr_url,
                sampling: "trilinear",
            }
        }

        // Video layer
        if let Some(video) = video_url {
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                layer: "2",
                opacity: "{video_opacity}",
                VideoBackgroundPlayer {
                    key: "{video}",
                    video_url: video,
                    on_ready: on_video_ready,
                }
            }
        }

        // Theme layer
        if let Some(theme) = theme_url {
            if let Ok(theme_url_parsed) = theme.parse::<Url>() {
                rect {
                    position: "absolute",
                    position_top: "0",
                    position_left: "0",
                    width: "100%",
                    height: "100%",
                    layer: "1",
                    MyNetworkImage {
                        url: theme_url_parsed,
                        sampling: "trilinear",
                    }
                }
            }
        }
    }
}

#[component]
fn TopRightButtons() -> Element {
    rsx! {
        rect {
            position: "absolute",
            position_top: "32",
            position_left: "0",
            width: "100%",
            height: "0",
            direction: "horizontal",
            main_align: "end",
            padding: "0 32",
            layer: "-1",
            
            MyButton {
                onpress: move |_| println!("Meow clicked!"),
                label {
                    font_size: "16",
                    font_weight: "500",
                    color: "white",
                    "Meow 🐾"
                }
            }
        }
    }
}

#[component]
fn BottomRightButtons(
    target_page: Signal<&'static str>,
) -> Element {
    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            direction: "horizontal",
            main_align: "end",
            cross_align: "end",
            spacing: "12",
            padding: "32",
            layer: "-1",
            
            MyButton {
                onpress: move |_| println!("Game tracker clicked!"),
                rect {
                    direction: "horizontal",
                    cross_align: "center",
                    spacing: "8",
                    svg {
                        width: "20",
                        height: "20",
                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10 10-4.5 10-10S17.5 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm.5-13H11v6l5.2 3.2.8-1.3-4.5-2.7V7z"/></svg>"#
                    }
                    label {
                        font_size: "16",
                        font_weight: "500",
                        color: "white",
                        "2h 34m"
                    }
                }
            }
            
            MyButton {
                onpress: move |_| target_page.set("settings"),
                rect {
                    direction: "horizontal",
                    cross_align: "center",
                    spacing: "8",
                    svg {
                        width: "20",
                        height: "20",
                        svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z"/></svg>"#
                    }
                    label {
                        font_size: "16",
                        font_weight: "500",
                        color: "white",
                        "Game Settings"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Game() -> Element {
    let selected_game_id = use_context::<Signal<Option<String>>>();
    let ctx = use_context::<Context>();
    let settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    
    let mut prev_game_id = use_signal(|| None::<String>);
    let mut page_ctrl = use_page_transition("game");
    
    let current_game_id = selected_game_id.read().clone();
    if prev_game_id.read().as_ref() != current_game_id.as_ref() {
        page_ctrl.current.set("game");
        page_ctrl.target.set("game");
        prev_game_id.set(current_game_id.clone());
    }

    page_ctrl.trigger_transition();
    page_ctrl.update_current_page();
    let page_opacity = page_ctrl.get_opacity();
 
    let game_id = selected_game_id.read();
    let Some(ref game_id_str) = *game_id else {
        return rsx! { rect { width: "fill", height: "fill" } };
    };

    let Some(game_data) = ctx.api_games.iter().find(|g| &g.id == game_id_str).cloned() else {
        return rsx! { 
            rect { 
                width: "fill",
                height: "fill",
                label { "Game not found" }
            }
        };
    };

    let bg_url = use_memo(move || {
        let game_id = selected_game_id.read();
        game_id.as_ref().and_then(|id| {
            ctx.api_games.iter()
                .find(|g| &g.id == id)
                .and_then(|g| g.display.background.url.parse::<Url>().ok())
        })
    });

    let Some(parsed_bg_url) = bg_url.read().clone() else {
        return rsx! {
            rect {
                width: "fill",
                height: "fill",
                label { "Invalid background image URL" }
            }
        };
    };

    let (video_url, theme_url) = use_memo(move || {
        let game_id = selected_game_id.read();
        game_id.as_ref().and_then(|id| {
            ctx.api_game_basic_info
                .iter()
                .find(|info| info.game.id == *id)
                .map(|info| {
                    let video = get_video_url(&info.backgrounds);
                    let theme = get_theme_url(&info.backgrounds);
                    (video, theme)
                })
        })
    }).read().clone().unwrap_or((None, None));

    let mut video_ctrl = use_video_fade(video_url.clone(), *page_ctrl.current.read());

    if video_ctrl.prev_url.read().as_ref() != video_url.as_ref() {
        video_ctrl.loaded.set(false);
        video_ctrl.prev_url.set(video_url.clone());
    }

    // Reset video when returning from GameSettings
    if *video_ctrl.prev_page.read() != *page_ctrl.current.read() {
        video_ctrl.prev_page.set(*page_ctrl.current.read());
        if *page_ctrl.current.read() == "game" {
            video_ctrl.loaded.set(false);
        }
    }
    
    let video_opacity = video_ctrl.get_opacity();

    let mut news_carousel_index = use_signal(|| 0usize);
    let game_id_for_effect = game_id_str.clone();
    use_effect(use_reactive!(|game_id_for_effect| {
        let _ = game_id_for_effect;
        news_carousel_index.set(0);
    }));

    // React to settings changes to detect installation completion
    let game_id_for_install_check = game_data.id.clone();
    let biz_for_install_check = game_data.biz.clone();
    let is_installed = use_memo(use_reactive!(|settings_sig, game_id_for_install_check, biz_for_install_check| {
        check_game_installed(&settings_sig, &game_id_for_install_check, &biz_for_install_check)
    }));
    
    let (progress_key, get_progress_fn) = create_progress_getter(
        settings_sig,
        game_data.id.clone(),
        game_data.biz.clone()
    );
    let onpress = create_game_action_handler(
        settings_sig,
        game_data.id.clone(),
        game_data.biz.clone()
    );
    
    let crossfade = use_crossfade_background(parsed_bg_url.clone());

    if *page_ctrl.current.read() != "game" {
        return rsx! {
            GameSettings {
                on_back: move |_| page_ctrl.target.set("game"),
                background_url: crossfade.curr_url.clone(),
                game_name: game_data.display.name.clone(),
            }
        };
    }

    rsx! {
        rect {
            width: "fill",
            height: "fill",

            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                opacity: "{page_opacity}",
                BackgroundLayers {
                    crossfade: crossfade,
                    video_url: video_url,
                    video_opacity: video_opacity,
                    theme_url: theme_url,
                    static_bg_url: parsed_bg_url,
                    on_video_ready: move |_| {
                        video_ctrl.mark_ready();
                    },
                }
            }

            TopRightButtons {}
            
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "92",
                width: "550",
                height: "100%",
                direction: "vertical",
                main_align: "end",
                cross_align: "start",
                padding: "40",
                spacing: "20",
                layer: "-1",
                
                rect {
                    width: "450",
                    MyNewsWidget {
                        game_id: game_data.id.clone(),
                        carousel_index: news_carousel_index,
                    }
                }
                
                DownloadControl {
                    game_id: game_data.id.clone(),
                    progress_key: progress_key,
                    installed: *is_installed.read(),
                    get_progress: get_progress_fn,
                    accent_color: "#ff9500".to_string(),
                    onpress: onpress,
                }
            }
                
            BottomRightButtons {
                target_page: page_ctrl.target,
            }
        }
    }
}