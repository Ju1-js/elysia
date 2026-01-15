mod polling;
mod types;

use polling::{poll_download, poll_runtime_setup, poll_tweaks_setup};
pub use types::*;

use crate::theme;
use freya::prelude::*;
use std::rc::Rc;

type GameProgressGetter = Rc<dyn Fn(&str) -> Option<types::DownloadProgress>>;
type RuntimeProgressGetter = Rc<dyn Fn(&str) -> Option<types::SetupProgress>>;
type TweaksProgressGetter = Rc<dyn Fn(&str) -> Option<types::SetupProgress>>;

#[derive(Props)]
pub struct DownloadControlProps {
    pub game_id: String,
    pub game_name: String,
    pub game_progress_key: String,
    pub get_game_progress: GameProgressGetter,
    pub get_runtime_progress: RuntimeProgressGetter,
    pub get_tweaks_progress: TweaksProgressGetter,
    #[props(default = theme::ACCENT_PRIMARY.to_string())]
    pub accent_color: String,
    #[props(default)]
    pub on_setup_runtime: Option<EventHandler<PressEvent>>,
    #[props(default)]
    pub on_setup_tweaks: Option<EventHandler<PressEvent>>,
    #[props(default)]
    pub on_download_game: Option<EventHandler<PressEvent>>,
    pub game_state: crate::pages::game::state::GlobalGameStateSignal,
    pub game_needs_tweaks: bool,
}

impl PartialEq for DownloadControlProps {
    fn eq(&self, other: &Self) -> bool {
        self.game_id == other.game_id
            && self.game_name == other.game_name
            && self.game_progress_key == other.game_progress_key
            && self.accent_color == other.accent_color
            && self.game_needs_tweaks == other.game_needs_tweaks
    }
}

impl Clone for DownloadControlProps {
    fn clone(&self) -> Self {
        Self {
            game_id: self.game_id.clone(),
            game_name: self.game_name.clone(),
            game_progress_key: self.game_progress_key.clone(),
            get_game_progress: self.get_game_progress.clone(),
            get_runtime_progress: self.get_runtime_progress.clone(),
            get_tweaks_progress: self.get_tweaks_progress.clone(),
            accent_color: self.accent_color.clone(),
            on_setup_runtime: self.on_setup_runtime,
            on_setup_tweaks: self.on_setup_tweaks,
            on_download_game: self.on_download_game,
            game_state: self.game_state,
            game_needs_tweaks: self.game_needs_tweaks,
        }
    }
}

#[component]
pub fn DownloadControl(props: DownloadControlProps) -> Element {
    let DownloadControlProps {
        game_id,
        game_name,
        game_progress_key,
        get_game_progress,
        get_runtime_progress,
        get_tweaks_progress,
        accent_color,
        on_setup_runtime,
        on_setup_tweaks,
        on_download_game,
        game_state,
        game_needs_tweaks,
    } = props;

    let ButtonTheme { font_theme, .. } = use_applied_theme!(&None, filled_button);

    // Local display signals for progress widgets
    let runtime_progress = use_signal(|| None::<types::SetupProgress>);
    let tweaks_progress = use_signal(|| None::<types::SetupProgress>);
    let game_progress = use_signal(|| None::<types::DownloadProgress>);

    let runtime_active = use_signal(|| false); // Dummy signal for polling (we poll unconditionally now)

    let runtime_ready = use_memo(use_reactive!(|game_state| {
        let ready = game_state.read().is_runtime_ready();
        eprintln!("[DownloadControl] runtime_ready: {ready}");
        ready
    }));

    let tweaks_active = use_memo(use_reactive!(|game_state, game_id| {
        game_state.read().get_tweaks_state(&game_id).active
    }));

    let tweaks_ready = use_memo(use_reactive!(|game_state, game_id, game_needs_tweaks| {
        if !game_needs_tweaks {
            return true;
        }

        game_state.read().get_tweaks_state(&game_id).ready
    }));

    let is_downloading = use_memo(use_reactive!(|game_state, game_id| {
        game_state.read().get_download_state(&game_id).active
    }));

    let is_installed = use_memo(use_reactive!(|game_state, game_id| {
        game_state.read().get_download_state(&game_id).installed
    }));

    // Convert memos to signals for polling functions
    let runtime_active_signal = runtime_active;
    let mut tweaks_active_signal = use_signal(move || *tweaks_active.read());
    let mut is_downloading_signal = use_signal(move || *is_downloading.read());

    use_effect(use_reactive!(|tweaks_active| {
        tweaks_active_signal.set(*tweaks_active.read());
    }));

    use_effect(use_reactive!(|is_downloading| {
        is_downloading_signal.set(*is_downloading.read());
    }));

    // Poll for progress - runtime polls unconditionally now
    poll_runtime_setup(
        runtime_active_signal,
        "runtime_setup",
        get_runtime_progress,
        runtime_progress,
        game_state,
    );

    poll_tweaks_setup(
        tweaks_active_signal,
        "tweaks_setup",
        game_id.clone(),
        get_tweaks_progress,
        tweaks_progress,
        game_state,
    );

    poll_download(
        is_downloading_signal,
        &game_progress_key,
        get_game_progress,
        game_progress,
        game_state,
        game_id.clone(),
    );

    let runtime_busy = runtime_progress.read().is_some();
    let tweaks_busy = tweaks_progress.read().is_some();
    let game_busy = game_progress.read().as_ref().is_some_and(|p| p.is_busy);

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

            if game_needs_tweaks {
                if let Some(setup) = tweaks_progress.read().as_ref() {
                    SetupWidget {
                        setup: setup.clone(),
                        title: "Tweaks Setup",
                        accent: accent_color.clone(),
                        font: font_theme.clone(),
                    }
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
                runtime_ready: *runtime_ready.read(),
                tweaks_ready: *tweaks_ready.read(),
                installed: *is_installed.read(),
                runtime_busy,
                tweaks_busy,
                game_busy,
                game_needs_tweaks,
                missing_components: game_state.read().get_missing_components(),
                on_setup_runtime,
                on_setup_tweaks,
                on_download_game,
            }
        }
    }
}

