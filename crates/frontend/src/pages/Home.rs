use backend::{settings::GlobalSettings, status::SystemStatus};
use freya::prelude::*;
use std::sync::{Arc, RwLock};

use crate::debug_info;

/// Home page component
#[component]
pub fn Home() -> Element {
    let settings = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();
    let mut system_status = use_context::<Signal<Option<SystemStatus>>>();
    let component_service = use_context::<Signal<Option<crate::services::ComponentService>>>();

    // Check component status once on Home page mount
    use_effect(use_reactive!(|| {
        let settings_arc = settings.read().clone();
        let component_svc = component_service;
        spawn(async move {
            // Wait for ComponentService to be ready
            while component_svc.read().is_none() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            
            let Some(service) = component_svc.read().clone() else {
                return;
            };
            
            let settings_data = {
                let Ok(s) = settings_arc.read() else { return };
                // Clone the data we need before the await
                s.clone()
            };
            debug_info!("Checking system component status...");
            
            // Use the cached ComponentManager from the service
            let manager = service.manager();
            let manager_guard = manager.read().await;
            let status = SystemStatus::check(&settings_data, &manager_guard);
            debug_info!(
                "Status check complete - Runtime ready: {}, Tweaks ready: {}",
                status.runtime_ready(),
                status.tweaks_ready()
            );
            system_status.set(Some(status));
        });
    }));
    
    rsx! {
        rect {
            width: "fill",
            height: "fill",
            cross_align: "center",
            main_align: "center",
            direction: "vertical",

            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                layer: "1",
                background: "rgb(22, 22, 30)",
            }

            rect {
                direction: "vertical",
                cross_align: "center",
                main_align: "center",
                spacing: "24",
                padding: "40",
                background: "rgb(35,35,45,0.5)",
                corner_radius: "20",
                shadow: "0 8 32 0 rgb(0, 0, 0, 60)",
                backdrop_blur: "16",
                max_width: "600",
                layer: "1",

                image {
                    width: "120",
                    height: "120",
                    sampling: "trilinear",
                    image_data: static_bytes(include_bytes!("../../../../assets/elysia.png"))
                }

                label {
                    font_family: "Noto Sans",
                    font_size: "32",
                    font_weight: "700",
                    "Welcome to Elysia!"
                }

                label {
                    font_family: "Noto Sans",
                    font_size: "16",
                    color: "rgb(200,200,210)",
                    text_align: "center",
                    "Select a game from the sidebar to get started."
                }

                if system_status.read().is_none() {
                    label {
                        font_size: "14",
                        color: "rgb(150,150,160)",
                        "Checking system components..."
                    }
                }
            }
        }
    }
}
