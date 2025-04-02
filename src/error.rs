use thiserror::Error;

#[derive(Error, Debug)]
pub enum KiwiError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Homebrew error: {0}")]
    Homebrew(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Dotfiles error: {0}")]
    Dotfiles(String),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<&str> for KiwiError {
    fn from(error: &str) -> Self {
        KiwiError::Sync(error.to_string())
    }
}

impl From<String> for KiwiError {
    fn from(error: String) -> Self {
        KiwiError::Sync(error)
    }
} 