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
    pub progress_key: String,
    pub installed: bool,
    pub get_progress: Rc<dyn Fn(&str) -> Option<DownloadProgress>>,
    #[props(default = "#ff9500".to_string())]
    pub accent_color: String,
    #[props(default)]
    pub onpress: Option<EventHandler<PressEvent>>,
}

impl PartialEq for DownloadControlProps {
    fn eq(&self, other: &Self) -> bool {
        self.game_id == other.game_id
            && self.progress_key == other.progress_key
            && self.installed == other.installed
            && self.accent_color == other.accent_color
            && Rc::ptr_eq(&self.get_progress, &other.get_progress)
    }
}

#[component]
pub fn DownloadControl(props: DownloadControlProps) -> Element {
    let DownloadControlProps {
        game_id: _, 
        progress_key,
        installed,
        get_progress,
        accent_color,
        onpress,
    } = props;

    let ButtonTheme {
        background: _,
        hover_background: _,
        disabled_background: _,
        border_fill,
        focus_border_fill: _,
        padding: _,
        margin: _,
        corner_radius: _,
        width: _,
        height: _,
        font_theme,
        shadow: _,
    } = use_applied_theme!(&None, filled_button);

    let mut progress_sig = use_signal(|| None::<DownloadProgress>);
    let mut installed_sig = use_signal(|| installed);

    use_effect(use_reactive!(|installed| {
        installed_sig.set(installed);
    }));

    {
        let key = progress_key.clone();
        let get_progress = get_progress.clone();
        use_future(move || {
            let key = key.clone();
            let get_progress = get_progress.clone();
            async move {
                loop {
                    let p = get_progress(&key);

                    if let Some(progress) = &p {
                        if !progress.is_busy && progress.downloaded == progress.total && progress.total > 0 {
                            installed_sig.set(true);
                        }
                    }
                    
                    progress_sig.set(p);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        });
    }

    let progress_opt = progress_sig.read().clone();

    let progress_element: Element = if let Some(p) = &progress_opt {
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
                    corner_radius: "12",
                    background: "#00000078",
                    border: "1 inner {border_fill}",
                    direction: "vertical",
                    spacing: "10",
                    shadow: "0 2 12 0 rgb(0, 0, 0, 40)",
                    backdrop_blur: "16",
                    
                    // Progress bar
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
                    
                    // Status text with percentage
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

    let button_label = if *installed_sig.read() {
        "Start Game"
    } else {
        "Download Game"
    };

    let is_busy = progress_opt
        .as_ref()
        .map(|p| p.is_busy)
        .unwrap_or(false);

    rsx! {
        rect {
            width: "100%",
            direction: "vertical",
            spacing: "8",
            { progress_element }

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