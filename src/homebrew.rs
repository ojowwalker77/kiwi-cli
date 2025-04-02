use std::process::Command;
use crate::{Result, KiwiError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    pub installed: bool,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub install_time: Option<u64>,
    #[serde(default)]
    pub last_update: Option<u64>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub is_cask: bool,
}

pub struct Homebrew {
    packages_file: PathBuf,
    cache: HashMap<String, Package>,
}

impl Homebrew {
    pub fn new(packages_file: PathBuf) -> Self {
        let cache = if packages_file.exists() {
            match std::fs::read_to_string(&packages_file) {
                Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        Self { packages_file, cache }
    }

    pub fn install(&mut self, package: &str) -> Result<()> {
        // Check if package is already installed
        if self.is_installed(package)? {
            return Err(KiwiError::PackageError {
                name: package.to_string(),
                message: "Package is already installed".to_string(),
            });
        }

        // Check if it's a cask
        let is_cask = self.is_cask(package)?;
        let install_cmd = if is_cask { "install --cask" } else { "install" };

        let output = Command::new("brew")
            .args(install_cmd.split_whitespace())
            .arg(package)
            .output()?;

        if !output.status.success() {
            return Err(KiwiError::PackageError {
                name: package.to_string(),
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        self.add_package(package)?;
        Ok(())
    }

    pub fn update(&mut self, package: Option<&str>) -> Result<()> {
        let mut command = Command::new("brew");
        command.arg("upgrade");

        if let Some(pkg) = package {
            if !self.is_installed(pkg)? {
                return Err(KiwiError::PackageError {
                    name: pkg.to_string(),
                    message: "Package is not installed".to_string(),
                });
            }
            command.arg(pkg);
        }

        let output = command.output()?;

        if !output.status.success() {
            return Err(KiwiError::PackageError {
                name: package.unwrap_or("all").to_string(),
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        // Update package metadata
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(pkg) = package {
            if let Some(p) = self.cache.get_mut(pkg) {
                p.last_update = Some(now);
            }
        } else {
            for p in self.cache.values_mut() {
                p.last_update = Some(now);
            }
        }

        self.save_cache()?;
        Ok(())
    }

    pub fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("brew")
            .arg("list")
            .arg("--versions")
            .output()?;

        if !output.status.success() {
            return Err(KiwiError::Homebrew("Failed to list installed packages".to_string()));
        }

        let packages_str = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in packages_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let name = parts[0].to_string();
            let version = parts.get(1).map(|v| v.to_string());
            
            let mut package = Package {
                name: name.clone(),
                version,
                installed: true,
                dependencies: Vec::new(),
                install_time: None,
                last_update: None,
                size: None,
                is_cask: false,
            };

            // Get package info
            if let Ok(info) = self.get_package_info(&name) {
                package.dependencies = info.dependencies;
                package.size = info.size;
                package.is_cask = info.is_cask;
            }

            // Get cached metadata
            if let Some(cached) = self.cache.get(&name) {
                package.install_time = cached.install_time;
                package.last_update = cached.last_update;
            }

            packages.push(package);
        }

        Ok(packages)
    }

    fn is_installed(&self, package: &str) -> Result<bool> {
        let output = Command::new("brew")
            .arg("list")
            .arg(package)
            .output()?;

        Ok(output.status.success())
    }

    fn is_cask(&self, package: &str) -> Result<bool> {
        let output = Command::new("brew")
            .args(["info", "--cask", package])
            .output()?;

        Ok(output.status.success())
    }

    fn get_package_info(&self, package: &str) -> Result<Package> {
        let output = Command::new("brew")
            .args(["info", "--json=v2", package])
            .output()?;

        if !output.status.success() {
            return Err(KiwiError::PackageError {
                name: package.to_string(),
                message: "Failed to get package info".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct BrewInfo {
            dependencies: Vec<String>,
            installed: Vec<InstalledInfo>,
        }

        #[derive(Deserialize)]
        struct InstalledInfo {
            size: Option<u64>,
        }

        let info: BrewInfo = serde_json::from_slice(&output.stdout)?;

        Ok(Package {
            name: package.to_string(),
            version: None,
            installed: true,
            dependencies: info.dependencies,
            install_time: None,
            last_update: None,
            size: info.installed.first().and_then(|i| i.size),
            is_cask: false,
        })
    }

    fn add_package(&mut self, package: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut pkg = if let Ok(info) = self.get_package_info(package) {
            info
        } else {
            Package {
                name: package.to_string(),
                version: None,
                installed: true,
                dependencies: Vec::new(),
                install_time: Some(now),
                last_update: Some(now),
                size: None,
                is_cask: false,
            }
        };

        pkg.install_time = Some(now);
        pkg.last_update = Some(now);

        self.cache.insert(package.to_string(), pkg);
        self.save_cache()?;
        Ok(())
    }

    fn save_cache(&self) -> Result<()> {
        let contents = serde_json::to_string_pretty(&self.cache)?;
        std::fs::write(&self.packages_file, contents)?;
        Ok(())
    }

    pub fn save_packages(&mut self, packages: &[Package]) -> Result<()> {
        let mut cache = HashMap::new();
        for package in packages {
            cache.insert(package.name.clone(), package.clone());
        }
        
        self.cache = cache;
        self.save_cache()?;
        Ok(())
    }
} 