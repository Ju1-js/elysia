use anyhow::Result;
use backend::{
    components::{ComponentManager, ComponentType, ComponentVersion},
    runners::Runners,
    settings::GlobalSettings,
};
use std::sync::{Arc, RwLock};

use crate::debug;

/// Centralized service for managing components (Wine, Proton, DXVK, etc.)
pub struct ComponentService {
    manager: Arc<RwLock<ComponentManager>>,
}

impl ComponentService {
    /// Create a new ComponentService with an initialized ComponentManager
    pub async fn new() -> Self {
        let manager = ComponentManager::new().await;
        Self {
            manager: Arc::new(RwLock::new(manager)),
        }
    }

    /// Get a cloned Arc reference to the underlying ComponentManager
    pub fn manager(&self) -> Arc<RwLock<ComponentManager>> {
        Arc::clone(&self.manager)
    }

    /// Check if a component is installed
    pub fn is_installed(&self, settings: &GlobalSettings, component_type: ComponentType) -> bool {
        match self.manager.try_read() {
            Ok(manager) => {
                let result = manager.is_installed(settings, component_type);
                eprintln!(
                    "[ComponentService::is_installed] component_type: {:?}, result: {}",
                    component_type, result
                );
                result
            }
            Err(_) => {
                eprintln!(
                    "[ComponentService::is_installed] ComponentManager is busy, assuming component not installed for {:?}",
                    component_type
                );
                debug!("ComponentManager is busy, assuming component not installed");
                false
            }
        }
    }

    /// Get the installed version of a component
    pub fn get_installed_version(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
    ) -> Option<String> {
        match self.manager.try_read() {
            Ok(manager) => manager.get_installed_version(settings, component_type),
            Err(_) => {
                debug!("ComponentManager is busy, cannot get installed version");
                None
            }
        }
    }

    /// Get the latest available version of a component
    pub fn get_latest_version(&self, component_type: ComponentType) -> Option<ComponentVersion> {
        match self.manager.try_read() {
            Ok(manager) => manager.get_latest_version(component_type).cloned(),
            Err(_) => {
                debug!("ComponentManager is busy, cannot get latest version");
                None
            }
        }
    }

    /// Refresh the version cache for a specific component
    pub async fn refresh_component(&self, component_type: ComponentType) -> Result<()> {
        match self.manager.try_write() {
            Ok(mut manager) => manager.refresh_component(component_type).await,
            Err(_) => Err(anyhow::anyhow!(
                "ComponentManager is busy, cannot refresh component cache"
            )),
        }
    }

    /// Download a component with optional progress tracking
    pub async fn download_component(
        &self,
        settings: &GlobalSettings,
        component_type: ComponentType,
        version: Option<&str>,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> Result<std::path::PathBuf> {
        let manager = match self.manager.try_read() {
            Ok(m) => m,
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "ComponentManager is busy, cannot start download"
                ));
            }
        };
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
        match self.manager.try_write() {
            Ok(mut manager) => {
                runner.get_wine_status(settings, &mut manager).await
            }
            Err(_) => {
                debug!("ComponentManager is busy, reading cached Wine versions");
                match self.manager.try_read() {
                    Ok(manager) => {
                        if !runner.is_wine() {
                            return None;
                        }
                        let installed_version =
                            manager.get_installed_version(settings, ComponentType::Wine);
                        let installed = installed_version.is_some();
                        let available_versions: Vec<ComponentVersion> = manager
                            .cache
                            .entries
                            .get(&ComponentType::Wine)
                            .map(|versions| versions.iter().take(3).cloned().collect())
                            .unwrap_or_default();
                        Some((installed, installed_version, available_versions))
                    }
                    Err(_) => {
                        debug!("ComponentManager is locked, cannot read Wine status");
                        None
                    }
                }
            }
        }
    }

    /// Get Proton status (installed, current version, available versions)
    pub async fn get_proton_status(
        &self,
        settings: &GlobalSettings,
        runner: &Runners,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        match self.manager.try_write() {
            Ok(mut manager) => {
                runner.get_proton_status(settings, &mut manager).await
            }
            Err(_) => {
                debug!("ComponentManager is busy, reading cached Proton versions");
                match self.manager.try_read() {
                    Ok(manager) => {
                        if !runner.is_proton() {
                            return None;
                        }
                        let installed_version =
                            manager.get_installed_version(settings, ComponentType::Proton);
                        let installed = installed_version.is_some();
                        let available_versions: Vec<ComponentVersion> = manager
                            .cache
                            .entries
                            .get(&ComponentType::Proton)
                            .map(|versions| versions.iter().take(3).cloned().collect())
                            .unwrap_or_default();
                        Some((installed, installed_version, available_versions))
                    }
                    Err(_) => {
                        debug!("ComponentManager is locked, cannot read Proton status");
                        None
                    }
                }
            }
        }
    }

    /// Get DXVK status (installed, current version, available versions)
    pub async fn get_dxvk_status(
        &self,
        settings: &GlobalSettings,
        runner: &Runners,
    ) -> Option<(bool, Option<String>, Vec<ComponentVersion>)> {
        match self.manager.try_write() {
            Ok(mut manager) => {
                runner.get_dxvk_status(settings, &mut manager).await
            }
            Err(_) => {
                debug!("ComponentManager is busy, reading cached DXVK versions");
                match self.manager.try_read() {
                    Ok(manager) => {
                        if !runner.is_wine() {
                            return None;
                        }
                        let installed_version =
                            manager.get_installed_version(settings, ComponentType::Dxvk);
                        let installed = installed_version.is_some();
                        let available_versions: Vec<ComponentVersion> = manager
                            .cache
                            .entries
                            .get(&ComponentType::Dxvk)
                            .map(|versions| versions.iter().take(3).cloned().collect())
                            .unwrap_or_default();
                        Some((installed, installed_version, available_versions))
                    }
                    Err(_) => {
                        debug!("ComponentManager is locked, cannot read DXVK status");
                        None
                    }
                }
            }
        }
    }
}

impl Clone for ComponentService {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
        }
    }
}
