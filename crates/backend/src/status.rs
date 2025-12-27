use crate::settings::GlobalSettings;
use crate::components::{ComponentManager, ComponentType};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub installed: bool,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub needs_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub umu: ComponentStatus,
    pub jadeite: ComponentStatus,
    pub steamrt: ComponentStatus,
}

impl SystemStatus {
    pub async fn check(settings: &GlobalSettings) -> Self {
        let mut component_manager = ComponentManager::new().await;
        
        // fixme: avoid umu rates locks :xdx:
        let hardcoded_umu_version = "1.3.0";
        let umu_installed_version = component_manager.get_installed_version(settings, ComponentType::Umu);
        let umu_installed = umu_installed_version.is_some();
        let umu_needs_update = umu_installed_version
            .as_ref()
            .map_or(false, |v| v != hardcoded_umu_version);
        
        let _ = component_manager.refresh_component(ComponentType::Jadeite).await;
        let jadeite_installed_version = component_manager.get_installed_version(settings, ComponentType::Jadeite);
        let jadeite_installed = jadeite_installed_version.is_some();
        let jadeite_latest = component_manager.get_latest_version(ComponentType::Jadeite)
            .map(|v| v.version.clone());
        let jadeite_needs_update = component_manager.needs_update(settings, ComponentType::Jadeite);
        
        let steamrt_dir = settings.components_directory.join("steamrt");
        let steamrt_installed = steamrt_dir.exists() 
            && steamrt_dir.read_dir().ok().map_or(false, |mut d| d.next().is_some());
        
        Self {
            umu: ComponentStatus {
                installed: umu_installed,
                installed_version: umu_installed_version,
                latest_version: Some(hardcoded_umu_version.to_string()),
                needs_update: umu_needs_update,
            },
            jadeite: ComponentStatus {
                installed: jadeite_installed,
                installed_version: jadeite_installed_version,
                latest_version: jadeite_latest,
                needs_update: jadeite_needs_update,
            },
            steamrt: ComponentStatus {
                installed: steamrt_installed,
                installed_version: None,
                latest_version: None,
                needs_update: false,
            },
        }
    }
    
    pub fn runtime_ready(&self) -> bool {
        self.umu.installed && self.steamrt.installed
    }
    
    pub fn runtime_needs_update(&self) -> bool {
        self.umu.needs_update
    }
    
    pub fn tweaks_ready(&self) -> bool {
        self.jadeite.installed
    }
    
    pub fn tweaks_need_update(&self) -> bool {
        self.jadeite.needs_update
    }
}
