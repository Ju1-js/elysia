use crate::pages::settings_modal::styles::{TEXT_SECONDARY, INTERACTIVE_BORDER};
use crate::pages::settings_modal::types::RunnerType;
use backend::settings::GlobalSettings;
use backend::runners::Runners;
use freya::prelude::*;
use std::sync::{Arc, RwLock};

/// Utilities settings page for Wine tools
#[component]
pub fn UtilitiesPage(
    settings_sig: Signal<Arc<RwLock<GlobalSettings>>>,
    context: crate::pages::settings_modal::SettingsContext,
    selected_runner_type: Signal<RunnerType>,
    selected_proton: Signal<String>,
    selected_wine: Signal<String>,
    selected_dxvk: Signal<String>,
    launch_wrapper: Signal<String>,
    enable_winewayland: Signal<bool>,
    enable_mangohud: Signal<bool>,
    enable_gamemode: Signal<bool>,
    disable_videos: Signal<bool>,
) -> Element {
    let mut command_output = use_signal(String::new);
    let mut is_running = use_signal(|| false);

    // Clone context for use in multiple button handlers
    let context_winecfg = context.clone();
    let context_regedit = context.clone();
    let context_reset = context;

    rsx! {
        rect {
            width: "fill",
            direction: "vertical",
            spacing: "28",
            padding: "0 16 16 0",

            // Description
            rect {
                direction: "vertical",
                spacing: "12",

                label {
                    font_size: "18",
                    font_weight: "600",
                    color: "white",
                    "Wine Utilities"
                }

                label {
                    font_size: "13",
                    color: "{TEXT_SECONDARY}",
                    "Run Wine configuration tools and manage your Wine prefix"
                }
            }

            // Buttons section
            rect {
                direction: "vertical",
                spacing: "16",

                // Winecfg button
                rect {
                    direction: "vertical",
                    spacing: "8",

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(60, 60, 75, 0.9)".into(),
                            hover_background: "rgb(70, 70, 85, 1)".into(),
                            border_fill: "rgb(100, 100, 120, 0.5)".into(),
                            focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                            padding: "12 20".into(),
                            corner_radius: "10".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: move |_| {
                            if !*is_running.read() {
                                // Save settings before launching utility
                                let settings = settings_sig.read().clone();
                                let ctx = context_winecfg.clone();
                                let runner_type = selected_runner_type.read().clone();
                                let proton_ver = selected_proton.read().clone();
                                let wine_ver = selected_wine.read().clone();
                                let dxvk_ver = selected_dxvk.read().clone();
                                let wrapper_val = launch_wrapper.read().clone();
                                let winewayland = *enable_winewayland.read();
                                let mangohud = *enable_mangohud.read();
                                let gamemode = *enable_gamemode.read();
                                let disable_videos_val = *disable_videos.read();
                                let utility_cmd = "winecfg".to_string();

                                // Save current settings to ensure the utility uses the latest configuration
                                crate::pages::settings_modal::save_settings_to_disk_sync(
                                    &settings,
                                    ctx.clone(),
                                    crate::pages::settings_modal::SaveSettingsParams {
                                        runner_type: runner_type.clone(),
                                        wine_ver: wine_ver.clone(),
                                        proton_ver: proton_ver.clone(),
                                        dxvk_ver,
                                        wrapper_val,
                                        winewayland,
                                        mangohud,
                                        gamemode,
                                        disable_videos: disable_videos_val,
                                    },
                                );

                                spawn(async move {
                                    is_running.set(true);
                                    command_output.set(format!("Running {utility_cmd}..."));

                                    let utility_cmd_for_blocking = utility_cmd.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        execute_wine_utility(
                                            &settings,
                                            &ctx,
                                            &runner_type,
                                            &proton_ver,
                                            &wine_ver,
                                            &utility_cmd_for_blocking,
                                        )
                                    })
                                    .await;

                                    match result {
                                        Ok(Ok(output)) => {
                                            command_output.set(format!("{utility_cmd} completed successfully"));
                                            if !output.is_empty() {
                                                command_output.set(format!("{utility_cmd} completed:\n{output}"));
                                            }
                                        }
                                        Ok(Err(e)) => {
                                            command_output.set(format!("Error running {utility_cmd}: {e}"));
                                        }
                                        Err(e) => {
                                            command_output.set(format!("Task error: {e}"));
                                        }
                                    }
                                    is_running.set(false);
                                });
                            }
                        },
                        label {
                            font_size: "15",
                            font_weight: "600",
                            "Run Winecfg"
                        }
                    }

                    label {
                        font_size: "12",
                        color: "{TEXT_SECONDARY}",
                        "Configure Wine settings"
                    }
                }

                // Regedit button
                rect {
                    direction: "vertical",
                    spacing: "8",

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(60, 60, 75, 0.9)".into(),
                            hover_background: "rgb(70, 70, 85, 1)".into(),
                            border_fill: "rgb(100, 100, 120, 0.5)".into(),
                            focus_border_fill: "rgb(110, 110, 130, 0.7)".into(),
                            padding: "12 20".into(),
                            corner_radius: "10".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: move |_| {
                            if !*is_running.read() {
                                // Save settings before launching utility
                                let settings = settings_sig.read().clone();
                                let ctx = context_regedit.clone();
                                let runner_type = selected_runner_type.read().clone();
                                let proton_ver = selected_proton.read().clone();
                                let wine_ver = selected_wine.read().clone();
                                let dxvk_ver = selected_dxvk.read().clone();
                                let wrapper_val = launch_wrapper.read().clone();
                                let winewayland = *enable_winewayland.read();
                                let mangohud = *enable_mangohud.read();
                                let gamemode = *enable_gamemode.read();
                                let disable_videos_val = *disable_videos.read();
                                let utility_cmd = "regedit".to_string();

                                // Save current settings to ensure the utility uses the latest configuration
                                crate::pages::settings_modal::save_settings_to_disk_sync(
                                    &settings,
                                    ctx.clone(),
                                    crate::pages::settings_modal::SaveSettingsParams {
                                        runner_type: runner_type.clone(),
                                        wine_ver: wine_ver.clone(),
                                        proton_ver: proton_ver.clone(),
                                        dxvk_ver,
                                        wrapper_val,
                                        winewayland,
                                        mangohud,
                                        gamemode,
                                        disable_videos: disable_videos_val,
                                    },
                                );

                                spawn(async move {
                                    is_running.set(true);
                                    command_output.set(format!("Running {utility_cmd}..."));

                                    let utility_cmd_for_blocking = utility_cmd.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        execute_wine_utility(
                                            &settings,
                                            &ctx,
                                            &runner_type,
                                            &proton_ver,
                                            &wine_ver,
                                            &utility_cmd_for_blocking,
                                        )
                                    })
                                    .await;

                                    match result {
                                        Ok(Ok(output)) => {
                                            command_output.set(format!("{utility_cmd} completed successfully"));
                                            if !output.is_empty() {
                                                command_output.set(format!("{utility_cmd} completed:\n{output}"));
                                            }
                                        }
                                        Ok(Err(e)) => {
                                            command_output.set(format!("Error running {utility_cmd}: {e}"));
                                        }
                                        Err(e) => {
                                            command_output.set(format!("Task error: {e}"));
                                        }
                                    }
                                    is_running.set(false);
                                });
                            }
                        },
                        label {
                            font_size: "15",
                            font_weight: "600",
                            "Run Regedit"
                        }
                    }

                    label {
                        font_size: "12",
                        color: "{TEXT_SECONDARY}",
                        "Edit Wine registry"
                    }
                }

                // Reset wineprefix button
                rect {
                    direction: "vertical",
                    spacing: "8",

                    Button {
                        theme: theme_with!(ButtonTheme {
                            background: "rgb(100, 40, 40, 0.9)".into(),
                            hover_background: "rgb(120, 50, 50, 1)".into(),
                            border_fill: "rgb(150, 60, 60, 0.5)".into(),
                            focus_border_fill: "rgb(170, 70, 70, 0.7)".into(),
                            padding: "12 20".into(),
                            corner_radius: "10".into(),
                            shadow: "0 2 8 0 rgb(0, 0, 0, 0.3)".into(),
                            font_theme: theme_with!(FontTheme {
                                color: "white".into(),
                            }),
                        }),
                        onpress: move |_| {
                            if !*is_running.read() {
                                let settings = settings_sig.read().clone();
                                let ctx = context_reset.clone();
                                let runner_type = selected_runner_type.read().clone();
                                let proton_ver = selected_proton.read().clone();
                                let wine_ver = selected_wine.read().clone();
                                let utility_cmd = "reset".to_string();

                                spawn(async move {
                                    is_running.set(true);
                                    command_output.set(format!("Running {utility_cmd}..."));

                                    let utility_cmd_for_blocking = utility_cmd.clone();
                                    let result = tokio::task::spawn_blocking(move || {
                                        execute_wine_utility(
                                            &settings,
                                            &ctx,
                                            &runner_type,
                                            &proton_ver,
                                            &wine_ver,
                                            &utility_cmd_for_blocking,
                                        )
                                    })
                                    .await;

                                    match result {
                                        Ok(Ok(output)) => {
                                            command_output.set(format!("{utility_cmd} completed successfully"));
                                            if !output.is_empty() {
                                                command_output.set(format!("{utility_cmd} completed:\n{output}"));
                                            }
                                        }
                                        Ok(Err(e)) => {
                                            command_output.set(format!("Error running {utility_cmd}: {e}"));
                                        }
                                        Err(e) => {
                                            command_output.set(format!("Task error: {e}"));
                                        }
                                    }
                                    is_running.set(false);
                                });
                            }
                        },
                        label {
                            font_size: "15",
                            font_weight: "600",
                            "Reset Wineprefix"
                        }
                    }

                    label {
                        font_size: "12",
                        color: "{TEXT_SECONDARY}",
                        "⚠️ Warning: This will delete and recreate the Wine prefix"
                    }
                }
            }

            // Output display
            if !command_output.read().is_empty() {
                rect {
                    width: "fill",
                    padding: "16",
                    background: "rgb(50, 50, 60, 0.8)",
                    corner_radius: "12",
                    border: "1 solid {INTERACTIVE_BORDER}",
                    direction: "vertical",
                    spacing: "8",

                    label {
                        font_size: "14",
                        font_weight: "600",
                        color: "white",
                        "Status"
                    }

                    label {
                        font_size: "12",
                        color: "{TEXT_SECONDARY}",
                        "{command_output.read()}"
                    }
                }
            }
        }
    }
}

