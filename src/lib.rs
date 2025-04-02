pub mod cli;
pub mod config;
pub mod dotfiles;
pub mod homebrew;
pub mod sync;
pub mod error;

pub use cli::Cli;
pub use config::Config;
pub use dotfiles::Dotfiles;
pub use homebrew::Homebrew;
pub use sync::Sync;
pub use error::KiwiError;

pub type Result<T> = std::result::Result<T, KiwiError>; 