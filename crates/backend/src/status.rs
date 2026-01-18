use crate::components::{ComponentManager, ComponentType};
use crate::settings::GlobalSettings;
use serde::{Deserialize, Serialize};

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
    pub proton: ComponentStatus,
    pub wine: ComponentStatus,
    pub dxvk: ComponentStatus,
}

impl SystemStatus {
    #[must_use]
    pub fn check(settings: &GlobalSettings, component_manager: &ComponentManager) -> Self {
        // Check UMU status
        let umu_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Umu);
        let umu_installed = umu_installed_version.is_some();
        let umu_latest = component_manager
            .get_latest_version(ComponentType::Umu)
            .map(|v| v.version.clone());
        let umu_needs_update = component_manager.needs_update(settings, ComponentType::Umu);

        // Check Jadeite status
        let jadeite_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Jadeite);
        let jadeite_installed = jadeite_installed_version.is_some();
        let jadeite_latest = component_manager
            .get_latest_version(ComponentType::Jadeite)
            .map(|v| v.version.clone());
        let jadeite_needs_update = component_manager.needs_update(settings, ComponentType::Jadeite);

        // Check SteamRT status
        let steamrt_installed_version =
            component_manager.get_installed_version(settings, ComponentType::SteamRuntime);
        let steamrt_installed = steamrt_installed_version.is_some();
        let steamrt_latest = component_manager
            .get_latest_version(ComponentType::SteamRuntime)
            .map(|v| v.version.clone());
        let steamrt_needs_update = component_manager.needs_update(settings, ComponentType::SteamRuntime);

        let proton_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Proton);
        let proton_installed = proton_installed_version.is_some();

        let wine_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Wine);
        let wine_installed = wine_installed_version.is_some();

        let dxvk_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Dxvk);
        let dxvk_installed = dxvk_installed_version.is_some();

        Self {
            umu: ComponentStatus {
                installed: umu_installed,
                installed_version: umu_installed_version,
                latest_version: umu_latest,
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
                installed_version: steamrt_installed_version,
                latest_version: steamrt_latest,
                needs_update: steamrt_needs_update,
            },
            proton: ComponentStatus {
                installed: proton_installed,
                installed_version: proton_installed_version,
                latest_version: None,
                needs_update: false,
            },
            wine: ComponentStatus {
                installed: wine_installed,
                installed_version: wine_installed_version,
                latest_version: None,
                needs_update: false,
            },
            dxvk: ComponentStatus {
                installed: dxvk_installed,
                installed_version: dxvk_installed_version,
                latest_version: None,
                needs_update: false,
            },
        }
    }

    #[must_use] 
    pub fn runtime_ready(&self) -> bool {
        self.proton.installed && self.umu.installed && self.steamrt.installed
    }

    #[must_use] 
    pub fn wine_runtime_ready(&self) -> bool {
        self.wine.installed && self.dxvk.installed
    }

    #[must_use] 
    pub fn runtime_needs_update(&self) -> bool {
        self.umu.needs_update || self.steamrt.needs_update
    }

    #[must_use] 
    pub fn tweaks_ready(&self) -> bool {
        self.jadeite.installed
    }

    #[must_use] 
    pub fn tweaks_need_update(&self) -> bool {
        self.jadeite.needs_update
    }
}
