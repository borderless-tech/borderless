use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Specifies the author of some package
///
/// The author should be serialized into:
/// "Author-Name <Author-E-Mail>"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
}

impl Author {
    pub fn new<S: AsRef<str>>(name: S, email: Option<S>) -> Self {
        Self {
            name: name.as_ref().to_string(),
            email: email.map(|s| s.as_ref().to_string()),
        }
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(mail) = &self.email {
            write!(f, "{} <{}>", self.name, mail)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl FromStr for Author {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(start) = s.find('<') {
            if let Some(end) = s[start + 1..].find('>') {
                let name_part = s[..start].trim();
                let email_part = &s[start + 1..start + 1 + end];
                if name_part.is_empty() {
                    return Err("Author name is empty".into());
                }
                if email_part.is_empty() {
                    return Err("E-mail is empty inside <..>".into());
                }
                Ok(Author {
                    name: name_part.to_string(),
                    email: Some(email_part.to_string()),
                })
            } else {
                Err("Missing closing '>' in author mail definition".into())
            }
        } else {
            // No '<', so entire string is the name
            let name_trim = s.trim();
            if name_trim.is_empty() {
                Err("Author string is empty".into())
            } else {
                Ok(Author {
                    name: name_trim.to_string(),
                    email: None,
                })
            }
        }
    }
}

impl Serialize for Author {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let as_str = self.to_string();
        serializer.serialize_str(&as_str)
    }
}

struct AuthorVisitor;

impl<'de> Visitor<'de> for AuthorVisitor {
    type Value = Author;
    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "a string of the form \"Name <email@…>\" or just \"Name\""
        )
    }
    fn visit_str<E>(self, v: &str) -> Result<Author, E>
    where
        E: DeError,
    {
        Author::from_str(v).map_err(DeError::custom)
    }
    fn visit_string<E>(self, v: String) -> Result<Author, E>
    where
        E: DeError,
    {
        Author::from_str(&v).map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for Author {
    fn deserialize<D>(deserializer: D) -> Result<Author, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(AuthorVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_to_string() {
        let author = Author::new("Klaus", Some("klaus@klausen.de"));
        assert_eq!(author.to_string(), "Klaus <klaus@klausen.de>");
        let author = Author::new("Klaus Kinski", None);
        assert_eq!(author.to_string(), "Klaus Kinski");
    }

    #[test]
    fn author_from_string() {
        let author = "Klaus <klaus@klausen.de>";
        let a = Author::from_str(&author);
        assert!(a.is_ok(), "{}", a.unwrap_err());
        assert_eq!(
            a.unwrap(),
            Author {
                name: "Klaus".to_string(),
                email: Some("klaus@klausen.de".to_string())
            }
        );
        let author = "Klaus Kinski";
        let a = Author::from_str(&author);
        assert!(a.is_ok(), "{}", a.unwrap_err());
        assert_eq!(
            a.unwrap(),
            Author {
                name: "Klaus Kinski".to_string(),
                email: None,
            }
        );
    }

    #[test]
    fn author_deserialize() {
        let author = r#""Klaus <klaus@klausen.de>""#;
        let a: Result<Author, _> = serde_json::from_str(author);
        assert!(a.is_ok(), "{}", a.unwrap_err());
        assert_eq!(
            a.unwrap(),
            Author {
                name: "Klaus".to_string(),
                email: Some("klaus@klausen.de".to_string())
            }
        );
        let author = r#""Klaus Kinski""#;
        let a: Result<Author, _> = serde_json::from_str(author);
        assert!(a.is_ok(), "{}", a.unwrap_err());
        assert_eq!(
            a.unwrap(),
            Author {
                name: "Klaus Kinski".to_string(),
                email: None,
            }
        );
    }

    #[test]
    fn author_serialize() {
        let author = Author::new("Klaus", Some("klaus@klausen.de"));
        let s = serde_json::to_string(&author);
        assert!(s.is_ok(), "{}", s.unwrap_err());
        assert_eq!(s.unwrap(), r#""Klaus <klaus@klausen.de>""#);

        let author = Author::new("Klaus Kinski", None);
        let s = serde_json::to_string(&author);
        assert!(s.is_ok(), "{}", s.unwrap_err());
        assert_eq!(s.unwrap(), r#""Klaus Kinski""#);
    }

    #[test]
    fn author_unescaped_email() {
        let author = "Klaus <foo";
        let a = Author::from_str(&author);
        assert!(a.is_err());
    }
}
