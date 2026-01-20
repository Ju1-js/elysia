mod polling;
mod types;

use polling::{poll_download, poll_runtime_setup, poll_tweaks_setup};
pub use types::*;

use crate::theme;
use freya::prelude::*;
use std::rc::Rc;
use std::path::PathBuf;

type GameProgressGetter = Rc<dyn Fn(&str) -> Option<types::DownloadProgress>>;
type RuntimeProgressGetter = Rc<dyn Fn(&str) -> Option<types::SetupProgress>>;
type TweaksProgressGetter = Rc<dyn Fn(&str) -> Option<types::SetupProgress>>;

#[derive(Props)]
pub struct DownloadControlProps {
    pub game_id: String,
    pub game_name: String,
    pub game_biz: String,
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
    #[props(default)]
    pub on_show_install_modal: Option<EventHandler<()>>,
    #[props(default)]
    pub on_show_import_modal: Option<EventHandler<()>>,
    pub game_state: crate::pages::game::state::GlobalGameStateSignal,
    pub game_needs_tweaks: bool,
    #[props(default = false)]
    pub tweaks_needs_update: bool,
    #[props(default = false)]
    pub runtime_needs_update: bool,
}

impl PartialEq for DownloadControlProps {
    fn eq(&self, other: &Self) -> bool {
        self.game_id == other.game_id
            && self.game_name == other.game_name
            && self.game_biz == other.game_biz
            && self.game_progress_key == other.game_progress_key
            && self.accent_color == other.accent_color
            && self.game_needs_tweaks == other.game_needs_tweaks
            && self.tweaks_needs_update == other.tweaks_needs_update
            && self.runtime_needs_update == other.runtime_needs_update
    }
}

impl Clone for DownloadControlProps {
    fn clone(&self) -> Self {
        Self {
            game_id: self.game_id.clone(),
            game_name: self.game_name.clone(),
            game_biz: self.game_biz.clone(),
            game_progress_key: self.game_progress_key.clone(),
            get_game_progress: self.get_game_progress.clone(),
            get_runtime_progress: self.get_runtime_progress.clone(),
            get_tweaks_progress: self.get_tweaks_progress.clone(),
            accent_color: self.accent_color.clone(),
            on_setup_runtime: self.on_setup_runtime,
            on_setup_tweaks: self.on_setup_tweaks,
            on_download_game: self.on_download_game,
            on_show_install_modal: self.on_show_install_modal,
            on_show_import_modal: self.on_show_import_modal,
            game_state: self.game_state,
            game_needs_tweaks: self.game_needs_tweaks,
            tweaks_needs_update: self.tweaks_needs_update,
            runtime_needs_update: self.runtime_needs_update,
        }
    }
}

