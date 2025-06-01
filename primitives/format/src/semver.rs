use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SemVer {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ERR: &str = "Failed to parse version string. Expected: major.minor.patch";
        let mut pieces = s.split('.');
        let major = u32::from_str(pieces.next().ok_or(ERR)?).map_err(|_| ERR)?;
        let minor = u32::from_str(pieces.next().ok_or(ERR)?).map_err(|_| ERR)?;
        let patch = u32::from_str(pieces.next().ok_or(ERR)?).map_err(|_| ERR)?;
        Ok(SemVer {
            major,
            minor,
            patch,
        })
    }
}

impl Default for SemVer {
    /// Initializes version with "0.1.0"
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
        }
    }
}

// Helper module to be able to parse SemVer from normal strings
pub mod semver_as_string {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &SemVer, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let version_str = format!("{}.{}.{}", value.major, value.minor, value.patch);
        serializer.serialize_str(&version_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SemVer, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(serde::de::Error::custom(
                "Expected version in 'major.minor.patch' format",
            ));
        }
        let major = parts[0].parse().map_err(serde::de::Error::custom)?;
        let minor = parts[1].parse().map_err(serde::de::Error::custom)?;
        let patch = parts[2].parse().map_err(serde::de::Error::custom)?;
        Ok(SemVer {
            major,
            minor,
            patch,
        })
    }
}
