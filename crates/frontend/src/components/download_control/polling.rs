use super::types::{DownloadProgress, SetupProgress};
use crate::debug;
use freya::prelude::*;
use std::rc::Rc;

type SetupProgressGetter = Rc<dyn Fn(&str) -> Option<SetupProgress>>;

pub fn poll_runtime_setup(
    _active: Signal<bool>,
    key: &str,
    get_progress: SetupProgressGetter,
    mut progress: Signal<Option<SetupProgress>>,
    mut game_state: crate::pages::game::state::GlobalGameStateSignal,
) {
    let key = key.to_string();

    use_effect(move || {
        let key = key.clone();
        let get_progress = get_progress.clone();

        spawn(async move {
            loop {
                if let Some(p) = get_progress(&key) {
                    progress.set(Some(p));
                } else if progress.read().is_some() {
                    // Clear progress display when download completes
                    // Note: The handler is responsible for setting runtime_ready state
                    game_state.write().set_runtime_active(false);
                    debug!("Runtime setup progress cleared, set runtime_active=false");
                    progress.set(None);
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    });
}

/// Poll for tweaks setup progress continuously
pub fn poll_tweaks_setup(
    _active: Signal<bool>,
    key: &str,
    game_id: String,
    get_progress: SetupProgressGetter,
    mut progress: Signal<Option<SetupProgress>>,
    mut game_state: crate::pages::game::state::GlobalGameStateSignal,
) {
    let key = key.to_string();

    use_effect(move || {
        let key = key.clone();
        let game_id = game_id.clone();
        let get_progress = get_progress.clone();

        spawn(async move {
            loop {
                if let Some(p) = get_progress(&key) {
                    progress.set(Some(p));
                } else if progress.read().is_some() {
                    // Clear progress display when download completes
                    // Note: The handler is responsible for setting tweaks_ready state
                    game_state.write().set_tweaks_active(&game_id, false);
                    progress.set(None);
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    });
}
type DownloadProgressGetter = Rc<dyn Fn(&str) -> Option<DownloadProgress>>;

pub fn poll_download(
    active: Signal<bool>,
    key: &str,
    get_progress: DownloadProgressGetter,
    mut progress: Signal<Option<DownloadProgress>>,
    mut game_state: crate::pages::game::state::GlobalGameStateSignal,
    game_id: String,
) {
    let is_active = *active.read();
    let key = key.to_string();

    use_effect(use_reactive!(|is_active| {
        if !is_active {
            progress.set(None);
            return;
        }

        let key = key.clone();
        let game_id = game_id.clone();
        let get_progress = get_progress.clone();

        spawn(async move {
            loop {
                if !active() {
                    progress.set(None);
                    break;
                }

                let current = get_progress(&key);

                // Update global state
                let mut state = game_state.write();
                state.set_download_progress(&game_id, current.clone());

                if let Some(ref p) = current {
                    let is_complete = !p.is_busy && p.downloaded == p.total && p.total > 0;

                    if is_complete {
                        state.set_download_installed(&game_id, true);
                    }

                    if !p.is_busy {
                        state.set_download_active(&game_id, false);
                    }
                }

                drop(state);

                // Update local display
                progress.set(current.clone());

                if let Some(p) = current
                    && !p.is_busy {
                        progress.set(None);
                        break;
                    }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }));
}
