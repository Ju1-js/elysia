use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct ComponentProgress {
    pub component_name: String,
    pub downloaded: u64,
    pub total: u64,
    pub is_busy: bool,
    pub is_finished: bool,
    pub step_index: Option<usize>,
    pub total_steps: Option<usize>,
}

#[derive(Clone)]
pub struct ProgressTracker {
    progress: Arc<RwLock<HashMap<String, ComponentProgress>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn report(&self, key: &str, component_name: &str, downloaded: u64, total: u64, is_busy: bool, step_index: Option<usize>, total_steps: Option<usize>) {
        let progress = ComponentProgress {
            component_name: component_name.to_string(),
            downloaded,
            total,
            is_busy,
            is_finished: false,
            step_index,
            total_steps,
        };
        
        let mut map = self.progress.write().unwrap();
        map.insert(key.to_string(), progress);
    }

    pub fn get(&self, key: &str) -> Option<ComponentProgress> {
        let map = self.progress.read().unwrap();
        map.get(key).cloned()
    }

    pub fn finish(&self, key: &str) {
        let mut map = self.progress.write().unwrap();
        if let Some(progress) = map.get_mut(key) {
            progress.is_finished = true;
            progress.is_busy = false;
        }
    }

    pub fn clear(&self, key: &str) {
        let mut map = self.progress.write().unwrap();
        map.remove(key);
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
