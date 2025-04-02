use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::{Result, KiwiError};
use std::fs;
use std::collections::HashMap;

const DEFAULT_SYNC_URL: &str = "http://34.41.188.73:8080";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub dotfiles_dir: PathBuf,
    pub sync_url: Option<String>,
    pub sync_token: Option<String>,
    pub environment: Option<String>,
    #[serde(default = "Preferences::default")]
    pub preferences: Preferences,
    #[serde(default)]
    pub custom_settings: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Preferences {
    #[serde(default = "default_auto_sync")]
    pub auto_sync: bool,
    #[serde(default = "default_backup_before_change")]
    pub backup_before_change: bool,
    #[serde(default = "default_check_updates_on_start")]
    pub check_updates_on_start: bool,
    #[serde(default = "default_show_progress_bars")]
    pub show_progress_bars: bool,
    #[serde(default = "default_verbose_output")]
    pub verbose_output: bool,
    #[serde(default = "default_max_parallel_downloads")]
    pub max_parallel_downloads: u32,
    #[serde(default = "default_backup_retention_days")]
    pub backup_retention_days: u32,
}

// Default value functions
fn default_auto_sync() -> bool { true }
fn default_backup_before_change() -> bool { true }
fn default_check_updates_on_start() -> bool { true }
fn default_show_progress_bars() -> bool { true }
fn default_verbose_output() -> bool { false }
fn default_max_parallel_downloads() -> u32 { 4 }
fn default_backup_retention_days() -> u32 { 30 }

impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_sync: default_auto_sync(),
            backup_before_change: default_backup_before_change(),
            check_updates_on_start: default_check_updates_on_start(),
            show_progress_bars: default_show_progress_bars(),
            verbose_output: default_verbose_output(),
            max_parallel_downloads: default_max_parallel_downloads(),
            backup_retention_days: default_backup_retention_days(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        Self {
            dotfiles_dir: home.join(".kiwi/dotfiles"),
            sync_url: Some(DEFAULT_SYNC_URL.to_string()),
            sync_token: None,
            environment: None,
            preferences: Preferences::default(),
            custom_settings: HashMap::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(&config_path).map_err(|e| {
            KiwiError::Config(format!("Failed to read config file: {}", e))
        })?;

        let config: Config = serde_json::from_str(&contents).map_err(|e| {
            KiwiError::Config(format!("Invalid config file format: {}", e))
        })?;

        // Validate and fix any issues
        config.validate()?;
        
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KiwiError::Config(format!("Failed to create config directory: {}", e))
            })?;
        }

        // Validate before saving
        self.validate()?;

        let contents = serde_json::to_string_pretty(self).map_err(|e| {
            KiwiError::Config(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&config_path, contents).map_err(|e| {
            KiwiError::Config(format!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            KiwiError::Config("Could not find home directory".to_string())
        })?;
        Ok(home.join(".kiwi/config.json"))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "dotfiles_dir" => Some(self.dotfiles_dir.to_str()?),
            "sync_url" => self.sync_url.as_deref(),
            "sync_token" => self.sync_token.as_deref(),
            "environment" => self.environment.as_deref(),
            _ => self.custom_settings.get(key).map(|s| s.as_str()),
        }
    }

    pub fn set(&mut self, key: &str, value: String) -> Result<()> {
        match key {
            "dotfiles_dir" => {
                let path = PathBuf::from(&value);
                if !path.exists() {
                    fs::create_dir_all(&path).map_err(|e| {
                        KiwiError::Config(format!("Failed to create dotfiles directory: {}", e))
                    })?;
                }
                self.dotfiles_dir = path;
            }
            "sync_url" => {
                // Validate URL format
                if !value.starts_with("http://") && !value.starts_with("https://") {
                    return Err(KiwiError::InvalidConfig {
                        key: key.to_string(),
                        message: "URL must start with http:// or https://".to_string(),
                    });
                }
                self.sync_url = Some(value);
            }
            "sync_token" => self.sync_token = Some(value),
            "environment" => {
                // Validate environment name
                if !value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    return Err(KiwiError::InvalidConfig {
                        key: key.to_string(),
                        message: "Environment name can only contain alphanumeric characters, underscores, and hyphens".to_string(),
                    });
                }
                self.environment = Some(value);
            }
            _ => {
                self.custom_settings.insert(key.to_string(), value);
            }
        }
        self.save()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // Validate dotfiles directory
        if !self.dotfiles_dir.exists() {
            fs::create_dir_all(&self.dotfiles_dir).map_err(|e| {
                KiwiError::Config(format!("Failed to create dotfiles directory: {}", e))
            })?;
        }

        // Validate sync URL if present
        if let Some(url) = &self.sync_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(KiwiError::InvalidConfig {
                    key: "sync_url".to_string(),
                    message: "URL must start with http:// or https://".to_string(),
                });
            }
        }

        // Validate environment name if present
        if let Some(env) = &self.environment {
            if !env.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(KiwiError::InvalidConfig {
                    key: "environment".to_string(),
                    message: "Environment name can only contain alphanumeric characters, underscores, and hyphens".to_string(),
                });
            }
        }

        // Validate preferences
        if self.preferences.max_parallel_downloads == 0 {
            return Err(KiwiError::InvalidConfig {
                key: "max_parallel_downloads".to_string(),
                message: "Must be greater than 0".to_string(),
            });
        }

        if self.preferences.backup_retention_days == 0 {
            return Err(KiwiError::InvalidConfig {
                key: "backup_retention_days".to_string(),
                message: "Must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    pub fn merge(&mut self, other: &Config) -> Result<()> {
        // Merge preferences
        self.preferences = other.preferences.clone();

        // Merge custom settings
        for (key, value) in &other.custom_settings {
            self.custom_settings.insert(key.clone(), value.clone());
        }

        // Only update optional fields if they are Some in other
        if other.sync_url.is_some() {
            self.sync_url = other.sync_url.clone();
        }
        if other.sync_token.is_some() {
            self.sync_token = other.sync_token.clone();
        }
        if other.environment.is_some() {
            self.environment = other.environment.clone();
        }

        // Validate the merged config
        self.validate()?;
        self.save()?;
        Ok(())
    }
} 