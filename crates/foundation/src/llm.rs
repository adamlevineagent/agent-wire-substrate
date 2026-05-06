use serde::{Deserialize, Serialize};

use crate::FoundationError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer5Provider {
    LmStudio,
    OpenRouter,
    DeterministicFixture,
    Unresolved,
}

impl Layer5Provider {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::LmStudio => "lm_studio",
            Self::OpenRouter => "openrouter",
            Self::DeterministicFixture => "deterministic_fixture",
            Self::Unresolved => "unresolved",
        }
    }

    pub fn adapter_id(&self) -> Layer5AdapterId {
        match self {
            Self::LmStudio => Layer5AdapterId::LmStudioChatCompletions,
            Self::OpenRouter => Layer5AdapterId::OpenRouterChatCompletions,
            Self::DeterministicFixture => Layer5AdapterId::FixtureLiveLlm,
            Self::Unresolved => Layer5AdapterId::Unresolved,
        }
    }

    pub fn parse(raw: &str) -> Result<Self, FoundationError> {
        match normalize_provider_slug(raw).as_str() {
            "lmstudio" | "lm_studio" => Ok(Self::LmStudio),
            "openrouter" => Ok(Self::OpenRouter),
            "deterministic_fixture" => Ok(Self::DeterministicFixture),
            "unresolved" => Ok(Self::Unresolved),
            other => Err(FoundationError::UnknownLiveLlmProvider {
                raw: other.to_owned(),
            }),
        }
    }
}

impl std::fmt::Display for Layer5Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layer5AdapterId {
    LmStudioChatCompletions,
    OpenRouterChatCompletions,
    FixtureLiveLlm,
    Unresolved,
}

impl Layer5AdapterId {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::LmStudioChatCompletions => "lm-studio-chat-completions",
            Self::OpenRouterChatCompletions => "openrouter-chat-completions",
            Self::FixtureLiveLlm => "fixture-live-llm-adapter",
            Self::Unresolved => "unresolved",
        }
    }
}

impl std::fmt::Display for Layer5AdapterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

fn normalize_provider_slug(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer5_provider_parses_closed_alias_set() {
        assert_eq!(
            Layer5Provider::parse("lm-studio").unwrap(),
            Layer5Provider::LmStudio
        );
        assert_eq!(
            Layer5Provider::parse("lmstudio").unwrap(),
            Layer5Provider::LmStudio
        );
        assert_eq!(
            Layer5Provider::parse("openrouter").unwrap(),
            Layer5Provider::OpenRouter
        );
        assert_eq!(
            Layer5Provider::parse("deterministic-fixture").unwrap(),
            Layer5Provider::DeterministicFixture
        );
    }

    #[test]
    fn layer5_provider_rejects_unknown_names() {
        assert_eq!(
            Layer5Provider::parse("some-new-provider"),
            Err(FoundationError::UnknownLiveLlmProvider {
                raw: "some_new_provider".to_owned()
            })
        );
    }

    #[test]
    fn layer5_provider_owns_adapter_mapping() {
        assert_eq!(
            Layer5Provider::LmStudio.adapter_id(),
            Layer5AdapterId::LmStudioChatCompletions
        );
        assert_eq!(
            Layer5Provider::OpenRouter.adapter_id().slug(),
            "openrouter-chat-completions"
        );
    }
}
