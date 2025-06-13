use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciIdentifier {
    pub registry: String,
    pub namespace: String,
    pub repository: String,
    pub tag: String,
}

impl OciIdentifier {
    pub fn new(registry: String, namespace: String, repository: String, tag: String) -> Self {
        Self {
            registry,
            namespace,
            repository,
            tag,
        }
    }
}

impl fmt::Display for OciIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        // Vollständige Form: registry/namespace/repository:tag
        if self.namespace.is_empty() {
            write!(f, "{}/{}:{}", self.registry, self.repository, self.tag)
        } else {
            write!(
                f,
                "{}/{}/{}:{}",
                self.registry, self.namespace, self.repository, self.tag
            )
        }
    }
}

impl FromStr for OciIdentifier {
    type Err = OciParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(OciParseError::Empty);
        }

        // 1. Tag extrahieren (nach dem letzten ':')
        let (image_part, tag) = if let Some(colon_pos) = s.rfind(':') {
            let tag_part = &s[colon_pos + 1..];

            // Prüfen ob es wirklich ein Tag ist oder Teil einer Registry mit Port
            if tag_part.contains('/') {
                // Das ':' gehört zur Registry (z.B. "localhost:5000/app")
                (s, "latest")
            } else {
                (&s[..colon_pos], tag_part)
            }
        } else {
            (s, "latest")
        };

        // 2. Registry, Namespace und Repository extrahieren
        let parts: Vec<&str> = image_part.split('/').collect();

        let (registry, namespace, repository) = match parts.len() {
            1 => {
                // Nur Repository: "nginx"
                (
                    "docker.io".to_string(),
                    "library".to_string(),
                    parts[0].to_string(),
                )
            }
            2 => {
                // Zwei Teile: könnte "user/repo" oder "registry.com/repo" sein
                if parts[0].contains('.') || parts[0].contains(':') {
                    // Registry ohne Namespace: "gcr.io/app"
                    (parts[0].to_string(), "".to_string(), parts[1].to_string())
                } else {
                    // User/Namespace: "myuser/app"
                    (
                        "docker.io".to_string(),
                        parts[0].to_string(),
                        parts[1].to_string(),
                    )
                }
            }
            _ => {
                // Drei oder mehr Teile: "registry/namespace/repo" oder "registry/namespace/subns/repo"
                let registry = parts[0].to_string();
                let repository = parts[parts.len() - 1].to_string();
                let namespace = parts[1..parts.len() - 1].join("/");
                (registry, namespace, repository)
            }
        };

        // 3. Validierung
        if repository.is_empty() {
            return Err(OciParseError::InvalidRepository);
        }

        // Repository Name validieren (OCI spec)
        if !is_valid_repository_name(&repository) {
            return Err(OciParseError::InvalidRepository);
        }

        // Namespace validieren (wenn nicht leer)
        if !namespace.is_empty() && !is_valid_namespace(&namespace) {
            return Err(OciParseError::InvalidNamespace);
        }

        // Tag validieren
        if !is_valid_tag(tag) {
            return Err(OciParseError::InvalidTag);
        }

        Ok(OciIdentifier {
            registry,
            namespace,
            repository,
            tag: tag.to_string(),
        })
    }
}

impl Serialize for OciIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialisiere als String
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for OciIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        OciIdentifier::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OciParseError {
    Empty,
    InvalidRepository,
    InvalidNamespace,
    InvalidTag,
    InvalidFormat,
}

impl fmt::Display for OciParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OciParseError::Empty => write!(f, "OCI identifier cannot be empty"),
            OciParseError::InvalidRepository => write!(f, "Invalid repository name"),
            OciParseError::InvalidNamespace => write!(f, "Invalid namespace"),
            OciParseError::InvalidTag => write!(f, "Invalid tag"),
            OciParseError::InvalidFormat => write!(f, "Invalid OCI identifier format"),
        }
    }
}

impl std::error::Error for OciParseError {}

// Hilfsfunktionen für Validierung nach OCI spec
fn is_valid_repository_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 255 {
        return false;
    }

    // Repository name: lowercase, alphanumeric, hyphens, underscores, periods
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.')
        && name.chars().next().unwrap().is_ascii_alphanumeric()
        && name.chars().last().unwrap().is_ascii_alphanumeric()
}