#[component]
pub fn DownloadControl(props: DownloadControlProps) -> Element {
    let DownloadControlProps {
        game_id,
        game_name,
        game_biz,
        game_progress_key,
        get_game_progress,
        get_runtime_progress,
        get_tweaks_progress,
        accent_color,
        on_setup_runtime,
        on_setup_tweaks,
        on_download_game,
        on_show_install_modal,
        on_show_import_modal,
        game_state,
        game_needs_tweaks,
        tweaks_needs_update,
        runtime_needs_update,
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
                tweaks_needs_update,
                runtime_needs_update,
                missing_components: game_state.read().get_missing_components(),
                on_setup_runtime,
                on_setup_tweaks,
                on_download_game,
                on_show_install_modal,
                on_show_import_modal,
                game_id: game_id.clone(),
                game_name: game_name.clone(),
                game_biz: game_biz.clone(),
                accent_color,
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
    tweaks_needs_update: bool,
    runtime_needs_update: bool,
    missing_components: Vec<&'static str>,
    on_setup_runtime: Option<EventHandler<PressEvent>>,
    on_setup_tweaks: Option<EventHandler<PressEvent>>,
    on_download_game: Option<EventHandler<PressEvent>>,
    on_show_install_modal: Option<EventHandler<()>>,
    on_show_import_modal: Option<EventHandler<()>>,
    game_id: String,
    game_name: String,
    game_biz: String,
    accent_color: String,
) -> Element {
    let game_state = use_context::<crate::pages::game::state::GlobalGameStateSignal>();
    let settings = use_context::<Signal<std::sync::Arc<std::sync::RwLock<backend::settings::GlobalSettings>>>>();
    let game_running = game_state.read().is_game_running();
    
    // Clone game_id for use in the closure
    let game_id_for_closure = game_id.clone();
    
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
    } else if runtime_ready && runtime_needs_update {
        "Update Runtime".to_string()
    } else if game_needs_tweaks && !tweaks_ready {
        "Download Tweaks".to_string()
    } else if game_needs_tweaks && tweaks_needs_update {
        "Update Tweaks".to_string()
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
        eprintln!("[ActionButton Click] runtime_ready: {runtime_ready}, tweaks_ready: {tweaks_ready}, installed: {installed}, game_needs_tweaks: {game_needs_tweaks}");
        
        if is_running {
            // Calculate and save playtime before killing game
            let elapsed_seconds = state.read().get_elapsed_playtime();
            eprintln!("[ActionButton] Game played for {elapsed_seconds} seconds, saving playtime...");
            
            // Save playtime to game_preferences
            let settings_arc = settings.read().clone();
            if let Ok(mut settings_guard) = settings_arc.write() {
                // Get or create game preferences for this game
                let prefs = settings_guard.game_preferences
                    .entry(game_id_for_closure.clone())
                    .or_insert_with(backend::settings::GamePreferences::default);
                
                prefs.playtime_seconds += elapsed_seconds;
                eprintln!("[ActionButton] Total playtime for {}: {} seconds", game_id_for_closure, prefs.playtime_seconds);
                
                // Save settings to persist playtime
                if let Err(e) = settings_guard.save() {
                    eprintln!("[ActionButton] Failed to save playtime: {e}");
                }
            }
            
            // Kill game
            match state.write().kill_game() {
                Ok(()) => eprintln!("[ActionButton] Game process killed successfully"),
                Err(e) => eprintln!("[ActionButton] Failed to kill game: {e}"),
            }
        } else if !runtime_ready {
            // Setup runtime
            eprintln!("[ActionButton] Calling setup runtime handler");
            if let Some(handler) = on_setup_runtime {
                handler.call(evt);
            } else {
                eprintln!("[ActionButton] No runtime setup handler available");
            }
        } else if runtime_ready && runtime_needs_update {
            // Update runtime
            eprintln!("[ActionButton] Calling update runtime handler");
            if let Some(handler) = on_setup_runtime {
                handler.call(evt);
            } else {
                eprintln!("[ActionButton] No runtime setup handler available");
            }
        } else if game_needs_tweaks && !tweaks_ready {
            // Setup tweaks (download for first time)
            eprintln!("[ActionButton] Calling setup tweaks handler");
            if let Some(handler) = on_setup_tweaks {
                handler.call(evt);
            } else {
                eprintln!("[ActionButton] No tweaks setup handler available");
            }
        } else if game_needs_tweaks && tweaks_needs_update {
            // Update tweaks (download new version)
            eprintln!("[ActionButton] Calling update tweaks handler");
            if let Some(handler) = on_setup_tweaks {
                handler.call(evt);
            } else {
                eprintln!("[ActionButton] No tweaks setup handler available");
            }
        } else if !installed {
            // Show modal for installation directory selection
            eprintln!("[ActionButton] Showing install modal");
            if let Some(handler) = on_show_install_modal {
                handler.call(());
            }
        } else {
            // Download or start game
            eprintln!("[ActionButton] Calling download/start game handler");
            if let Some(handler) = on_download_game {
                handler.call(evt);
            } else {
                eprintln!("[ActionButton] No installer implemented for this game");
            }
        }
    });

    rsx! {
        rect {
            width: "100%",
            direction: "vertical",
            spacing: "8",
            
            rect {
                width: "100%",
                padding: "0 0 0 12",  // Add left padding to shift button right
                
                crate::components::MyButton {
                    onpress: dynamic_handler,
                    rect {
                        direction: "horizontal",
                        cross_align: "center",
                        main_align: "center",
                        padding: "4 8",
                        label {
                            font_size: "18",
                            font_weight: "600",
                            "{label}"
                        }
                    }
                }
            }

            // DirectX11 checkbox - only show when game is installed and ready to launch
            if runtime_ready && (!game_needs_tweaks || tweaks_ready) && installed && !game_running {
                DirectX11Checkbox {
                    game_id: game_id.clone(),
                    settings,
                }
            }

            // Show import link when in "download game" state
            if runtime_ready && (!game_needs_tweaks || tweaks_ready) && !installed {
                rect {
                    width: "100%",
                    main_align: "center",
                    
                    rect {
                        direction: "horizontal",
                        spacing: "3",
                        cross_align: "center",
                        main_align: "center",
                        padding: "8 25",
                        corner_radius: "6",
                        background: "rgb(35, 35, 40)",
                        background_opacity: "0.6",
                        border: "1 inner rgb(255, 255, 255, 0.15)",
                        shadow: "0 2 8 0 rgb(0, 0, 0, 0.5)",
                        backdrop_blur: "12",
                        onclick: move |_| {
                            if let Some(handler) = on_show_import_modal {
                                handler.call(());
                            }
                        },
                        
                        label {
                            font_size: "13",
                            color: "white",
                            "Already installed?"
                        }
                        
                        label {
                            font_size: "13",
                            color: "{accent_color}",
                            font_weight: "600",
                            " Import here!"
                        }
                    }
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

#[component]
pub fn InstallDirectoryModal(
    on_close: EventHandler<()>,
    on_confirm: EventHandler<Option<PathBuf>>,
    settings: Signal<std::sync::Arc<std::sync::RwLock<backend::settings::GlobalSettings>>>,
    game_name: String,
    game_biz: String,
) -> Element {
    let default_path = {
        let settings_guard = settings.read();
        if let Ok(s) = settings_guard.read() {
            s.games_directory.join(&game_name).display().to_string()
        } else {
            format!("~/.local/share/elysia/games/{game_name}")
        }
    };

    let current_path = use_signal(|| default_path.clone());
    let mut error_message = use_signal(|| Option::<String>::None);

    let on_browse = move |_| {
        let mut path_clone = current_path;
        spawn(async move {
            let current_dir = path_clone.read().clone();
            if let Some(folder) = rfd::AsyncFileDialog::new()
                .set_title("Select Game Installation Directory")
                .set_directory(&current_dir)
                .pick_folder()
                .await
            {
                path_clone.set(folder.path().display().to_string());
            }
        });
    };
    
    let on_confirm_click = move |_| {
        // Check if an installer exists for this game
        let settings_guard = settings.read();
        if let Ok(s) = settings_guard.read() {
            let installer = backend::game_providers::installer::InstallerManager::create_installer(
                &game_name,
                &game_biz,
                s.temp_directory.clone(),
                s.games_directory.clone(),
            );
            
            if installer.is_none() {
                // No installer implemented
                error_message.set(Some(format!("No installer implemented for {game_name}")));
                eprintln!("[InstallDirectoryModal] No installer implemented for game: {game_name} (biz: {game_biz})");
                return;
            }
        }
        
        // Installer exists, proceed with confirmation
        let path_str = current_path.read().clone();
        let custom_path = if path_str == default_path {
            None
        } else {
            Some(PathBuf::from(path_str))
        };
        on_confirm.call(custom_path);
    };

    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            background: "rgb(0, 0, 0, 0.5)",
            backdrop_blur: "8",
            main_align: "center",
            cross_align: "center",
            onclick: move |_| on_close.call(()),

            // Modal content
            rect {
                width: "500",
                background: "rgb(40, 40, 48, 0.95)",
                backdrop_blur: "20",
                corner_radius: "16",
                border: "1 solid rgb(100, 100, 110, 0.3)",
                padding: "32",
                direction: "vertical",
                spacing: "24",
                shadow: "0 8 32 0 rgb(0, 0, 0, 0.5)",
                onclick: move |e| {
                    e.stop_propagation();
                },

                // Header
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "8",

                    label {
                        font_size: "24",
                        font_weight: "700",
                        color: "white",
                        "Select Installation Directory"
                    }

                    label {
                        font_size: "14",
                        color: "rgb(200, 200, 210)",
                        "Choose where to install the game"
                    }
                }

                rect {
                    width: "fill",
                    height: "1",
                    background: "rgb(100, 100, 110, 0.3)",
                }

                // Directory selection
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "12",

                    label {
                        font_size: "13",
                        font_weight: "600",
                        color: "rgb(220, 220, 230)",
                        "Installation Path:"
                    }

                    rect {
                        width: "fill",
                        padding: "12",
                        background: "rgb(30, 30, 38)",
                        corner_radius: "8",
                        border: "1 solid rgb(100, 100, 110, 0.2)",

                        label {
                            font_size: "13",
                            color: "rgb(200, 200, 210)",
                            "{current_path.read()}"
                        }
                    }

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(60, 60, 75, 0.9)".into(),
                            hover_background: "rgb(70, 70, 85, 1)".into(),
                            border_fill: "rgb(100, 100, 120, 0.5)".into(),
                            focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                            padding: "10 18".into(),
                            corner_radius: "8".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: on_browse,
                        label {
                            font_size: "14",
                            font_weight: "600",
                            "Browse..."
                        }
                    }
                }

                // Error message if installer not found
                if let Some(error) = error_message.read().as_ref() {
                    rect {
                        width: "fill",
                        padding: "12",
                        background: "rgb(80, 40, 40)",
                        corner_radius: "8",
                        border: "1 solid rgb(150, 60, 60)",

                        label {
                            font_size: "13",
                            color: "rgb(255, 200, 200)",
                            "{error}"
                        }
                    }
                }

                // Action buttons
                rect {
                    width: "fill",
                    direction: "horizontal",
                    spacing: "12",
                    main_align: "end",

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(50, 50, 60, 0.9)".into(),
                            hover_background: "rgb(60, 60, 70, 1)".into(),
                            border_fill: "rgb(100, 100, 120, 0.5)".into(),
                            focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                            padding: "10 24".into(),
                            corner_radius: "8".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: move |_| on_close.call(()),
                        label {
                            font_size: "14",
                            font_weight: "600",
                            "Cancel"
                        }
                    }

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: theme::ACCENT_PRIMARY.into(),
                            hover_background: "rgb(80, 100, 180)".into(),
                            border_fill: "rgb(100, 120, 200, 0.5)".into(),
                            focus_border_fill: "rgb(110, 130, 210, 0.7)".into(),
                            padding: "10 24".into(),
                            corner_radius: "8".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: on_confirm_click,
                        label {
                            font_size: "14",
                            font_weight: "600",
                            "Confirm"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ImportGameModal(
    on_close: EventHandler<()>,
    on_confirm: EventHandler<PathBuf>,
    settings: Signal<std::sync::Arc<std::sync::RwLock<backend::settings::GlobalSettings>>>,
    game_name: String,
    game_biz: String,
) -> Element {
    let current_path = use_signal(|| Option::<String>::None);
    let mut error_message = use_signal(|| Option::<String>::None);

    let on_browse = move |_| {
        let mut path_clone = current_path;
        let mut error_clone = error_message;
        spawn(async move {
            if let Some(folder) = rfd::AsyncFileDialog::new()
                .set_title("Select Existing Game Installation Directory")
                .pick_folder()
                .await
            {
                path_clone.set(Some(folder.path().display().to_string()));
                // Clear error message when new path is selected
                error_clone.set(None);
            }
        });
    };
    
    let on_confirm_click = move |_| {
        let path_opt = current_path.read().clone();
        
        if let Some(path_str) = path_opt {
            // Verify game executable exists before confirming
            let settings_guard = settings.read();
            if let Ok(s) = settings_guard.read() {
                let import_path = PathBuf::from(&path_str);
                
                // Check if installer exists for this game
                let installer = backend::game_providers::installer::InstallerManager::create_installer(
                    &game_name,
                    &game_biz,
                    s.temp_directory.clone(),
                    s.games_directory.clone(),
                );
                
                if let Some(installer) = installer {
                    let executable_name = installer.get_executable_name();
                    let executable_path = import_path.join(executable_name);
                    
                    if executable_path.exists() {
                        // Executable found, proceed with import
                        on_confirm.call(import_path);
                    } else {
                        // Executable not found
                        error_message.set(Some(format!(
                            "Game executable '{executable_name}' not found in the selected directory"
                        )));
                        eprintln!("[ImportGameModal] Executable not found: {}", executable_path.display());
                    }
                } else {
                    error_message.set(Some(format!("No installer implemented for {game_name}")));
                    eprintln!("[ImportGameModal] No installer implemented for game: {game_name} (biz: {game_biz})");
                }
            }
        } else {
            error_message.set(Some("Please select a directory".to_string()));
        }
    };

    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "0",
            width: "100%",
            height: "100%",
            background: "rgb(0, 0, 0, 0.5)",
            backdrop_blur: "8",
            main_align: "center",
            cross_align: "center",
            onclick: move |_| on_close.call(()),

            // Modal content
            rect {
                width: "500",
                background: "rgb(40, 40, 48, 0.95)",
                backdrop_blur: "20",
                corner_radius: "16",
                border: "1 solid rgb(100, 100, 110, 0.3)",
                padding: "32",
                direction: "vertical",
                spacing: "24",
                shadow: "0 8 32 0 rgb(0, 0, 0, 0.5)",
                onclick: move |e| {
                    e.stop_propagation();
                },

                // Header
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "8",

                    label {
                        font_size: "24",
                        font_weight: "700",
                        color: "white",
                        "Import Existing Game"
                    }

                    label {
                        font_size: "14",
                        color: "rgb(200, 200, 210)",
                        "Select the folder containing your existing game installation"
                    }
                }

                rect {
                    width: "fill",
                    height: "1",
                    background: "rgb(100, 100, 110, 0.3)",
                }

                // Directory selection
                rect {
                    width: "fill",
                    direction: "vertical",
                    spacing: "12",

                    label {
                        font_size: "13",
                        font_weight: "600",
                        color: "rgb(220, 220, 230)",
                        "Game Directory:"
                    }

                    rect {
                        width: "fill",
                        padding: "12",
                        background: "rgb(30, 30, 38)",
                        corner_radius: "8",
                        border: "1 solid rgb(100, 100, 110, 0.2)",

                        label {
                            font_size: "13",
                            color: "rgb(200, 200, 210)",
                            "{current_path.read().clone().unwrap_or_else(|| \"No directory selected\".to_string())}"
                        }
                    }

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(60, 60, 75, 0.9)".into(),
                            hover_background: "rgb(70, 70, 85, 1)".into(),
                            border_fill: "rgb(100, 100, 120, 0.5)".into(),
                            focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                            padding: "10 18".into(),
                            corner_radius: "8".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: on_browse,
                        label {
                            font_size: "14",
                            font_weight: "600",
                            "Browse..."
                        }
                    }
                }

                // Error message if any
                if let Some(error) = error_message.read().as_ref() {
                    rect {
                        width: "fill",
                        padding: "12",
                        background: "rgb(80, 40, 40)",
                        corner_radius: "8",
                        border: "1 solid rgb(150, 60, 60)",

                        label {
                            font_size: "13",
                            color: "rgb(255, 200, 200)",
                            "{error}"
                        }
                    }
                }

                // Action buttons
                rect {
                    width: "fill",
                    direction: "horizontal",
                    spacing: "12",
                    main_align: "end",

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(50, 50, 60, 0.9)".into(),
                            hover_background: "rgb(60, 60, 70, 1)".into(),
                            border_fill: "rgb(100, 100, 120, 0.5)".into(),
                            focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                            padding: "10 24".into(),
                            corner_radius: "8".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: move |_| on_close.call(()),
                        label {
                            font_size: "14",
                            font_weight: "600",
                            "Cancel"
                        }
                    }

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: theme::ACCENT_PRIMARY.into(),
                            hover_background: "rgb(80, 100, 180)".into(),
                            border_fill: "rgb(100, 120, 200, 0.5)".into(),
                            focus_border_fill: "rgb(110, 130, 210, 0.7)".into(),
                            padding: "10 24".into(),
                            corner_radius: "8".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: on_confirm_click,
                        label {
                            font_size: "14",
                            font_weight: "600",
                            "Import"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DirectX11Checkbox(
    game_id: String,
    settings: Signal<std::sync::Arc<std::sync::RwLock<backend::settings::GlobalSettings>>>,
) -> Element {
    // Get current state from game preferences
    let use_directx11 = {
        let settings_guard = settings.read();
        if let Ok(s) = settings_guard.read() {
            s.game_preferences
                .get(&game_id)
                .map(|prefs| prefs.use_directx11)
                .or_else(|| s.installed_games.get(&game_id).map(|game| game.use_directx11))
                .unwrap_or(false)
        } else {
            false
        }
    };

    let mut checked = use_signal(|| use_directx11);

    let on_toggle = move |_| {
        let new_value = !*checked.read();
        checked.set(new_value);

        // Save to both game preferences and installed game
        if let Ok(mut settings_guard) = settings.read().write() {
            // Update game preferences
            let prefs = settings_guard.game_preferences
                .entry(game_id.clone())
                .or_insert_with(backend::settings::GamePreferences::default);
            prefs.use_directx11 = new_value;

            // Update installed game if it exists
            if let Some(game) = settings_guard.installed_games.get_mut(&game_id) {
                game.use_directx11 = new_value;
            }

            // Save settings
            if let Err(e) = settings_guard.save() {
                eprintln!("[DirectX11Checkbox] Failed to save settings: {e}");
            }
        }
    };

    rsx! {
        rect {
            width: "100%",
            main_align: "center",
            padding: "0 0 0 12",

            rect {
                direction: "horizontal",
                spacing: "8",
                cross_align: "center",
                padding: "6 12",
                corner_radius: "6",
                background: "rgb(35, 35, 40)",
                background_opacity: "0.4",
                border: "1 inner rgb(255, 255, 255, 0.1)",
                shadow: "0 1 4 0 rgb(0, 0, 0, 0.3)",
                backdrop_blur: "8",
                onclick: on_toggle,

                // Checkbox
                rect {
                    width: "16",
                    height: "16",
                    corner_radius: "3",
                    background: if *checked.read() { "rgb(70, 90, 170)" } else { "rgb(50, 50, 55)" },
                    border: "1 solid rgb(255, 255, 255, 0.2)",
                    main_align: "center",
                    cross_align: "center",

                    if *checked.read() {
                        svg {
                            width: "12",
                            height: "12",
                            svg_content: r#"<svg viewBox="0 0 24 24" fill="white"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg>"#
                        }
                    }
                }

                label {
                    font_size: "12",
                    color: "white",
                    "Launch with DirectX11"
                }
            }
        }
    }
}
