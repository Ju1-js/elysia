use freya::prelude::*;
use std::rc::Rc;
use super::types::{DownloadProgress, SetupProgress};

pub fn poll_runtime_setup(
    active: Signal<bool>,
    key: &str,
    get_progress: Rc<dyn Fn(&str) -> Option<SetupProgress>>,
    mut progress: Signal<Option<SetupProgress>>,
    mut game_state: crate::pages::GlobalGameStateSignal,
) {
    let is_active = *active.read();
    let key = key.to_string();
    
    use_effect(use_reactive!(|is_active| {
        if !is_active {
            progress.set(None);
            return;
        }

        let key = key.clone();
        let get_progress = get_progress.clone();
        
        spawn(async move {
            loop {
                if !active() {
                    progress.set(None);
                    break;
                }
                
                if let Some(p) = get_progress(&key) {
                    progress.set(Some(p));
                } else {
                    game_state.write().set_runtime_ready(true);
                    game_state.write().set_runtime_active(false);
                    eprintln!("[POLLING] Set runtime_ready=true, runtime_active=false");
                    
                    progress.set(None);
                    break;
                }
                
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }));
}

pub fn poll_tweaks_setup(
    active: Signal<bool>,
    key: &str,
    game_id: String,
    get_progress: Rc<dyn Fn(&str) -> Option<SetupProgress>>,
    mut progress: Signal<Option<SetupProgress>>,
    mut game_state: crate::pages::GlobalGameStateSignal,
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
                
                if let Some(p) = get_progress(&key) {
                    progress.set(Some(p));
                } else {
                    game_state.write().set_tweaks_ready(&game_id, true);
                    game_state.write().set_tweaks_active(&game_id, false);
                    
                    progress.set(None);
                    break;
                }
                
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }));
}

pub fn poll_download(
    active: Signal<bool>,
    key: &str,
    get_progress: Rc<dyn Fn(&str) -> Option<DownloadProgress>>,
    mut progress: Signal<Option<DownloadProgress>>,
    mut game_state: crate::pages::GlobalGameStateSignal,
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
                
                if let Some(p) = current {
                    if !p.is_busy {
                        progress.set(None);
                        break;
                    }
                }
                
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }));
}
