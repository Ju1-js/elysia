use std::path::PathBuf;
use version::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionDiff {
    NotInstalled {
        latest: Version,
        installation_path: Option<PathBuf>,
    },

    Latest {
        version: Version,
    },

    Diff {
        current: Version,
        latest: Version,
        installation_path: Option<PathBuf>,
    },
}

impl VersionDiff {
    pub fn is_installed(&self) -> bool {
        !matches!(self, Self::NotInstalled { .. })
    }

    pub fn is_latest(&self) -> bool {
        matches!(self, Self::Latest { .. })
    }

    pub fn needs_update(&self) -> bool {
        matches!(self, Self::Diff { .. })
    }
}
