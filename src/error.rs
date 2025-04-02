use thiserror::Error;
use std::path::PathBuf;

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

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Invalid configuration: {key} - {message}")]
    InvalidConfig { key: String, message: String },

    #[error("Package error: {name} - {message}")]
    PackageError { name: String, message: String },

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Operation cancelled by user")]
    UserCancelled,
}

impl KiwiError {
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            KiwiError::ValidationError(_) |
            KiwiError::UserCancelled |
            KiwiError::InvalidConfig { .. } |
            KiwiError::FileNotFound { .. }
        )
    }

    pub fn is_system_error(&self) -> bool {
        matches!(
            self,
            KiwiError::Io(_) |
            KiwiError::Network(_) |
            KiwiError::PermissionDenied { .. }
        )
    }

    pub fn suggestion(&self) -> Option<String> {
        match self {
            KiwiError::FileNotFound { path } => {
                Some(format!("Please check if the file exists at: {}", path.display()))
            }
            KiwiError::PermissionDenied { path } => {
                Some(format!("Try running with sudo or check file permissions at: {}", path.display()))
            }
            KiwiError::InvalidConfig { key, .. } => {
                Some(format!("Try updating the configuration with: kiwi config {} <value>", key))
            }
            KiwiError::PackageError { name, .. } => {
                Some(format!("Try running 'brew doctor' or 'brew update' before installing {}", name))
            }
            KiwiError::Network(_) => {
                Some("Check your internet connection and try again".to_string())
            }
            _ => None
        }
    }
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