#![allow(non_snake_case)]

use crate::pages::general_settings_modal::GeneralSettingsModal;
use freya::prelude::*;
use freya_router::prelude::use_navigator;

/// Settings page component
#[component]
pub fn Settings() -> Element {
    let navigator = use_navigator();
    
    rsx! {
        rect {
            width: "100%",
            height: "100%",
            background: "rgb(72, 72, 85)",
            
            // Settings modal overlay
            GeneralSettingsModal {
                on_close: move |()| {
                    // Navigate back when settings are closed
                    navigator.go_back();
                },
                scale: 1.0,
            }
        }
    }
}
