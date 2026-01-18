use anyhow::Result;
use backend::{
    components::{ComponentManager, ComponentType, ComponentVersion},
    runners::Runners,
    settings::GlobalSettings,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Centralized service for managing components (Wine, Proton, DXVK, etc.)
pub struct ComponentService {
    manager: Arc<RwLock<ComponentManager>>,
}

impl ComponentService {
    /// Create a new `ComponentService` with an initialized `ComponentManager`
    /// This also refreshes the component index to load available versions
    pub async fn new() -> Self {
        let mut manager = ComponentManager::new().await;
        
        // Refresh the component index during initialization
        // This fetches the latest component definitions from the remote repository
        if let Err(err) = manager.refresh_index().await {
            eprintln!("[ComponentService] Failed to refresh component index during initialization: {err}");
        }
        
        Self {
            manager: Arc::new(RwLock::new(manager)),
        }
    }

    /// Get a cloned Arc reference to the underlying `ComponentManager`
    pub fn manager(&self) -> Arc<RwLock<ComponentManager>> {
        Arc::clone(&self.manager)
    }

    /// Check if a component is installed
    pub async fn is_installed(&self, settings: &GlobalSettings, component_type: ComponentType) -> bool {
        let manager = self.manager.read().await;
        let result = manager.is_installed(settings, component_type);
        eprintln!(
            "[ComponentService::is_installed] component_type: {component_type:?}, result: {result}"
        );
        result
    }

    /// Get the installed version of a component
    #[allow(dead_code)]
    pub async fn get_installed_version(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
    ) -> Option<String> {
        let manager = self.manager.read().await;
        manager.get_installed_version(settings, component_type)
    }

    /// Get the latest available version of a component
    #[allow(dead_code)]
    pub async fn get_latest_version(&self, component_type: ComponentType) -> Option<ComponentVersion> {
        let manager = self.manager.read().await;
        manager.get_latest_version(component_type).cloned()
    }

    /// Refresh the version cache for a specific component
    #[allow(dead_code)]
    pub async fn refresh_component(&self, component_type: ComponentType) -> Result<()> {
        let mut manager = self.manager.write().await;
        manager.refresh_component(component_type).await
    }

    /// Download a component with optional progress tracking
    #[allow(dead_code)]
    pub async fn download_component(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
        version: Option<&str>,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> Result<std::path::PathBuf> {
        let manager = self.manager.read().await;
        manager
            .download_component(settings, component_type, version, progress_callback)
            .await
    }

    /// Get Wine status (installed, current version, available versions)
    pub async fn get_wine_status(
        &self,
        settings: &GlobalSettings,
        runner: &Runners,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        let mut manager = self.manager.write().await;
        runner.get_wine_status(settings, &mut manager)
    }

    /// Get Proton status (installed, current version, available versions)
    pub async fn get_proton_status(
        &self,
        settings: &GlobalSettings,
        runner: &Runners,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        let mut manager = self.manager.write().await;
        runner.get_proton_status(settings, &mut manager)
    }

    /// Get DXVK status (installed, current version, available versions)
    pub async fn get_dxvk_status(
        &self,
        settings: &GlobalSettings,
        runner: &Runners,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        let mut manager = self.manager.write().await;
        runner.get_dxvk_status(settings, &mut manager)
    }
}

impl Clone for ComponentService {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
        }
    }
}
