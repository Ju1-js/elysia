use regex::Regex;
use std::fmt::{Debug, Display};

#[derive(Clone)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u16,
    // Empty for no extra, or make it an Option<_>?
    pub extra: String,
}

impl Version {
    pub fn new(major: u8, minor: u8, patch: u16) -> Self {
        Version {
            major,
            minor,
            patch,
            extra: String::new(),
        }
    }
}

impl From<&String> for Version {
    fn from(s: &String) -> Self {
        let regex = Regex::new(r"^(\d+)\.(\d+)\.(\d+)[\.\+]?(.*)$");

        if let Ok(regex) = regex
            && let Some(matches) = regex.captures(s)
            && let Ok(major) = matches[1].parse()
            && let Ok(minor) = matches[2].parse()
            && let Ok(patch) = matches[3].parse()
        {
            Version {
                major,
                minor,
                patch,
                extra: matches[4].parse().unwrap_or_else(|n| match n {}),
            }
        } else {
            eprintln!("Failed to parse version string: {}", s);
            Version {
                major: 0,
                minor: 0,
                patch: 0,
                extra: String::new(),
            }
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.minor.partial_cmp(&other.minor) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.patch.partial_cmp(&other.patch) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.extra.partial_cmp(&other.extra)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.extra == other.extra
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.extra.is_empty() {
            write!(f, ".{}", self.extra)?;
        }
        Ok(())
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
