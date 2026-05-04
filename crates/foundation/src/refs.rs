use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::namespace::{validate_slug, NamespaceId};
use crate::FoundationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandlePath(Vec<String>);

impl HandlePath {
    pub fn new(
        parts: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, FoundationError> {
        let parts = parts.into_iter().map(Into::into).collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "handle_path",
            });
        }
        for part in &parts {
            validate_slug("handle_path", part)?;
        }
        Ok(Self(parts))
    }

    pub fn parts(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Display for HandlePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("/"))
    }
}

impl FromStr for HandlePath {
    type Err = FoundationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value.split('/').map(str::to_owned))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContributionRef {
    pub namespace: NamespaceId,
    pub day: u32,
    pub sequence: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CrossGraphRef {
    pub namespace: NamespaceId,
    pub day: u32,
    pub slug: Option<String>,
    pub sequence: u32,
}

impl FromStr for CrossGraphRef {
    type Err = FoundationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value.split('/').collect::<Vec<_>>();
        match parts.as_slice() {
            [namespace, day, sequence] => Ok(Self {
                namespace: NamespaceId::new(*namespace)?,
                day: parse_u32("day", day)?,
                slug: None,
                sequence: parse_u32("sequence", sequence)?,
            }),
            [namespace, day, slug, sequence] => {
                validate_slug("slug", slug)?;
                Ok(Self {
                    namespace: NamespaceId::new(*namespace)?,
                    day: parse_u32("day", day)?,
                    slug: Some((*slug).to_owned()),
                    sequence: parse_u32("sequence", sequence)?,
                })
            }
            _ => Err(FoundationError::InvalidFormat {
                field: "cross_graph_ref",
            }),
        }
    }
}

impl fmt::Display for CrossGraphRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.slug {
            Some(slug) => write!(
                f,
                "{}/{}/{}/{}",
                self.namespace.as_str(),
                self.day,
                slug,
                self.sequence
            ),
            None => write!(
                f,
                "{}/{}/{}",
                self.namespace.as_str(),
                self.day,
                self.sequence
            ),
        }
    }
}

fn parse_u32(field: &'static str, value: &str) -> Result<u32, FoundationError> {
    value
        .parse::<u32>()
        .map_err(|_| FoundationError::InvalidFormat { field })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mid_slug_cross_graph_ref() {
        let parsed = "playful/183/kitty/7".parse::<CrossGraphRef>().unwrap();

        assert_eq!(parsed.namespace.as_str(), "playful");
        assert_eq!(parsed.day, 183);
        assert_eq!(parsed.slug.as_deref(), Some("kitty"));
        assert_eq!(parsed.sequence, 7);
        assert_eq!(parsed.to_string(), "playful/183/kitty/7");
    }

    #[test]
    fn parses_legacy_three_part_ref() {
        let parsed = "playful/183/7".parse::<CrossGraphRef>().unwrap();

        assert_eq!(parsed.slug, None);
        assert_eq!(parsed.to_string(), "playful/183/7");
    }
}