#[component]
fn ActionButton(
    runtime_ready: bool,
    tweaks_ready: bool,
    installed: bool,
    runtime_busy: bool,
    tweaks_busy: bool,
    game_busy: bool,
    game_needs_tweaks: bool,
    missing_components: Vec<&'static str>,
    on_setup_runtime: Option<EventHandler<PressEvent>>,
    on_setup_tweaks: Option<EventHandler<PressEvent>>,
    on_download_game: Option<EventHandler<PressEvent>>,
) -> Element {
    let game_state = use_context::<crate::pages::game::state::GlobalGameStateSignal>();
    let game_running = game_state.read().is_game_running();
    
    // Don't show button while any operation is in progress
    if runtime_busy || tweaks_busy || game_busy {
        return rsx! {};
    }

    // Determine button LABEL based on current state
    let label = if game_running {
        "Kill Game".to_string()
    } else if !runtime_ready {
        if missing_components.is_empty() {
            "Download Runtime Setup".to_string()
        } else {
            format!("Download {}", missing_components.join(" & "))
        }
    } else if game_needs_tweaks && !tweaks_ready {
        "Download Tweaks".to_string()
    } else if !installed {
        "Download Game".to_string()
    } else {
        "Start Game".to_string()
    };

    // Create ONE dynamic handler that checks state when clicked
    let dynamic_handler = EventHandler::new(move |evt| {
        let mut state = game_state;
        let is_running = state.read().is_game_running();
        
        eprintln!("[ActionButton Click] game_running: {is_running}");
        
        if is_running {
            // Kill game
            match state.write().kill_game() {
                Ok(()) => eprintln!("[ActionButton] Game process killed successfully"),
                Err(e) => eprintln!("[ActionButton] Failed to kill game: {e}"),
            }
        } else if !runtime_ready {
            // Setup runtime
            if let Some(handler) = on_setup_runtime {
                handler.call(evt);
            }
        } else if game_needs_tweaks && !tweaks_ready {
            // Setup tweaks
            if let Some(handler) = on_setup_tweaks {
                handler.call(evt);
            }
        } else {
            // Download or start game
            if let Some(handler) = on_download_game {
                handler.call(evt);
            }
        }
    });

    rsx! {
        crate::components::MyButton {
            onpress: dynamic_handler,
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
fn SetupWidget(
    setup: types::SetupProgress,
    title: &'static str,
    accent: String,
    font: FontTheme,
) -> Element {
    // Top label shows generic step description (e.g., "Downloading Wine")
    let generic_step = setup.current_step.description();

    // Format step counter
    let step = format!(
        "Step {} of {}",
        setup.current_step_index + 1,
        setup.total_steps
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
                "{generic_step}"
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
fn DownloadWidget(
    progress: types::DownloadProgress,
    name: String,
    accent: String,
    font: FontTheme,
) -> Element {
    #[allow(clippy::cast_precision_loss)]
    let pct = if progress.total > 0 {
        (progress.downloaded as f64 / progress.total as f64) * 100.0
    } else {
        0.0
    };

    let status = if progress.total > 0
        && (progress.status.starts_with("Downloading") || progress.status.starts_with("Extracting"))
    {
        #[allow(clippy::cast_precision_loss)]
        let dl_gb = progress.downloaded as f64 / 1_000_000_000.0;
        #[allow(clippy::cast_precision_loss)]
        let total_gb = progress.total as f64 / 1_000_000_000.0;

        if progress.speed_mb_s > 0.0 {
            format!(
                "{} - {:.2} GB / {:.2} GB - {:.2} MB/s",
                progress.status, dl_gb, total_gb, progress.speed_mb_s
            )
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
    #[allow(clippy::cast_precision_loss)]
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
