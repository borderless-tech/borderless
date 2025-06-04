use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

/// Represents Git “describe” information in the form:
/// - `tag-<commits>-<hash>[-dirty]`
/// - or, if there is no tag, just `<hash>[-dirty]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInfo {
    pub tag: Option<String>,
    pub commits_past_tag: Option<usize>,
    pub commit_hash_short: String,
    pub dirty: bool,
}

impl GitInfo {
    pub fn new<T: Into<String>>(
        tag: Option<T>,
        commits_past_tag: Option<usize>,
        commit_hash_short: T,
        dirty: bool,
    ) -> Self {
        GitInfo {
            tag: tag.map(Into::into),
            commits_past_tag,
            commit_hash_short: commit_hash_short.into(),
            dirty,
        }
    }
}

impl fmt::Display for GitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(tag), Some(count)) = (&self.tag, &self.commits_past_tag) {
            // Format as "tag-<commits>-<hash>"
            if self.dirty {
                write!(f, "{}-{}-{}-dirty", tag, count, self.commit_hash_short)
            } else {
                write!(f, "{}-{}-{}", tag, count, self.commit_hash_short)
            }
        } else {
            // No tag: just emit the hash
            if self.dirty {
                write!(f, "{}-dirty", self.commit_hash_short)
            } else {
                write!(f, "{}", self.commit_hash_short)
            }
        }
    }
}

impl FromStr for GitInfo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty input string".into());
        }

        // Check for "-dirty" suffix
        let (base, dirty) = if let Some(stripped) = s.strip_suffix("-dirty") {
            if stripped.is_empty() {
                return Err("Empty GitInfo before `-dirty`".into());
            }
            (stripped, true)
        } else {
            (s, false)
        };

        // Attempt to parse "tag-<commits>-<hash>" by finding the last two hyphens in `base`.
        if let Some(idx1) = base.rfind('-') {
            let after_idx1 = &base[idx1 + 1..];
            if after_idx1.is_empty() {
                return Err("Empty hash after last '-'".into());
            }
            // Now find the previous '-' before idx1
            if let Some(idx2) = base[..idx1].rfind('-') {
                let tag_part = base[..idx2].trim();
                let commits_part = base[idx2 + 1..idx1].trim();
                let hash_part = after_idx1.trim();

                if tag_part.is_empty() {
                    return Err("Tag is empty before commits count".into());
                }
                if commits_part.is_empty() {
                    return Err("Commits-past-tag portion is empty".into());
                }
                // Parse commits count
                let commits_num: usize = commits_part.parse().map_err(|e: ParseIntError| {
                    format!("Invalid commits-past-tag `{}`: {}", commits_part, e)
                })?;

                // Validate that hash_part is hex
                if !hash_part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!("Invalid commit-hash `{}`", hash_part));
                }

                return Ok(GitInfo {
                    tag: Some(tag_part.to_string()),
                    commits_past_tag: Some(commits_num),
                    commit_hash_short: hash_part.to_string(),
                    dirty,
                });
            }
            // If we found one hyphen but not a second one, fall back to "only hash" logic below.
        }

        // Treat `base` as just a hash (no tag/commits)
        let hash_candidate = base;
        if !hash_candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid commit-hash `{}`", hash_candidate));
        }
        Ok(GitInfo {
            tag: None,
            commits_past_tag: None,
            commit_hash_short: hash_candidate.to_string(),
            dirty,
        })
    }
}

impl Serialize for GitInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let as_str = self.to_string();
        serializer.serialize_str(&as_str)
    }
}

struct GitInfoVisitor;

impl<'de> Visitor<'de> for GitInfoVisitor {
    type Value = GitInfo;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "a string of the form \"tag-<count>-<hash>[-dirty]\" or just \"<hash>[-dirty]\""
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<GitInfo, E>
    where
        E: DeError,
    {
        GitInfo::from_str(v).map_err(DeError::custom)
    }

