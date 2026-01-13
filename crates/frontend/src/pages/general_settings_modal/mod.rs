use crate::pages::settings_modal::{SettingsModal, SettingsContext};
use freya::prelude::*;

#[component]
pub fn GeneralSettingsModal(on_close: EventHandler<()>, scale: f64) -> Element {
    rsx! {
        SettingsModal {
            on_close: on_close,
            context: SettingsContext::General,
            scale: scale,
        }
    }
}
