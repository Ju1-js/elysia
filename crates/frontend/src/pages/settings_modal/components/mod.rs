mod active_download;
mod component_section;
mod dropdown;
mod runner_toggle;
mod sidebar;
mod toggle;

pub use active_download::ActiveDownloadWidget;
pub use component_section::ComponentVersionSection;
pub use dropdown::StylizedDropdown;
pub use runner_toggle::RunnerToggleButton;
pub use sidebar::SidebarOption;
pub use toggle::ToggleOption;

pub fn check_command_exists(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
