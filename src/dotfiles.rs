use std::path::{Path, PathBuf};
use std::fs;
use crate::{Result, KiwiError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Dotfile {
    pub path: PathBuf,
    pub alias: Option<String>,
    pub synced: bool,
}

pub struct Dotfiles {
    dotfiles_dir: PathBuf,
    dotfiles_file: PathBuf,
}

impl Dotfiles {
    pub fn new(dotfiles_dir: PathBuf, dotfiles_file: PathBuf) -> Self {
        Self {
            dotfiles_dir,
            dotfiles_file,
        }
    }

    pub fn add(&self, path: &Path, alias: Option<String>) -> Result<()> {
        let path = path.canonicalize()?;
        
        if !path.exists() {
            return Err(KiwiError::Dotfiles(format!("File does not exist: {}", path.display())));
        }

        let mut dotfiles = self.load_dotfiles()?;
        
        if dotfiles.iter().any(|d| d.path == path) {
            return Err(KiwiError::Dotfiles(format!("File already tracked: {}", path.display())));
        }

        let dotfile = Dotfile {
            path: path.clone(),
            alias: alias.clone(),
            synced: false,
        };

        let target = self.dotfiles_dir.join(alias.unwrap_or_else(|| path.file_name().unwrap().to_string_lossy().to_string()));
        
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        if target.exists() {
            fs::remove_file(&target)?;
        }

        std::os::unix::fs::symlink(&path, &target)?;

        dotfiles.push(dotfile);
        self.save_dotfiles(&dotfiles)?;

        Ok(())
    }

    pub fn remove(&self, path: &Path) -> Result<()> {
        let path = path.canonicalize()?;
        let mut dotfiles = self.load_dotfiles()?;

        if let Some(index) = dotfiles.iter().position(|d| d.path == path) {
            let dotfile = &dotfiles[index];
            
            if let Some(alias) = &dotfile.alias {
                let target = self.dotfiles_dir.join(alias);
                if target.exists() {
                    fs::remove_file(target)?;
                }
            } else {
                let target = self.dotfiles_dir.join(path.file_name().unwrap());
                if target.exists() {
                    fs::remove_file(target)?;
                }
            }

            dotfiles.remove(index);
            self.save_dotfiles(&dotfiles)?;
        } else {
            return Err(KiwiError::Dotfiles(format!("File not tracked: {}", path.display())));
        }

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Dotfile>> {
        Ok(self.load_dotfiles()?)
    }

    pub fn sync(&self, _prefer_local: bool) -> Result<()> {
        let dotfiles = self.load_dotfiles()?;
        
        for dotfile in dotfiles {
            if !dotfile.synced {
                continue;
            }
        }

        Ok(())
    }

    fn load_dotfiles(&self) -> Result<Vec<Dotfile>> {
        if !self.dotfiles_file.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.dotfiles_file)?;
        let dotfiles: Vec<Dotfile> = serde_json::from_str(&contents)?;
        Ok(dotfiles)
    }

    fn save_dotfiles(&self, dotfiles: &[Dotfile]) -> Result<()> {
        let contents = serde_json::to_string_pretty(dotfiles)?;
        fs::write(&self.dotfiles_file, contents)?;
        Ok(())
    }
} 