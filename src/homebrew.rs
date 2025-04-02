use std::process::Command;
use crate::{Result, KiwiError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    pub installed: bool,
}

pub struct Homebrew {
    packages_file: PathBuf,
}

impl Homebrew {
    pub fn new(packages_file: PathBuf) -> Self {
        Self { packages_file }
    }

    pub fn install(&self, package: &str) -> Result<()> {
        let output = Command::new("brew")
            .arg("install")
            .arg(package)
            .output()?;

        if !output.status.success() {
            return Err(KiwiError::Homebrew(format!(
                "Failed to install package {}: {}",
                package,
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        self.add_package(package)?;
        Ok(())
    }

    pub fn update(&self, package: Option<&str>) -> Result<()> {
        let mut command = Command::new("brew");
        command.arg("upgrade");

        if let Some(pkg) = package {
            command.arg(pkg);
        }

        let output = command.output()?;

        if !output.status.success() {
            return Err(KiwiError::Homebrew(format!(
                "Failed to update package: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    pub fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("brew")
            .arg("list")
            .arg("--versions")
            .output()?;

        if !output.status.success() {
            return Err(KiwiError::Homebrew(format!(
                "Failed to list packages: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<Package> = output_str
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                Some(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).map(|&v| v.to_string()),
                    installed: true,
                })
            })
            .collect();

        Ok(packages)
    }

    fn add_package(&self, package: &str) -> Result<()> {
        let mut packages = self.load_packages()?;
        
        if !packages.iter().any(|p| p.name == package) {
            packages.push(Package {
                name: package.to_string(),
                version: None,
                installed: true,
            });
            self.save_packages(&packages)?;
        }

        Ok(())
    }

    fn load_packages(&self) -> Result<Vec<Package>> {
        if !self.packages_file.exists() {
            return Ok(Vec::new());
        }

        let contents = std::fs::read_to_string(&self.packages_file)?;
        let packages: Vec<Package> = serde_json::from_str(&contents)?;
        Ok(packages)
    }

    pub fn save_packages(&self, packages: &[Package]) -> Result<()> {
        let contents = serde_json::to_string_pretty(packages)?;
        std::fs::write(&self.packages_file, contents)?;
        Ok(())
    }
} 