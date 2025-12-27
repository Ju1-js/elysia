mod types;
mod polling;

pub use types::*;
use polling::{poll_setup, poll_download};

use freya::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct ComponentState {
    pub ready: Signal<bool>,
    pub needs_update: bool,
    pub active: Signal<bool>,
}

impl std::fmt::Debug for ComponentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentState")
            .field("ready", &"Signal<bool>")
            .field("needs_update", &self.needs_update)
            .field("active", &"Signal<bool>")
            .finish()
    }
}

#[derive(Props, Clone)]
pub struct DownloadControlProps {
    pub game_name: String,
    pub game_progress_key: String,
    pub installed: bool,
    pub runtime: ComponentState,
    pub tweaks: ComponentState,
    pub download_active: Signal<bool>,
    pub get_game_progress: Rc<dyn Fn(&str) -> Option<types::DownloadProgress>>,
    pub get_runtime_progress: Rc<dyn Fn(&str) -> Option<types::SetupProgress>>,
    pub get_tweaks_progress: Rc<dyn Fn(&str) -> Option<types::SetupProgress>>,
    #[props(default = "#ff9500".to_string())]
    pub accent_color: String,
    #[props(default)]
    pub on_setup_runtime: Option<EventHandler<PressEvent>>,
    #[props(default)]
    pub on_setup_tweaks: Option<EventHandler<PressEvent>>,
    #[props(default)]
    pub on_download_game: Option<EventHandler<PressEvent>>,
}

impl PartialEq for DownloadControlProps {
    fn eq(&self, other: &Self) -> bool {
        self.game_name == other.game_name
            && self.game_progress_key == other.game_progress_key
            && self.installed == other.installed
            && self.runtime == other.runtime
            && self.tweaks == other.tweaks
            && self.accent_color == other.accent_color
    }
}

#[component]
pub fn DownloadControl(props: DownloadControlProps) -> Element {
    let DownloadControlProps {
        game_name,
        game_progress_key,
        installed,
        runtime,
        tweaks,
        download_active,
        get_game_progress,
        get_runtime_progress,
        get_tweaks_progress,
        accent_color,
        on_setup_runtime,
        on_setup_tweaks,
        on_download_game,
    } = props;

    let ButtonTheme { font_theme, .. } = use_applied_theme!(&None, filled_button);

    let runtime_progress = use_signal(|| None::<types::SetupProgress>);
    let tweaks_progress = use_signal(|| None::<types::SetupProgress>);
    let game_progress = use_signal(|| None::<types::DownloadProgress>);
    let mut is_installed = use_signal(|| installed);

    use_effect(use_reactive!(|installed| {
        is_installed.set(installed);
    }));
    
    poll_setup(runtime.active, "runtime_setup", get_runtime_progress, runtime_progress, runtime.ready);
    poll_setup(tweaks.active, "tweaks_setup", get_tweaks_progress, tweaks_progress, tweaks.ready);
    poll_download(download_active, &game_progress_key, get_game_progress, game_progress, is_installed);

    let runtime_busy = runtime_progress.read().is_some();
    let tweaks_busy = tweaks_progress.read().is_some();
    let game_busy = game_progress.read().as_ref().map_or(false, |p| p.is_busy);

    rsx! {
        rect {
            width: "100%",
            direction: "vertical",
            spacing: "8",
            
            if let Some(setup) = runtime_progress.read().as_ref() {
                SetupWidget {
                    setup: setup.clone(),
                    title: "Runtime Setup",
                    accent: accent_color.clone(),
                    font: font_theme.clone(),
                }
            }
            
            if let Some(setup) = tweaks_progress.read().as_ref() {
                SetupWidget {
                    setup: setup.clone(),
                    title: "Tweaks Setup",
                    accent: accent_color.clone(),
                    font: font_theme.clone(),
                }
            }
            
            if let Some(progress) = game_progress.read().as_ref() {
                if progress.is_busy {
                    DownloadWidget {
                        progress: progress.clone(),
                        name: game_name.clone(),
                        accent: accent_color.clone(),
                        font: font_theme.clone(),
                    }
                }
            }

            ActionButton {
                runtime,
                tweaks,
                installed: *is_installed.read(),
                runtime_busy,
                tweaks_busy,
                game_busy,
                on_setup_runtime,
                on_setup_tweaks,
                on_download_game,
            }
        }
    }
}

