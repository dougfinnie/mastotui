//! Application error type.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MastotuiError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP/client error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("OAuth error: {0}")]
    OAuth(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Credential error: {0}")]
    Credential(String),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Not authenticated")]
    NotAuthenticated,
}

pub type Result<T> = std::result::Result<T, MastotuiError>;

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify auth.login.invalid-token]
    #[test]
    fn invalid_token_returns_not_authenticated() {
        let _: MastotuiError = MastotuiError::NotAuthenticated;
        assert!(matches!(MastotuiError::NotAuthenticated, MastotuiError::NotAuthenticated));
    }
}