fn execute_wine_utility(
    settings_arc: &Arc<RwLock<GlobalSettings>>,
    context: &crate::pages::settings_modal::SettingsContext,
    runner_type: &RunnerType,
    proton_ver: &str,
    wine_ver: &str,
    utility: &str,
) -> Result<String, String> {
    let settings = settings_arc
        .read()
        .map_err(|e| format!("Failed to read settings: {e}"))?;

    // Get game_id from context
    let game_id = match context {
        crate::pages::settings_modal::SettingsContext::Game { game_id, .. } => game_id,
        crate::pages::settings_modal::SettingsContext::General => {
            return Err("Utilities can only be run from game-specific settings".to_string());
        }
    };

    // Get the game to find the biz_name for wineprefix
    let game = settings
        .installed_games
        .get(game_id)
        .ok_or_else(|| "Game not found in installed games".to_string())?;

    let wineprefix = settings.wineprefixes_directory.join(&game.biz_name);

    // Handle reset wineprefix specially
    if utility == "reset" {
        if wineprefix.exists() {
            std::fs::remove_dir_all(&wineprefix)
                .map_err(|e| format!("Failed to remove wineprefix: {e}"))?;
        }
        return Ok("Wineprefix has been reset. It will be recreated on next game launch.".to_string());
    }

    // Execute winecfg or regedit using backend runners
    match runner_type {
        RunnerType::Proton => {
            let proton = backend::runners::Proton {
                version: proton_ver.to_string(),
            };
            match utility {
                "winecfg" => proton.launch_winecfg(&settings, game),
                "regedit" => proton.launch_regedit(&settings, game),
                _ => return Err(format!("Unknown utility: {utility}")),
            }
            .map_err(|e| format!("Failed to launch {utility}: {e}"))?;
        }
        RunnerType::Wine => {
            let wine = backend::runners::Wine {
                version: wine_ver.to_string(),
            };
            match utility {
                "winecfg" => wine.launch_winecfg(&settings, game),
                "regedit" => wine.launch_regedit(&settings, game),
                _ => return Err(format!("Unknown utility: {utility}")),
            }
            .map_err(|e| format!("Failed to launch {utility}: {e}"))?;
        }
    }

    Ok(format!("{utility} launched successfully"))
}

