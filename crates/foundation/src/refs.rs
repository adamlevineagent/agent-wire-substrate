use agent_wire_contracts::HandlePathDto;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphSlug(String);

impl GraphSlug {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandlePath {
    pub handle: String,
    pub wire_day: u32,
    pub graph_slug: Option<GraphSlug>,
    pub sequence: u32,
}

impl HandlePath {
    pub fn mainnet(handle: impl Into<String>, wire_day: u32, sequence: u32) -> Self {
        Self {
            handle: handle.into(),
            wire_day,
            graph_slug: None,
            sequence,
        }
    }

    pub fn cross_graph(
        handle: impl Into<String>,
        wire_day: u32,
        graph_slug: GraphSlug,
        sequence: u32,
    ) -> Self {
        Self {
            handle: handle.into(),
            wire_day,
            graph_slug: Some(graph_slug),
            sequence,
        }
    }
}

impl fmt::Display for HandlePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.graph_slug {
            Some(slug) => write!(
                f,
                "{}/{}/{}/{}",
                self.handle,
                self.wire_day,
                slug.as_str(),
                self.sequence
            ),
            None => write!(f, "{}/{}/{}", self.handle, self.wire_day, self.sequence),
        }
    }
}

impl From<HandlePathDto> for HandlePath {
    fn from(value: HandlePathDto) -> Self {
        Self {
            handle: value.handle,
            wire_day: value.wire_day,
            graph_slug: value.graph_slug.map(GraphSlug::new),
            sequence: value.sequence,
        }
    }
}

impl From<HandlePath> for HandlePathDto {
    fn from(value: HandlePath) -> Self {
        Self {
            handle: value.handle,
            wire_day: value.wire_day,
            graph_slug: value.graph_slug.map(GraphSlug::into_inner),
            sequence: value.sequence,
        }
    }
}
