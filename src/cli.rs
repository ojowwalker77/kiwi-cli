use clap::{Parser, Subcommand};
use crate::{Result, Config, Homebrew, Dotfiles, Sync};
use std::path::PathBuf;
use colored::*;
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "kiwi")]
#[command(about = "CLI tool for managing macOS environment", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize or reconfigure the user's environment
    Init {
        /// Restore configurations and dotfiles from cloud backup
        #[arg(short, long)]
        restore: bool,
        /// Environment type (dev, prod, design)
        #[arg(short, long)]
        env: Option<String>,
        /// Sync Homebrew packages
        #[arg(short = 'b', long)]
        sync_homebrew: bool,
    },
    /// Sync configuration files between local and cloud
    Sync {
        /// Pull configurations from remote
        #[arg(long, conflicts_with = "push")]
        pull: bool,
        /// Push configurations to remote
        #[arg(long, conflicts_with = "pull")]
        push: bool,
        /// Prefer local files over cloud in case of conflicts
        #[arg(short, long)]
        prefer_local: bool,
    },
    /// Add a dotfile or configuration to sync
    Add {
        /// Path to the file to add
        path: String,
        /// Alias for the file
        #[arg(short, long)]
        alias: Option<String>,
    },
    /// Remove a dotfile or configuration from sync
    Remove {
        /// Path to the file to remove
        path: String,
    },
    /// Update packages and configurations
    Update {
        /// Update all dependencies and packages
        #[arg(short, long)]
        all: bool,
        /// Update a specific package
        #[arg(short, long)]
        package: Option<String>,
    },
    /// Install packages via Homebrew
    Install {
        /// Package name to install
        package: String,
    },
    /// List managed dotfiles and packages
    List {
        /// Type of items to list (dotfiles or packages)
        #[arg(short, long)]
        type_: Option<String>,
    },
    /// Manage global configuration
    Config {
        /// Configuration key
        key: Option<String>,
        /// Configuration value
        value: Option<String>,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        let mut config = Config::load()?;
        let homebrew = Homebrew::new(config.dotfiles_dir.join("packages.json"));
        let dotfiles = Dotfiles::new(
            config.dotfiles_dir.clone(),
            config.dotfiles_dir.join("dotfiles.json"),
        );

        // Clone the values we need before creating sync
        let sync_url = config.sync_url.clone();
        let sync_token = config.sync_token.clone();
        let dotfiles_dir = config.dotfiles_dir.clone();

        let sync = if let (Some(url), Some(token)) = (sync_url, sync_token) {
            Some(Sync::new(
                crate::sync::SyncConfig { url, token },
                dotfiles_dir,
            ))
        } else {
            None
        };

        match &self.command {
            Commands::Init { restore, env, sync_homebrew } => {
                println!("🥝 Welcome to Kiwi - Quick sync your packages and configs");
                println!("{}", "Initializing environment...".green().bold());
                
                if let Some(env_type) = env {
                    println!("{} {}", "Setting environment:".blue(), env_type);
                    config.set("environment", env_type.clone())?;
                }

                if *sync_homebrew {
                    println!("{}", "\nListing installed Homebrew packages:".blue().bold());
                    let packages = homebrew.list_installed()?;
                    
                    if packages.is_empty() {
                        println!("{}", "No Homebrew packages installed.".yellow());
                    } else {
                        println!("\n{}", "Formulas and Casks:".yellow());
                        for package in &packages {
                            let version_str = package.version.as_deref().unwrap_or("latest");
                            println!("  {} ({})", package.name, version_str);
                        }
                        
                        print!("\n{}", "Do you want to sync these packages? [y/N]: ".blue());
                        io::stdout().flush()?;
                        
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        
                        if input.trim().eq_ignore_ascii_case("y") {
                            println!("{}", "\nSyncing Homebrew packages...".yellow());
                            // Save the current packages to the packages.json file
                            homebrew.save_packages(&packages)?;
                            println!("{}", "✓ Homebrew packages synced".green());
                        }
                    }
                }

                if *restore {
                    println!("{}", "\nRestoring from backup...".yellow());
                    if let Some(sync) = &sync {
                        sync.pull(true).await?;
                        println!("{}", "✓ Restore complete".green());
                    }
                }
                
                println!("{}", "\n✓ Initialization complete".green());
            },
            Commands::Sync { pull, push, prefer_local } => {
                println!("{}", "Syncing configurations...".blue().bold());
                if let Some(sync) = &sync {
                    if *push {
                        println!("{}", "Preparing to push to remote...".yellow());
                        // Get list of installed Homebrew packages
                        let packages = homebrew.list_installed()?;
                        println!("\n{}", "Homebrew packages to sync:".yellow());
                        for package in &packages {
                            let version_str = package.version.as_deref().unwrap_or("latest");
                            println!("  {} ({})", package.name, version_str);
                        }
                        
                        // Save packages before pushing
                        homebrew.save_packages(&packages)?;
                        
                        println!("{}", "\nPushing to remote...".yellow());
                        sync.push().await?;
                        println!("{}", "✓ Push complete".green());
                    } else if *pull {
                        println!("{} {}", "Pulling from remote...".yellow(), 
                            if *prefer_local { "(preferring local files)" } else { "" });
                        sync.pull(*prefer_local).await?;
                        println!("{}", "✓ Pull complete".green());
                    } else {
                        println!("{}", "Please specify --push or --pull".red());
                    }
                } else {
                    println!("{}", "Sync not configured. Please set sync_url and sync_token in config.".red());
                }
            },
            Commands::Add { path, alias } => {
                println!("{} {}", "Adding file:".blue().bold(), path);
                dotfiles.add(PathBuf::from(path).as_path(), alias.clone())?;
                println!("{}", "✓ File added successfully".green());
            },
            Commands::Remove { path } => {
                println!("{} {}", "Removing file:".blue().bold(), path);
                dotfiles.remove(PathBuf::from(path).as_path())?;
                println!("{}", "✓ File removed successfully".green());
            },
            Commands::Update { all: update_all, package } => {
                println!("{}", "Updating packages...".blue().bold());
                if *update_all {
                    println!("{}", "Updating all packages...".yellow());
                    homebrew.update(None)?;
                } else if let Some(pkg) = package {
                    println!("{} {}", "Updating package:".yellow(), pkg);
                    homebrew.update(Some(pkg))?;
                }
                println!("{}", "✓ Update complete".green());
            },
            Commands::Install { package } => {
                println!("{} {}", "Installing package:".blue().bold(), package);
                homebrew.install(package)?;
                println!("{}", "✓ Installation complete".green());
            },
            Commands::List { type_ } => {
                println!("{}", "Listing items...".blue().bold());
                match type_.as_deref() {
                    Some("dotfiles") => {
                        println!("{}", "Managed dotfiles:".yellow());
                        let dotfiles = dotfiles.list()?;
                        for dotfile in dotfiles {
                            println!("  {}", dotfile.path.display());
                        }
                    },
                    Some("packages") => {
                        println!("{}", "Installed packages:".yellow());
                        let packages = homebrew.list_installed()?;
                        for package in packages {
                            println!("  {}", package.name);
                        }
                    },
                    _ => {
                        println!("{}", "Please specify type: dotfiles or packages".red());
                    },
                }
            },
            Commands::Config { key, value } => {
                println!("{}", "Managing configuration...".blue().bold());
                match (key, value) {
                    (Some(k), Some(v)) => {
                        println!("{} {} = {}", "Setting config:".yellow(), k, v);
                        config.set(k, v.clone())?;
                        println!("{}", "✓ Configuration updated".green());
                    },
                    (Some(k), None) => {
                        if let Some(v) = config.get(k) {
                            println!("{} = {}", k.yellow(), v);
                        } else {
                            println!("{} {}", "Config key not found:".red(), k);
                        }
                    },
                    (None, _) => {
                        println!("{}", "Please specify a config key".red());
                    },
                }
            },
        }

        Ok(())
    }
} 