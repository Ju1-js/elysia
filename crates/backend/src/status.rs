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
    pub async fn check(settings: &GlobalSettings) -> Self {
        let mut component_manager = ComponentManager::new().await;

        // fixme: avoid umu rates locks :xdx:
        let hardcoded_umu_version = "1.3.0";
        let umu_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Umu);
        let umu_installed = umu_installed_version.is_some();
        let umu_needs_update = umu_installed_version
            .as_ref()
            .is_some_and(|v| v != hardcoded_umu_version);

        let _ = component_manager
            .refresh_component(ComponentType::Jadeite)
            .await;
        let jadeite_installed_version =
            component_manager.get_installed_version(settings, ComponentType::Jadeite);
        let jadeite_installed = jadeite_installed_version.is_some();
        let jadeite_latest = component_manager
            .get_latest_version(ComponentType::Jadeite)
            .map(|v| v.version.clone());
        let jadeite_needs_update = component_manager.needs_update(settings, ComponentType::Jadeite);

        let steamrt_dir = settings.components_directory.join("steamrt");
        let steamrt_version_dir = steamrt_dir.join(crate::components::steamrt::STEAMRT_VERSION);
        let steamrt_installed = steamrt_version_dir.exists()
            && steamrt_version_dir
                .join(".elysia_steamrt_installed")
                .exists();

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
        self.umu.needs_update
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
