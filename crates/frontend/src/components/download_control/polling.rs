use freya::prelude::*;
use std::rc::Rc;
use super::types::{DownloadProgress, SetupProgress};

pub fn poll_setup(
    mut active: Signal<bool>,
    key: &str,
    get_progress: Rc<dyn Fn(&str) -> Option<SetupProgress>>,
    mut progress: Signal<Option<SetupProgress>>,
    mut ready: Signal<bool>,
) {
    let is_active = *active.read();
    let key = key.to_string();
    
    use_effect(use_reactive!(|is_active| {
        if !is_active {
            return;
        }
        
        let key = key.clone();
        let get_progress = get_progress.clone();
        
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            
            let mut empty_count = 0;
            
            loop {
                match get_progress(&key) {
                    None => {
                        empty_count += 1;
                        if empty_count >= 5 {
                            ready.set(true);
                            active.set(false);
                            progress.set(None);
                            break;
                        }
                    }
                    Some(p) => {
                        empty_count = 0;
                        progress.set(Some(p));
                    }
                }
                
                if !active() {
                    progress.set(None);
                    break;
                }
                
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }));
}

pub fn poll_download(
    mut active: Signal<bool>,
    key: &str,
    get_progress: Rc<dyn Fn(&str) -> Option<DownloadProgress>>,
    mut progress: Signal<Option<DownloadProgress>>,
    mut installed: Signal<bool>,
) {
    let is_active = *active.read();
    let key = key.to_string();
    
    use_effect(use_reactive!(|is_active| {
        if !is_active {
            return;
        }
        
        let key = key.clone();
        let get_progress = get_progress.clone();
        
        spawn(async move {
            loop {
                let current = get_progress(&key);
                progress.set(current.clone());
                
                if let Some(p) = current {
                    let is_complete = !p.is_busy && p.downloaded == p.total && p.total > 0;
                    
                    if is_complete {
                        installed.set(true);
                    }
                    
                    if !p.is_busy {
                        active.set(false);
                        progress.set(None);
                        break;
                    }
                }
                
                if !active() {
                    progress.set(None);
                    break;
                }
                
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }));
}
