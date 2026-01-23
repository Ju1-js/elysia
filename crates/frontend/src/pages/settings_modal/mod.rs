mod components;
mod pages;
mod styles;
mod types;
mod version_loader;

pub use components::*;
pub use pages::*;
pub use styles::*;
pub use types::*;
pub use version_loader::*;

use crate::components::ComponentDownloadProgress;
use crate::{debug_error, debug_info};
use backend::components::ComponentManager;
use backend::runners::Runners;
use backend::settings::GlobalSettings;
use freya::prelude::*;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

/// Context for the settings modal - determines if it's for general settings or game-specific
#[derive(Clone, PartialEq)]
pub enum SettingsContext {
    General,
    Game { game_id: String, game_name: String },
}

/// Generic settings modal component that handles both general and game-specific settings
#[component]
pub fn SettingsModal(
    on_close: EventHandler<()>,
    context: SettingsContext,
    scale: f64,
) -> Element {
    let settings_sig = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    let mut current_page = use_signal(|| SettingsPage::LaunchOptions);

    // Get game-specific context if needed
    let (game_id_opt, _game_name_opt, game_state_opt) = match &context {
        SettingsContext::Game { game_id, game_name } => {
            let game_state = use_context::<crate::pages::game::state::GlobalGameStateSignal>();
            (Some(game_id.clone()), Some(game_name.clone()), Some(game_state))
        }
        SettingsContext::General => (None, None, None),
    };

    // Load initial settings based on context
    let (
        initial_runner_type,
        initial_wine_version,
        initial_proton_version,
        initial_dxvk_version,
        initial_launch_wrapper,
        initial_winewayland,
        initial_mangohud,
        initial_gamemode,
        initial_disable_videos,
    ) = load_initial_settings(&settings_sig, &context);

    let selected_runner_type = use_signal(move || initial_runner_type);
    let mut selected_proton = use_signal(move || initial_proton_version);
    let mut selected_wine = use_signal(move || initial_wine_version);
    let mut selected_dxvk = use_signal(move || initial_dxvk_version);
    let mut wine_versions = use_signal(Vec::<ComponentVersionInfo>::new);
    let mut proton_versions = use_signal(Vec::<ComponentVersionInfo>::new);
    let mut dxvk_versions = use_signal(Vec::<ComponentVersionInfo>::new);
    let downloading_wine = use_signal(|| false);
    let downloading_proton = use_signal(|| false);
    let downloading_dxvk = use_signal(|| false);
    let mut wine_installed = use_signal(|| false);
    let mut proton_installed = use_signal(|| false);
    let mut dxvk_installed = use_signal(|| false);
    let launch_wrapper = use_signal(move || initial_launch_wrapper);
    let enable_winewayland = use_signal(move || initial_winewayland);
    let enable_mangohud = use_signal(move || initial_mangohud);
    let enable_gamemode = use_signal(move || initial_gamemode);
    let disable_videos = use_signal(move || initial_disable_videos);

    let component_download_progress = use_signal(|| None::<ComponentDownloadProgress>);
    let mut versions_loading = use_signal(|| true);
    let component_progress_tracker = use_signal(backend::progress::ProgressTracker::new);

    let component_service = use_context::<Signal<Option<crate::services::ComponentService>>>();

    // Load versions
    use_effect(move || {
        spawn(async move {
            let service_option = component_service.read().clone();
            let Some(service) = service_option else {
                debug_error!("ComponentService not yet initialized");
                return;
            };

            let settings_data = {
                let settings = settings_sig.read();
                settings.read().expect("Settings lock poisoned").clone()
            };

            // Get currently selected versions to check if they're installed
            let current_wine = selected_wine.read().clone();
            let current_proton = selected_proton.read().clone();
            let current_dxvk = selected_dxvk.read().clone();

            // Load Proton versions with specific version check
            if let Some((installed, version_infos)) =
                load_proton_versions(&service, &settings_data, &current_proton).await
            {
                proton_installed.set(installed);
                proton_versions.set(version_infos.clone());
                // Only auto-select if there's NO saved version (truly empty from fresh settings)
                // Do NOT overwrite existing saved versions
                if current_proton.is_empty() && !version_infos.is_empty()
                    && let Some(first) = version_infos.first()
                {
                    selected_proton.set(first.internal_name.clone());
                }
            }

            // Load Wine versions with specific version check
            if let Some((installed, version_infos)) =
                load_wine_versions(&service, &settings_data, &current_wine).await
            {
                wine_installed.set(installed);
                wine_versions.set(version_infos.clone());
                // Only auto-select if there's NO saved version
                if current_wine.is_empty() && !version_infos.is_empty()
                    && let Some(first) = version_infos.first()
                {
                    selected_wine.set(first.internal_name.clone());
                }
            }

            // Load DXVK versions with specific version check
            if let Some((installed, version_infos)) =
                load_dxvk_versions(&service, &settings_data, &current_dxvk).await
            {
                dxvk_installed.set(installed);
                dxvk_versions.set(version_infos.clone());
                // Only set first version if no version was loaded from settings
                // Always auto-select when empty, regardless of current runner, because
                // the dropdown will be used if user switches to Wine runner
                if current_dxvk.is_empty() && !version_infos.is_empty()
                    && let Some(first) = version_infos.first()
                {
                    selected_dxvk.set(first.internal_name.clone());
                }
            }

            versions_loading.set(false);
        });
    });

    // Update installation status when selected versions change
    // This ensures the "Download" button shows correctly when user changes the dropdown
    use_effect(use_reactive!(|selected_wine| {
        let selected = selected_wine();
        let settings = settings_sig.read();
        let settings_data = settings.read().expect("Settings lock poisoned");
        let components_dir = settings_data.components_directory.clone();

        spawn(async move {
            // Check if the selected Wine version is installed
            let wine_path = components_dir.join("wine").join(&selected);
            let is_installed = if selected == "system" {
                backend::runners::is_system_wine_available()
            } else if !selected.is_empty() {
                wine_path.exists()
            } else {
                false
            };
            wine_installed.set(is_installed);
        });
    }));

    use_effect(use_reactive!(|selected_proton| {
        let selected = selected_proton();
        let settings = settings_sig.read();
        let settings_data = settings.read().expect("Settings lock poisoned");
        let components_dir = settings_data.components_directory.clone();

        spawn(async move {
            // Check if the selected Proton version is installed
            let proton_path = components_dir.join("proton").join(&selected);
            let is_installed = if selected.is_empty() {
                false
            } else {
                proton_path.exists()
            };
            proton_installed.set(is_installed);
        });
    }));

    use_effect(use_reactive!(|selected_dxvk| {
        let selected = selected_dxvk();
        let settings = settings_sig.read();
        let settings_data = settings.read().expect("Settings lock poisoned");
        let components_dir = settings_data.components_directory.clone();

        spawn(async move {
            // Check if the selected DXVK version is installed
            let dxvk_path = components_dir.join("dxvk").join(&selected);
            let is_installed = if selected.is_empty() {
                false
            } else {
                dxvk_path.exists()
            };
            dxvk_installed.set(is_installed);
        });
    }));

    // Poll component progress tracker
    use_effect(move || {
        let tracker = component_progress_tracker.read().clone();
        let mut progress_signal = component_download_progress;
        spawn(async move {
            loop {
                if let Some(progress) = tracker.get("component_download") {
                    progress_signal.set(Some(ComponentDownloadProgress {
                        component_name: progress.component_name.clone(),
                        downloaded: progress.downloaded,
                        total: progress.total,
                        status: if progress.downloaded == progress.total && progress.total > 0 {
                            "Extracting...".to_string()
                        } else {
                            "Downloading...".to_string()
                        },
                        is_active: progress.is_busy,
                    }));
                } else if progress_signal.read().is_some() {
                    progress_signal.set(None);
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    });

    // Effect to update GlobalGameState when settings change
    // Only update for game-specific settings since each game page initializes its own state from default_preferences
    if let (Some(mut game_state), Some(_)) = (game_state_opt, &game_id_opt) {
        // Game-specific effect to update game state
        use_effect(use_reactive!(|wine_installed,
                                  dxvk_installed,
                                  proton_installed,
                                  selected_runner_type,
                                  selected_wine| {
            let wine_ready = *wine_installed.read();
            let dxvk_ready = *dxvk_installed.read();
            let proton_ready = *proton_installed.read();
            let runner = selected_runner_type.read().clone();
            let wine_ver = selected_wine.read().clone();

            let wine_ready = if wine_ver == "system" { true } else { wine_ready };

            game_state.write().set_wine_ready(wine_ready);
            game_state.write().set_dxvk_ready(dxvk_ready);
            game_state.write().set_proton_ready(proton_ready);

            let runtime_type = match runner {
                RunnerType::Wine => crate::pages::game::state::RunnerType::Wine,
                RunnerType::Proton => crate::pages::game::state::RunnerType::Proton,
            };
            game_state.write().set_runner_type(runtime_type);
        }));
    }
    // Note: General settings do not update GlobalGameState because each game page
    // creates its own GlobalGameState instance and initializes it from default_preferences

    // Save handler - wrap in Rc to allow multiple calls
    let context_for_save = context.clone();
    let save_settings = Rc::new(move || {
        let settings_arc = settings_sig.read().clone();
        let context_clone = context_for_save.clone();
        let runner_type = selected_runner_type.read().clone();
        let wine_ver = selected_wine.read().clone();
        let proton_ver = selected_proton.read().clone();
        let dxvk_ver = selected_dxvk.read().clone();
        let wrapper_val = launch_wrapper.read().clone();
        let winewayland = *enable_winewayland.read();
        let mangohud = *enable_mangohud.read();
        let gamemode = *enable_gamemode.read();
        let disable_videos_val = *disable_videos.read();

        // Call save synchronously instead of spawning async task
        // This ensures the save completes before the modal closes
        save_settings_to_disk_sync(
            &settings_arc,
            context_clone,
            SaveSettingsParams {
                runner_type,
                wine_ver,
                proton_ver,
                dxvk_ver,
                wrapper_val,
                winewayland,
                mangohud,
                gamemode,
                disable_videos: disable_videos_val,
            },
        );
    });

    let title = match &context {
        SettingsContext::General => "General Settings",
        SettingsContext::Game { .. } => "Settings",
    };

    let subtitle = match &context {
        SettingsContext::General => "Default settings for all games",
        SettingsContext::Game { game_name, .. } => game_name.as_str(),
    };

    rsx! {
        rect {
            position: "absolute",
            position_top: "0",
            position_left: "80",
            width: "calc(100% - 80)",
            height: "100%",
            background: "rgb(0, 0, 0, 0.3)",
            backdrop_blur: "8",
            padding: "32",
            onclick: {
                let save_settings = save_settings.clone();
                move |_| {
                    save_settings();
                    on_close.call(());
                }
            },

            rect {
                width: "fill",
                height: "fill",
                direction: "horizontal",
                spacing: "16",
                onclick: move |e| {
                    e.stop_propagation();
                },
                rotate: "0deg",
                scale: "{scale}",

                // Left sidebar navigation
                rect {
                    width: "280",
                    height: "fill",
                    background: "rgb(50, 50, 58)",
                    background_opacity: "0.7",
                    backdrop_blur: "20",
                    corner_radius: "16",
                    border: "1 solid rgb(100, 100, 110, 0.2)",
                    padding: "24",
                    direction: "vertical",
                    spacing: "20",
                    shadow: "0 8 32 0 rgb(0, 0, 0, 0.4)",

                    label {
                        font_size: "18",
                        font_weight: "700",
                        color: "white",
                        "{title}"
                    }

                    rect {
                        width: "fill",
                        height: "1",
                        background: "rgb(100, 100, 110, 0.3)",
                    }

                    rect {
                        width: "fill",
                        direction: "vertical",
                        spacing: "8",

                        SidebarOption {
                            page: SettingsPage::LaunchOptions,
                            current_page: current_page.read().clone(),
                            on_select: move |page| current_page.set(page),
                        }

                        SidebarOption {
                            page: SettingsPage::Runner,
                            current_page: current_page.read().clone(),
                            on_select: move |page| current_page.set(page),
                        }

                        // Only show Utilities page in Game settings
                        if matches!(context, SettingsContext::Game { .. }) {
                            SidebarOption {
                                page: SettingsPage::Utilities,
                                current_page: current_page.read().clone(),
                                on_select: move |page| current_page.set(page),
                            }
                        }

                        // Only show Videos page in General settings
                        if matches!(context, SettingsContext::General) {
                            SidebarOption {
                                page: SettingsPage::Videos,
                                current_page: current_page.read().clone(),
                                on_select: move |page| current_page.set(page),
                            }
                        }
                    }
                }

                // Right main content area
                rect {
                    width: "fill",
                    height: "fill",
                    background: "rgb(40, 40, 48, 0.6)",
                    backdrop_blur: "20",
                    corner_radius: "16",
                    border: "1 solid rgb(100, 100, 110, 0.2)",
                    padding: "32",
                    direction: "vertical",
                    spacing: "24",
                    shadow: "0 8 32 0 rgb(0, 0, 0, 0.4)",

                    // Header
                    rect {
                        width: "fill",
                        direction: "horizontal",
                        main_align: "space-between",
                        cross_align: "center",

                        rect {
                            direction: "vertical",
                            spacing: "4",

                            label {
                                font_size: "28",
                                font_weight: "700",
                                color: "white",
                                "{current_page.read().display_name()}"
                            }

                            label {
                                font_size: "16",
                                font_weight: "400",
                                color: "{TEXT_SECONDARY}",
                                "{subtitle}"
                            }
                        }

                        Button {
                            theme: theme_with!(ButtonTheme {
                                background: "rgb(60, 60, 75, 0.9)".into(),
                                hover_background: "rgb(70, 70, 85, 1)".into(),
                                border_fill: "rgb(100, 100, 120, 0.5)".into(),
                                focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                                padding: "10 18".into(),
                                corner_radius: "10".into(),
                                shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                                font_theme: theme_with!(FontTheme {
                                    color: TEXT_PRIMARY.into(),
                                }),
                            }),
                            onpress: {
                                let save_settings = save_settings.clone();
                                move |_| {
                                    save_settings();
                                    on_close.call(());
                                }
                            },
                            label {
                                font_size: "15",
                                font_weight: "600",
                                "Close"
                            }
                        }
                    }

                    rect {
                        width: "fill",
                        height: "1",
                        background: "rgb(100, 100, 110, 0.3)",
                    }

                    // Scrollable content
                    ScrollView {
                        show_scrollbar: true,
                        width: "fill",
                        height: "fill",

                        rect {
                            width: "fill",
                            direction: "vertical",
                            spacing: "20",
                            padding: "0 16 16 0",

                            // Show active downloads for game settings
                            if let (Some(game_id), Some(game_state)) = (&game_id_opt, game_state_opt) {
                                ActiveDownloadWidget {
                                    game_id: game_id.clone(),
                                    game_state: game_state,
                                    component_download_progress: component_download_progress,
                                    current_page: current_page.read().clone(),
                                }
                            } else if component_download_progress.read().is_some() && *current_page.read() != SettingsPage::Runner {
                                // Show component downloads for general settings (only when NOT on Runner page)
                                // On Runner page, they're shown inline, so would be duplicate
                                rect {
                                    width: "fill",
                                    padding: "16",
                                    background: "rgb(50, 50, 60, 0.8)",
                                    corner_radius: "12",
                                    border: "1 solid rgb(100, 100, 120, 0.3)",
                                    direction: "vertical",
                                    spacing: "12",

                                    if let Some(progress) = component_download_progress.read().as_ref() {
                                        label {
                                            font_size: "14",
                                            font_weight: "600",
                                            color: "white",
                                            "{progress.component_name}"
                                        }

                                        label {
                                            font_size: "12",
                                            color: "{TEXT_SECONDARY}",
                                            "{progress.status}"
                                        }
                                    }
                                }
                            }

                            match *current_page.read() {
                                SettingsPage::LaunchOptions => rsx! {
                                    LaunchOptionsPage {
                                        launch_wrapper: launch_wrapper,
                                        enable_winewayland: enable_winewayland,
                                        enable_mangohud: enable_mangohud,
                                        enable_gamemode: enable_gamemode,
                                    }
                                },
                                SettingsPage::Runner => rsx! {
                                    RunnerPage {
                                        selected_runner_type: selected_runner_type,
                                        selected_proton: selected_proton,
                                        selected_wine: selected_wine,
                                        selected_dxvk: selected_dxvk,
                                        wine_versions: wine_versions,
                                        proton_versions: proton_versions,
                                        dxvk_versions: dxvk_versions,
                                        downloading_wine: downloading_wine,
                                        downloading_proton: downloading_proton,
                                        downloading_dxvk: downloading_dxvk,
                                        wine_installed: wine_installed,
                                        proton_installed: proton_installed,
                                        dxvk_installed: dxvk_installed,
                                        settings_sig: settings_sig,
                                        component_download_progress: component_download_progress,
                                        component_progress_tracker: component_progress_tracker,
                                    }
                                },
                                SettingsPage::Videos => rsx! {
                                    VideosPage {
                                        disable_videos: disable_videos,
                                    }
                                },
                                SettingsPage::Utilities => rsx! {
                                    UtilitiesPage {
                                        settings_sig: settings_sig,
                                        context: context.clone(),
                                        selected_runner_type: selected_runner_type,
                                        selected_proton: selected_proton,
                                        selected_wine: selected_wine,
                                        selected_dxvk: selected_dxvk,
                                        launch_wrapper: launch_wrapper,
                                        enable_winewayland: enable_winewayland,
                                        enable_mangohud: enable_mangohud,
                                        enable_gamemode: enable_gamemode,
                                        disable_videos: disable_videos,
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Load initial settings based on context
fn load_initial_settings(
    settings_sig: &Signal<Arc<RwLock<GlobalSettings>>>,
    context: &SettingsContext,
) -> (RunnerType, String, String, String, String, bool, bool, bool, bool) {
    let settings = settings_sig.read();
    if let Ok(settings_guard) = settings.read() {
        match context {
            SettingsContext::General => {
                // Load from default preferences and global settings
                let prefs = &settings_guard.default_preferences;
                let mut result = load_preferences_values(prefs);
                result.8 = settings_guard.disable_videos;
                result
            }
            SettingsContext::Game { game_id, .. } => {
                // Try installed game first, then preferences, then default
                let mut result = if let Some(game) = settings_guard.installed_games.get(game_id) {
                    load_game_values(game)
                } else if let Some(prefs) = settings_guard.game_preferences.get(game_id) {
                    load_preferences_values(prefs)
                } else {
                    load_preferences_values(&settings_guard.default_preferences)
                };
                result.8 = settings_guard.disable_videos;
                result
            }
        }
    } else {
        // Use global default if can't read settings
        let prefs = backend::settings::GamePreferences::default();
        load_preferences_values(&prefs)
    }
}

/// Load values from `GamePreferences`
fn load_preferences_values(
    prefs: &backend::settings::GamePreferences,
) -> (RunnerType, String, String, String, String, bool, bool, bool, bool) {
    let (runner_type, wine_ver, proton_ver) = match &prefs.runner {
        Runners::Native => {
            // Native runner shouldn't exist in settings, but if it does, default to Wine
            debug_error!("Native runner found in preferences, defaulting to Wine");
            (RunnerType::Wine, String::new(), String::new())
        }
        Runners::Wine(wine) => (RunnerType::Wine, wine.version.clone(), String::new()),
        Runners::Proton(proton) => (RunnerType::Proton, String::new(), proton.version.clone()),
    };

    let dxvk_ver = prefs
        .runtime_components
        .iter()
        .find_map(|component| {
            if let backend::settings::RuntimeComponents::Dxvk(version) = component {
                Some(version.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let wrapper = prefs.command_wrapper.clone().unwrap_or_default();
    (
        runner_type,
        wine_ver,
        proton_ver,
        dxvk_ver,
        wrapper,
        prefs.enable_winewayland,
        prefs.enable_mangohud,
        prefs.enable_gamemode,
        false, // disable_videos default (will be overwritten from GlobalSettings)
    )
}

/// Load values from `InstalledGame`
fn load_game_values(
    game: &backend::settings::InstalledGame,
) -> (RunnerType, String, String, String, String, bool, bool, bool, bool) {
    let (runner_type, wine_ver, proton_ver) = match &game.runner {
        Runners::Native => (RunnerType::Proton, String::new(), String::new()),
        Runners::Wine(wine) => (RunnerType::Wine, wine.version.clone(), String::new()),
        Runners::Proton(proton) => (RunnerType::Proton, String::new(), proton.version.clone()),
    };

    let dxvk_ver = game
        .runtime_components
        .iter()
        .find_map(|component| {
            if let backend::settings::RuntimeComponents::Dxvk(version) = component {
                Some(version.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let wrapper = game.command_wrapper.clone().unwrap_or_default();
    
    // Load boolean flags from GamePreferences if they exist
    let winewayland = game.enable_winewayland;
    let mangohud = game.enable_mangohud;
    let gamemode = game.enable_gamemode;

    (
        runner_type,
        wine_ver,
        proton_ver,
        dxvk_ver,
        wrapper,
        winewayland,
        mangohud,
        gamemode,
        false, // disable_videos default (will be overwritten from GlobalSettings)
    )
}

#[allow(clippy::struct_excessive_bools)]
pub struct SaveSettingsParams {
    pub runner_type: RunnerType,
    pub wine_ver: String,
    pub proton_ver: String,
    pub dxvk_ver: String,
    pub wrapper_val: String,
    pub winewayland: bool,
    pub mangohud: bool,
    pub gamemode: bool,
    pub disable_videos: bool,
}

/// Save settings to disk based on context
pub fn save_settings_to_disk_sync(
    settings_arc: &Arc<RwLock<GlobalSettings>>,
    context: SettingsContext,
    params: SaveSettingsParams,
) {
    if let Ok(mut settings) = settings_arc.write() {
        // Create runner based on selected type
        let runner = match params.runner_type {
            RunnerType::Wine => {
                let version = params.wine_ver.clone();
                Runners::Wine(backend::runners::Wine { version })
            }
            RunnerType::Proton => {
                let version = params.proton_ver.clone();
                Runners::Proton(backend::runners::Proton { version })
            }
        };

        // Create runtime components list for DXVK
        let mut runtime_components = Vec::new();
        if matches!(params.runner_type, RunnerType::Wine) && !params.dxvk_ver.is_empty() {
            runtime_components.push(backend::settings::RuntimeComponents::Dxvk(params.dxvk_ver.clone()));
        }

        // Prepare command_wrapper
        let command_wrapper = if params.wrapper_val.is_empty() {
            None
        } else {
            Some(params.wrapper_val)
        };

        // Update disable_videos in GlobalSettings (applies to both general and game-specific)
        settings.disable_videos = params.disable_videos;

        match context {
            SettingsContext::General => {
                // Update default preferences - preserve existing playtime
                let existing_playtime = settings.default_preferences.playtime_seconds;
                settings.default_preferences = backend::settings::GamePreferences {
                    runner: runner.clone(),
                    runtime_components,
                    command_wrapper,
                    enable_winewayland: params.winewayland,
                    enable_mangohud: params.mangohud,
                    enable_gamemode: params.gamemode,
                    playtime_seconds: existing_playtime,
                    use_directx11: false,
                };

                debug_info!("Saving default preferences - runner: {:?}", runner);
            }
            SettingsContext::Game { game_id, .. } => {
                // Save to installed game if it exists
                if let Some(game) = settings.installed_games.get_mut(&game_id) {
                    game.runner = runner.clone();
                    game.runtime_components.clone_from(&runtime_components);
                    game.command_wrapper.clone_from(&command_wrapper);
                    game.enable_winewayland = params.winewayland;
                    game.enable_mangohud = params.mangohud;
                    game.enable_gamemode = params.gamemode;
                }

                // Always save to game_preferences (even for uninstalled games)
                // Preserve existing playtime if it exists
                let existing_playtime = settings.game_preferences
                    .get(&game_id)
                    .map_or(0, |prefs| prefs.playtime_seconds);
                
                settings.game_preferences.insert(
                    game_id.clone(),
                    backend::settings::GamePreferences {
                        runner,
                        runtime_components,
                        command_wrapper,
                        enable_winewayland: params.winewayland,
                        enable_mangohud: params.mangohud,
                        enable_gamemode: params.gamemode,
                        playtime_seconds: existing_playtime,
                        use_directx11: false,
                    },
                );

                debug_info!("Settings saved successfully for game {}", game_id);
            }
        }

        // Save to disk
        if let Err(e) = settings.save() {
            debug_error!("Failed to save settings: {}", e);
        }
    } else {
        debug_error!(
            "Failed to get write lock on settings - might be locked by another operation"
        );
    }
}
