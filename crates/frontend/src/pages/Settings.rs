#![allow(non_snake_case)]

use crate::pages::{general_settings_modal::GeneralSettingsModal, Home};
use freya::prelude::*;
use freya_router::prelude::use_navigator;

/// Settings page component - displays the general settings modal with Home as background
#[component]
pub fn Settings() -> Element {
    let navigator = use_navigator();
    
    rsx! {
        rect {
            width: "100%",
            height: "100%",
            // Home page as background
            Home {}
            
            // Settings modal overlay
            GeneralSettingsModal {
                on_close: move |_| {
                    // Navigate back when settings are closed
                    navigator.go_back();
                },
                scale: 1.0,
            }
        }
    }
}