#[component]
fn ActionButton(
    runtime: ComponentState,
    tweaks: ComponentState,
    installed: bool,
    runtime_busy: bool,
    tweaks_busy: bool,
    game_busy: bool,
    on_setup_runtime: Option<EventHandler<PressEvent>>,
    on_setup_tweaks: Option<EventHandler<PressEvent>>,
    on_download_game: Option<EventHandler<PressEvent>>,
) -> Element {
    let runtime_ready = *runtime.ready.read();
    let tweaks_ready = *tweaks.ready.read();
    
    if runtime_busy || tweaks_busy || game_busy {
        return rsx! {};
    }
    
    let (label, handler) = if !runtime_ready {
        ("Download Runtime Setup", on_setup_runtime)
    } else if runtime.needs_update {
        ("Update Runtime", on_setup_runtime)
    } else if !tweaks_ready {
        ("Download Tweaks", on_setup_tweaks)
    } else if tweaks.needs_update {
        ("Update Tweaks", on_setup_tweaks)
    } else if !installed {
        ("Download Game", on_download_game)
    } else {
        ("Start Game", on_download_game)
    };

    rsx! {
        crate::components::MyButton {
            onpress: handler,
            rect {
                direction: "horizontal",
                cross_align: "center",
                main_align: "center",
                label {
                    font_size: "16",
                    font_weight: "500",
                    "{label}"
                }
            }
        }
    }
}

#[component]
fn SetupWidget(setup: types::SetupProgress, title: &'static str, accent: String, font: FontTheme) -> Element {
    let step = format!("Step {} of {}: {}", 
        setup.current_step_index + 1, 
        setup.total_steps,
        setup.current_step.description()
    );
    
    rsx!(
        rect {
            width: "100%",
            padding: "16",
            corner_radius: "8",
            background: "rgb(35, 35, 40)",
            background_opacity: "0.6",
            border: "1 solid rgb(255, 255, 255, 0.15)",
            direction: "vertical",
            spacing: "10",
            shadow: "0 4 16 0 rgb(0, 0, 0, 80), 0 2 6 0 rgb(0, 0, 0, 50)",
            backdrop_blur: "16",
            
            label {
                color: "{font.color}",
                font_size: "14",
                font_weight: "700",
                "{title}"
            }
            label {
                color: "{font.color}",
                font_size: "13",
                "{step}"
            }
            
            if let Some(ref p) = setup.step_progress {
                ProgressBar {
                    progress: p.clone(),
                    accent,
                    font: font.clone(),
                }
            }
        }
    )
}

#[component]
fn DownloadWidget(progress: types::DownloadProgress, name: String, accent: String, font: FontTheme) -> Element {
    let pct = if progress.total > 0 {
        (progress.downloaded as f64 / progress.total as f64) * 100.0
    } else {
        0.0
    };
    
    let status = if progress.total > 0 && (progress.status.starts_with("Downloading") || progress.status.starts_with("Extracting")) {
        let dl_gb = progress.downloaded as f64 / 1_000_000_000.0;
        let total_gb = progress.total as f64 / 1_000_000_000.0;
        
        if progress.speed_mb_s > 0.0 {
            format!("{} - {:.2} GB / {:.2} GB - {:.2} MB/s", progress.status, dl_gb, total_gb, progress.speed_mb_s)
        } else {
            format!("{} - {:.2} GB / {:.2} GB", progress.status, dl_gb, total_gb)
        }
    } else {
        progress.status.clone()
    };

    rsx!(
        rect {
            width: "100%",
            padding: "16",
            corner_radius: "8",
            background: "rgb(35, 35, 40)",
            background_opacity: "0.6",
            border: "1 solid rgb(255, 255, 255, 0.15)",
            direction: "vertical",
            spacing: "10",
            shadow: "0 4 16 0 rgb(0, 0, 0, 80), 0 2 6 0 rgb(0, 0, 0, 50)",
            backdrop_blur: "16",
            
            label {
                color: "{font.color}",
                font_size: "14",
                font_weight: "700",
                "{name}"
            }
            
            rect {
                width: "100%",
                height: "6",
                background: "rgb(40,40,40)",
                corner_radius: "3",
                overflow: "clip",
                rect {
                    width: "{pct:.0}%",
                    height: "6",
                    background: "{accent}",
                    corner_radius: "3",
                }
            }
            
            rect {
                width: "100%",
                direction: "horizontal",
                main_align: "space-between",
                cross_align: "center",
                label {
                    color: "{font.color}",
                    font_size: "14",
                    "{status}"
                }
                label {
                    color: "{font.color}",
                    font_size: "14",
                    font_weight: "600",
                    "{pct:.1}%"
                }
            }
        }
    )
}

#[component]
fn ProgressBar(progress: types::DownloadProgress, accent: String, font: FontTheme) -> Element {
    let pct = if progress.total > 0 {
        (progress.downloaded as f64 / progress.total as f64) * 100.0
    } else {
        0.0
    };

    rsx!(
        rect {
            width: "100%",
            height: "6",
            background: "rgb(40,40,40)",
            corner_radius: "3",
            overflow: "clip",
            rect {
                width: "{pct:.0}%",
                height: "6",
                background: "{accent}",
                corner_radius: "3",
            }
        }
        rect {
            width: "100%",
            direction: "horizontal",
            main_align: "space-between",
            cross_align: "center",
            label {
                color: "{font.color}",
                font_size: "12",
                "{progress.status}"
            }
            label {
                color: "{font.color}",
                font_size: "12",
                font_weight: "600",
                "{pct:.1}%"
            }
        }
    )
}
