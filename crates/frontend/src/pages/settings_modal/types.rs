#[derive(Clone, PartialEq)]
pub enum SettingsPage {
    LaunchOptions,
    Runner,
    Videos,
    Utilities,
}

impl SettingsPage {
    pub fn display_name(&self) -> &str {
        match self {
            SettingsPage::LaunchOptions => "Launch Options",
            SettingsPage::Runner => "Runner",
            SettingsPage::Videos => "Videos",
            SettingsPage::Utilities => "Utilities",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum RunnerType {
    Proton,
    Wine,
}

impl RunnerType {
    pub fn display_name(&self) -> &str {
        match self {
            RunnerType::Proton => "Proton",
            RunnerType::Wine => "Wine",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentVersionInfo {
    pub internal_name: String,
    pub display_name: String,
}
