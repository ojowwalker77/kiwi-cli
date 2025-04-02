use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::{Result, KiwiError};
use std::fs;
use dotenv::dotenv;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub dotfiles_dir: PathBuf,
    pub sync_url: Option<String>,
    pub sync_token: Option<String>,
    pub environment: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dotfiles_dir: PathBuf::from("/var/lib/docker/volumes/jonatas77walker_kiwi-data/_data/users"),
            sync_url: None,
            sync_token: None,
            environment: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv().ok();
        
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            let mut config = Config::default();
            if let Ok(url) = std::env::var("KIWI_SYNC_URL") {
                config.sync_url = Some(url);
            }
            if let Ok(token) = std::env::var("KIWI_AUTH_TOKEN") {
                config.sync_token = Some(token);
            }
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(config_path)?;
        let mut config: Config = serde_json::from_str(&contents)?;
        
        if let Ok(url) = std::env::var("KIWI_SYNC_URL") {
            config.sync_url = Some(url);
        }
        if let Ok(token) = std::env::var("KIWI_AUTH_TOKEN") {
            config.sync_token = Some(token);
        }
        
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| KiwiError::Config("Could not find home directory".to_string()))?;
        Ok(home.join(".kiwi/config.json"))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "dotfiles_dir" => Some(self.dotfiles_dir.to_str()?),
            "sync_url" => self.sync_url.as_deref(),
            "sync_token" => self.sync_token.as_deref(),
            "environment" => self.environment.as_deref(),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        match key {
            "dotfiles_dir" => self.dotfiles_dir = PathBuf::from(value),
            "sync_url" => self.sync_url = Some(value),
            "sync_token" => self.sync_token = Some(value),
            "environment" => self.environment = Some(value),
            _ => return Err(KiwiError::Config(format!("Unknown config key: {}", key))),
        }
        self.save()?;
        Ok(())
    }
} 