    fn visit_string<E>(self, v: String) -> Result<GitInfo, E>
    where
        E: DeError,
    {
        GitInfo::from_str(&v).map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for GitInfo {
    fn deserialize<D>(deserializer: D) -> Result<GitInfo, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(GitInfoVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn gitinfo_display_with_tag_clean() {
        let gi = GitInfo::new(Some("v0.2.0"), Some(95), "5a85959", false);
        assert_eq!(gi.to_string(), "v0.2.0-95-5a85959");

        // Tag containing hyphens
        let gi2 = GitInfo::new(Some("release-1.2.3"), Some(4), "abcd123", false);
        assert_eq!(gi2.to_string(), "release-1.2.3-4-abcd123");
    }

    #[test]
    fn gitinfo_display_with_tag_dirty() {
        let gi = GitInfo::new(Some("v0.2.0"), Some(95), "5a85959", true);
        assert_eq!(gi.to_string(), "v0.2.0-95-5a85959-dirty");

        let gi2 = GitInfo::new(Some("release-1.2.3"), Some(4), "abcd123", true);
        assert_eq!(gi2.to_string(), "release-1.2.3-4-abcd123-dirty");
    }

    #[test]
    fn gitinfo_display_without_tag_clean() {
        let gi = GitInfo::new(None::<&str>, None, "deadbeef", false);
        assert_eq!(gi.to_string(), "deadbeef");
    }

    #[test]
    fn gitinfo_display_without_tag_dirty() {
        let gi = GitInfo::new(None::<&str>, None, "deadbeef", true);
        assert_eq!(gi.to_string(), "deadbeef-dirty");
    }

    #[test]
    fn gitinfo_from_str_with_tag_clean() {
        let s = "v0.2.0-95-5a85959";
        let gi: GitInfo = s.parse().expect("Parsing failed");
        assert_eq!(
            gi,
            GitInfo {
                tag: Some("v0.2.0".into()),
                commits_past_tag: Some(95),
                commit_hash_short: "5a85959".into(),
                dirty: false,
            }
        );

        // Hyphens in tag
        let s2 = "release-1.2.3-4-abcd123";
        let gi2: GitInfo = s2.parse().expect("Parsing failed");
        assert_eq!(
            gi2,
            GitInfo {
                tag: Some("release-1.2.3".into()),
                commits_past_tag: Some(4),
                commit_hash_short: "abcd123".into(),
                dirty: false,
            }
        );
    }

    #[test]
    fn gitinfo_from_str_with_tag_dirty() {
        let s = "v0.2.0-95-5a85959-dirty";
        let gi: GitInfo = s.parse().expect("Parsing failed");
        assert_eq!(
            gi,
            GitInfo {
                tag: Some("v0.2.0".into()),
                commits_past_tag: Some(95),
                commit_hash_short: "5a85959".into(),
                dirty: true,
            }
        );

        let s2 = "release-1.2.3-4-abcd123-dirty";
        let gi2: GitInfo = s2.parse().expect("Parsing failed");
        assert_eq!(
            gi2,
            GitInfo {
                tag: Some("release-1.2.3".into()),
                commits_past_tag: Some(4),
                commit_hash_short: "abcd123".into(),
                dirty: true,
            }
        );
    }

    #[test]
    fn gitinfo_from_str_without_tag_clean() {
        let s = "abcdef";
        let gi: GitInfo = s.parse().expect("Parsing failed");
        assert_eq!(
            gi,
            GitInfo {
                tag: None,
                commits_past_tag: None,
                commit_hash_short: "abcdef".into(),
                dirty: false,
            }
        );
    }

    #[test]
    fn gitinfo_from_str_without_tag_dirty() {
        let s = "abcdef-dirty";
        let gi: GitInfo = s.parse().expect("Parsing failed");
        assert_eq!(
            gi,
            GitInfo {
                tag: None,
                commits_past_tag: None,
                commit_hash_short: "abcdef".into(),
                dirty: true,
            }
        );
    }

    #[test]
    fn gitinfo_from_str_invalid() {
        // Missing hash after last hyphen
        let err = GitInfo::from_str("v1.0.0-5-").unwrap_err();
        assert!(err.contains("Empty hash"), "Unexpected error: {}", err);

        // Non-numeric commits
        let err2 = GitInfo::from_str("v1.0.0-xx-abcdef").unwrap_err();
        assert!(
            err2.contains("Invalid commits-past-tag"),
            "Unexpected error: {}",
            err2
        );

        // Invalid hex in hash
        let err3 = GitInfo::from_str("v1.0.0-5-ghijk").unwrap_err();
        assert!(
            err3.contains("Invalid commit-hash"),
            "Unexpected error: {}",
            err3
        );

        // Empty string
        let err4 = GitInfo::from_str("").unwrap_err();
        assert!(err4.contains("Empty input string"));

        // Just "-dirty"
        let err5 = GitInfo::from_str("-dirty").unwrap_err();
        assert!(err5.contains("Empty GitInfo before `-dirty`"));
    }

    #[test]
    fn gitinfo_serialize_deserialize() {
        // With tag, clean
        let gi = GitInfo::new(Some("v0.2.0"), Some(95), "5a85959", false);
        let json = serde_json::to_string(&gi).expect("Serialization failed");
        assert_eq!(json, r#""v0.2.0-95-5a85959""#);

        let parsed: GitInfo = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(parsed, gi);

        // With tag, dirty
        let gi2 = GitInfo::new(Some("v0.2.0"), Some(95), "5a85959", true);
        let json2 = serde_json::to_string(&gi2).expect("Serialization failed");
        assert_eq!(json2, r#""v0.2.0-95-5a85959-dirty""#);

        let parsed2: GitInfo = serde_json::from_str(&json2).expect("Deserialization failed");
        assert_eq!(parsed2, gi2);

        // Without tag, clean
        let gi3 = GitInfo::new(None::<&str>, None, "deadbeef", false);
        let json3 = serde_json::to_string(&gi3).expect("Serialization failed");
        assert_eq!(json3, r#""deadbeef""#);

        let parsed3: GitInfo = serde_json::from_str(&json3).expect("Deserialization failed");
        assert_eq!(parsed3, gi3);

        // Without tag, dirty
        let gi4 = GitInfo::new(None::<&str>, None, "deadbeef", true);
        let json4 = serde_json::to_string(&gi4).expect("Serialization failed");
        assert_eq!(json4, r#""deadbeef-dirty""#);

        let parsed4: GitInfo = serde_json::from_str(&json4).expect("Deserialization failed");
        assert_eq!(parsed4, gi4);
    }

    #[test]
    fn gitinfo_serde_error() {
        let bad = r#""v1.0.0-5-""#; // empty hash
        let res: Result<GitInfo, _> = serde_json::from_str(bad);
        assert!(res.is_err());

        let bad_dirty = r#""-dirty""#; // no base before "-dirty"
        let res2: Result<GitInfo, _> = serde_json::from_str(bad_dirty);
        assert!(res2.is_err());
    }
}
