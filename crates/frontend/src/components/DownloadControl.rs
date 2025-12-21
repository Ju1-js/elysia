use freya::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub mb_s: f32,
    pub part_index: usize,
    pub parts_total: usize,
    pub status: String,
    pub is_busy: bool,
}

#[derive(Props, Clone)]
pub struct DownloadControlProps {
    pub game_id: String,
    pub game_name: String,
    pub progress_key: String,
    pub installed: bool,
    pub get_progress: Rc<dyn Fn(&str) -> Option<DownloadProgress>>,
    #[props(default = "#ff9500".to_string())]
    pub accent_color: String,
    #[props(default)]
    pub onpress: Option<EventHandler<PressEvent>>,
    pub download_active: Signal<bool>,
}

impl PartialEq for DownloadControlProps {
    fn eq(&self, other: &Self) -> bool {
        self.game_id == other.game_id
            && self.game_name == other.game_name
            && self.progress_key == other.progress_key
            && self.installed == other.installed
            && self.accent_color == other.accent_color
            && Rc::ptr_eq(&self.get_progress, &other.get_progress)
    }
}

#[component]
pub fn DownloadControl(props: DownloadControlProps) -> Element {
    let DownloadControlProps {
        game_id, 
        game_name,
        progress_key,
        installed,
        get_progress,
        accent_color,
        onpress,
        mut download_active,
    } = props;

    let ButtonTheme {
        font_theme,
        ..
    } = use_applied_theme!(&None, filled_button);

    let mut progress = use_signal(|| None::<DownloadProgress>);
    let mut is_installed = use_signal(|| installed);

    use_effect(use_reactive!(|installed| {
        is_installed.set(installed);
    }));
    
    let key_check = progress_key.clone();
    let get_progress_check = get_progress.clone();
    
    use_effect(use_reactive!(|game_id| {
        let current = get_progress_check(&key_check);
        if let Some(ref p) = current {
            if p.is_busy {
                download_active.set(true);
            } else {
                download_active.set(false);
                progress.set(None);
            }
        } else {
            download_active.set(false);
            progress.set(None);
        }
        
        let _ = game_id;
    }));

    let active = download_active();
    use_effect(use_reactive!(|active| {
        if active {
            let key = progress_key.clone();
            let getter = get_progress.clone();
            
            spawn(async move {
                loop {
                    if !download_active() {
                        break;
                    }
                    
                    let current = getter(&key);

                    if let Some(ref p) = current {
                        if !p.is_busy && p.downloaded == p.total && p.total > 0 {
                            is_installed.set(true);
                            download_active.set(false);
                            break;
                        }
                        
                        if !p.is_busy {
                            download_active.set(false);
                            break;
                        }
                    }
                    
                    progress.set(current);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            });
        }
    }));

    let current = progress.read().clone();

    let progress_widget: Element = if let Some(p) = &current {
        if !p.is_busy {
            rsx!({})
        } else {
            let pct = if p.total > 0 {
                (p.downloaded as f64 / p.total as f64) * 100.0
            } else {
                0.0
            };
            let bar_width = format!("{:.0}%", pct);
            
            let label_text = if p.total > 0 && (p.status.starts_with("Downloading") || p.status.starts_with("Extracting")) {
                let downloaded_gb = (p.downloaded as f64) / 1_000_000_000.0;
                let total_gb = (p.total as f64) / 1_000_000_000.0;
                
                if p.mb_s > 0.0 {
                    format!("{} - {:.2} GB / {:.2} GB - {:.2} MB/s",
                        p.status, downloaded_gb, total_gb, p.mb_s)
                } else {
                    format!("{} - {:.2} GB / {:.2} GB",
                        p.status, downloaded_gb, total_gb)
                }
            } else {
                p.status.to_string()
            };

            let pct_text = format!("{:.1}%", pct);

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
                        color: "{font_theme.color}",
                        font_size: "14",
                        font_weight: "700",
                        "{game_name}"
                    }
                    
                    rect {
                        width: "100%",
                        height: "6",
                        background: "rgb(40,40,40)",
                        corner_radius: "3",
                        overflow: "clip",
                        rect {
                            width: "{bar_width}",
                            height: "6",
                            background: "{accent_color}",
                            corner_radius: "3",
                        }
                    }
                    
                    rect {
                        width: "100%",
                        direction: "horizontal",
                        main_align: "space-between",
                        cross_align: "center",
                        label {
                            color: "{font_theme.color}",
                            font_size: "14",
                            "{label_text}"
                        }
                        label {
                            color: "{font_theme.color}",
                            font_size: "14",
                            font_weight: "600",
                            "{pct_text}"
                        }
                    }
                }
            )
        }
    } else {
        rsx!({})
    };

    let button_label = if *is_installed.read() {
        "Start Game"
    } else {
        "Download Game"
    };

    let is_busy = current
        .as_ref()
        .map(|p| p.is_busy)
        .unwrap_or(false);

    rsx! {
        rect {
            width: "100%",
            direction: "vertical",
            spacing: "8",
            { progress_widget }

            {
                if !is_busy {
                    rsx!(
                        rect {
                            width: "100%",
                            direction: "horizontal",
                            main_align: "start",
                            crate::components::MyButton {
                                onpress: onpress,
                                rect {
                                    direction: "horizontal",
                                    cross_align: "center",
                                    main_align: "center",
                                    label {
                                        font_size: "16",
                                        font_weight: "500",
                                        "{button_label}"
                                    }
                                }
                            }
                        }
                    )
                } else {
                    rsx!({})
                }
            }
        }
    }
}
