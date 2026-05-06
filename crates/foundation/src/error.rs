use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FoundationError {
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },

    #[error("{field} contains an invalid character")]
    InvalidCharacter { field: &'static str },

    #[error("{field} has invalid format")]
    InvalidFormat { field: &'static str },

    #[error("{field} has unsupported scheme")]
    UnsupportedScheme { field: &'static str },

    #[error("{field} is outside the supported range")]
    OutOfRange { field: &'static str },

    #[error("{field} uses a reserved foundation name")]
    ReservedName { field: &'static str },

    #[error("unknown live LLM provider `{raw}`")]
    UnknownLiveLlmProvider { raw: String },
}