fn is_valid_namespace(namespace: &str) -> bool {
    if namespace.is_empty() {
        return true; // Leerer Namespace ist erlaubt
    }

    // Namespace kann Pfad-Segmente enthalten, jedes Segment muss valide sein
    for segment in namespace.split('/') {
        if !is_valid_repository_name(segment) {
            return false;
        }
    }
    true
}

fn is_valid_tag(tag: &str) -> bool {
    if tag.is_empty() || tag.len() > 128 {
        return false;
    }

    // Tag: alphanumeric, hyphens, underscores, periods (case-sensitive)
    tag.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !tag.starts_with('.')
        && !tag.starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_parse_simple() {
        let oci: OciIdentifier = "nginx".parse().unwrap();
        assert_eq!(oci.registry, "docker.io");
        assert_eq!(oci.namespace, "library");
        assert_eq!(oci.repository, "nginx");
        assert_eq!(oci.tag, "latest");
    }

    #[test]
    fn test_parse_with_tag() {
        let oci: OciIdentifier = "nginx:1.21".parse().unwrap();
        assert_eq!(oci.repository, "nginx");
        assert_eq!(oci.tag, "1.21");
    }

    #[test]
    fn test_parse_with_namespace() {
        let oci: OciIdentifier = "myuser/nginx:latest".parse().unwrap();
        assert_eq!(oci.registry, "docker.io");
        assert_eq!(oci.namespace, "myuser");
        assert_eq!(oci.repository, "nginx");
        assert_eq!(oci.tag, "latest");
    }

    #[test]
    fn test_parse_with_registry() {
        let oci: OciIdentifier = "gcr.io/project/nginx:v1.0".parse().unwrap();
        assert_eq!(oci.registry, "gcr.io");
        assert_eq!(oci.namespace, "project");
        assert_eq!(oci.repository, "nginx");
        assert_eq!(oci.tag, "v1.0");
    }

    #[test]
    fn test_parse_complex_namespace() {
        let oci: OciIdentifier = "registry.company.com/team/subteam/service:v2.1.0"
            .parse()
            .unwrap();
        assert_eq!(oci.registry, "registry.company.com");
        assert_eq!(oci.namespace, "team/subteam");
        assert_eq!(oci.repository, "service");
        assert_eq!(oci.tag, "v2.1.0");
    }

    #[test]
    fn test_parse_registry_with_port() {
        let oci: OciIdentifier = "localhost:5000/myapp".parse().unwrap();
        assert_eq!(oci.registry, "localhost:5000");
        assert_eq!(oci.namespace, "");
        assert_eq!(oci.repository, "myapp");
        assert_eq!(oci.tag, "latest");
    }

    #[test]
    fn test_display() {
        let oci = OciIdentifier::new(
            "gcr.io".to_string(),
            "project".to_string(),
            "nginx".to_string(),
            "v1.0".to_string(),
        );
        assert_eq!(oci.to_string(), "gcr.io/project/nginx:v1.0");
    }

    #[test]
    fn test_short_name() {
        let oci: OciIdentifier = "nginx:1.21".parse().unwrap();
        assert_eq!(oci.short_name(), "nginx:1.21");

        let oci: OciIdentifier = "myuser/nginx:latest".parse().unwrap();
        assert_eq!(oci.short_name(), "myuser/nginx:latest");

        let oci: OciIdentifier = "gcr.io/project/nginx:v1.0".parse().unwrap();
        assert_eq!(oci.short_name(), "gcr.io/project/nginx:v1.0");
    }

    #[test]
    fn test_serde() {
        let oci: OciIdentifier = "gcr.io/project/nginx:v1.0".parse().unwrap();

        // Serialize
        let json = serde_json::to_string(&oci).unwrap();
        assert_eq!(json, "\"gcr.io/project/nginx:v1.0\"");

        // Deserialize
        let deserialized: OciIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(oci, deserialized);
    }

    #[test]
    fn test_invalid_names() {
        assert!("".parse::<OciIdentifier>().is_err());
        assert!("UPPERCASE/app".parse::<OciIdentifier>().is_err());
        assert!("app:".parse::<OciIdentifier>().is_err());
        assert!("app:.invalid".parse::<OciIdentifier>().is_err());
    }
}
