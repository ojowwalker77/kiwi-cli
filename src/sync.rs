use std::path::PathBuf;
use crate::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncConfig {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncData {
    pub files: std::collections::HashMap<String, String>,
    pub packages: Vec<crate::homebrew::Package>,
}

pub struct Sync {
    client: Client,
    config: SyncConfig,
    base_dir: PathBuf,
}

impl Sync {
    pub fn new(config: SyncConfig, base_dir: PathBuf) -> Self {
        Self {
            client: Client::new(),
            config,
            base_dir,
        }
    }

    pub async fn push(&self) -> Result<()> {
        let url = &self.config.url;
        
        let packages_file = self.base_dir.join("packages.json");
        let packages = if packages_file.exists() {
            let contents = fs::read_to_string(&packages_file)?;
            serde_json::from_str(&contents)?
        } else {
            Vec::new()
        };

        let sync_data = SyncData {
            files: std::collections::HashMap::new(),
            packages,
        };

        let response = self.client
            .post(url)
            .header("Authorization", self.get_auth_header())
            .json(&sync_data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to push: {}", response.status()).into());
        }
        Ok(())
    }

    pub async fn pull(&self, prefer_local: bool) -> Result<()> {
        if !self.base_dir.exists() && !prefer_local {
            return Err("Base directory does not exist".into());
        }

        let url = &self.config.url;
        let response = self.client
            .get(url)
            .header("Authorization", self.get_auth_header())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to pull: {}", response.status()).into());
        }

        let sync_data: SyncData = response.json().await?;
        
        if !sync_data.packages.is_empty() {
            let packages_file = self.base_dir.join("packages.json");
            fs::write(
                &packages_file,
                serde_json::to_string_pretty(&sync_data.packages)?,
            )?;
        }

        Ok(())
    }

    pub async fn sync_dotfiles(&self, _prefer_local: bool) -> Result<()> {
        Ok(())
    }

    pub async fn sync_packages(&self) -> Result<()> {
        Ok(())
    }

    fn get_auth_header(&self) -> String {
        format!("Bearer {}", self.config.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_config() {
        let config = SyncConfig {
            url: "https://api.example.com".to_string(),
            token: "test-token".to_string(),
        };
        let sync = Sync::new(config, PathBuf::from("/tmp"));
        assert_eq!(sync.get_auth_header(), "Bearer test-token");
    }
} 