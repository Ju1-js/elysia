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

#[derive(Copy, Clone)]
pub struct ReportParams {
    pub downloaded: u64,
    pub total: u64,
    pub is_busy: bool,
    pub step_index: Option<usize>,
    pub total_steps: Option<usize>,
}

impl ProgressTracker {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// # Panics
    /// Panics if the lock is poisoned.
    pub fn report(
        &self,
        key: &str,
        component_name: &str,
        params: ReportParams,
    ) {
        eprintln!("[ProgressTracker] report() called - key: {}, component: {}, downloaded: {}/{}, is_busy: {}", 
            key, component_name, params.downloaded, params.total, params.is_busy);
        
        let progress = ComponentProgress {
            component_name: component_name.to_string(),
            downloaded: params.downloaded,
            total: params.total,
            is_busy: params.is_busy,
            is_finished: false,
            step_index: params.step_index,
            total_steps: params.total_steps,
        };

        let mut map = self.progress.write().expect("Progress lock poisoned");
        map.insert(key.to_string(), progress);
        eprintln!("[ProgressTracker] Progress stored successfully for key: {key}");
    }

    /// # Panics
    /// Panics if the lock is poisoned.
    #[must_use] 
    pub fn get(&self, key: &str) -> Option<ComponentProgress> {
        let map = self.progress.read().expect("Progress lock poisoned");
        let result = map.get(key).cloned();
        if result.is_some() {
            eprintln!("[ProgressTracker] get() found progress for key: {key}");
        }
        result
    }

    /// # Panics
    /// Panics if the lock is poisoned.
    pub fn finish(&self, key: &str) {
        let mut map = self.progress.write().expect("Progress lock poisoned");
        if let Some(progress) = map.get_mut(key) {
            progress.is_finished = true;
            progress.is_busy = false;
        }
    }

    /// # Panics
    /// Panics if the lock is poisoned.
    pub fn clear(&self, key: &str) {
        let mut map = self.progress.write().expect("Progress lock poisoned");
        map.remove(key);
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
