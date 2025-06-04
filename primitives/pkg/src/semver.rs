use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Semantic version
///
/// Is serialized as "major.minor.patch".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
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

impl Serialize for SemVer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let as_str = self.to_string();
        serializer.serialize_str(&as_str)
    }
}

struct SemVerVisitor;

impl<'de> Visitor<'de> for SemVerVisitor {
    type Value = SemVer;
    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a string of the form \"major.minor.patch\"")
    }

    fn visit_str<E>(self, v: &str) -> Result<SemVer, E>
    where
        E: DeError,
    {
        SemVer::from_str(v).map_err(DeError::custom)
    }

    fn visit_string<E>(self, v: String) -> Result<SemVer, E>
    where
        E: DeError,
    {
        SemVer::from_str(&v).map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for SemVer {
    fn deserialize<D>(deserializer: D) -> Result<SemVer, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(SemVerVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_to_string() {
        let version = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn semver_from_string() {
        let version = "1.1.23";
        let v = SemVer::from_str(&version);
        assert!(v.is_ok(), "{}", v.unwrap_err());
        assert_eq!(
            v.unwrap(),
            SemVer {
                major: 1,
                minor: 1,
                patch: 23
            }
        );
    }

    #[test]
    fn semver_deserialize() {
        let version = r#""1.1.23""#;
        let v: Result<SemVer, _> = serde_json::from_str(version);
        assert!(v.is_ok(), "{}", v.unwrap_err());
        assert_eq!(
            v.unwrap(),
            SemVer {
                major: 1,
                minor: 1,
                patch: 23
            }
        );
    }

    #[test]
    fn semver_serialize() {
        let version = SemVer {
            major: 1,
            minor: 1,
            patch: 23,
        };
        let s = serde_json::to_string(&version);
        assert!(s.is_ok(), "{}", s.unwrap_err());
        assert_eq!(s.unwrap(), r#""1.1.23""#);
    }

    #[test]
    fn semver_invalid_strings() {
        let invalid = ["1.23", "1.", "1", "asdf", "a.b.c"];
        for version in invalid {
            let v = SemVer::from_str(&version);
            assert!(v.is_err());
        }
    }

    #[test]
    fn semver_parsing() {
        let version = "1.0.0".parse::<SemVer>();
        assert!(version.is_ok());
        assert_eq!(
            version.unwrap(),
            SemVer {
                major: 1,
                minor: 0,
                patch: 0
            }
        );
        let version = "asdf".parse::<SemVer>();
        assert!(version.is_err());
        let version = "v1.0.3".parse::<SemVer>();
        assert!(version.is_err());
        let version = "1.0".parse::<SemVer>();
        assert!(version.is_err());
        let version = "1".parse::<SemVer>();
        assert!(version.is_err());
        let version = "1.0.-10".parse::<SemVer>();
        assert!(version.is_err());
    }

    #[test]
    fn semver_default() {
        let v1 = SemVer::default();
        assert_eq!(
            v1,
            SemVer {
                major: 0,
                minor: 1,
                patch: 0
            }
        );
        assert_eq!(v1, "0.1.0".parse().unwrap());
    }
}
