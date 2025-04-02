use clap::{Parser, Subcommand, ValueEnum};
use crate::{Result, Config, Homebrew, Dotfiles, Sync};
use std::path::PathBuf;
use colored::*;
use std::io::{self, Write};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::fmt;
use std::time::Duration;

const SPINNER_TEMPLATE: &str = "{spinner:.green} {prefix:.bold.dim} {wide_msg}";
const PROGRESS_TEMPLATE: &str = "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {wide_msg}";
const PROGRESS_CHARS: &str = "█▉▊▋▌▍▎▏  ";

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum EnvType {
    Dev,
    Prod,
    Design,
    Custom,
}

impl fmt::Display for EnvType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvType::Dev => write!(f, "dev"),
            EnvType::Prod => write!(f, "prod"),
            EnvType::Design => write!(f, "design"),
            EnvType::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ListType {
    Dotfiles,
    Packages,
    All,
}

#[derive(Parser)]
#[command(name = "kiwi")]
#[command(about = "🥝 Kiwi - The Ultimate macOS Environment Manager", long_about = "A powerful CLI tool for seamlessly managing your macOS environment, including dotfiles, Homebrew packages, and cloud sync.")]
#[command(version)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize or reconfigure the user's environment
    Init {
        /// Restore configurations and dotfiles from cloud backup
        #[arg(short, long)]
        restore: bool,
        /// Environment type
        #[arg(short, long, value_enum)]
        env: Option<EnvType>,
        /// Custom environment name (when env is Custom)
        #[arg(short = 'n', long, requires = "env")]
        env_name: Option<String>,
        /// Sync Homebrew packages
        #[arg(short = 'b', long)]
        sync_homebrew: bool,
        /// Skip interactive prompts
        #[arg(short = 'y', long)]
        yes: bool,
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
        /// Force sync even if there are conflicts
        #[arg(short, long)]
        force: bool,
        /// Show a diff before syncing
        #[arg(short, long)]
        diff: bool,
    },
    /// Add a dotfile or configuration to sync
    Add {
        /// Path to the file to add
        path: String,
        /// Alias for the file
        #[arg(short, long)]
        alias: Option<String>,
        /// Create symlink automatically
        #[arg(short, long)]
        symlink: bool,
        /// Skip backup of existing file
        #[arg(short = 'B', long)]
        no_backup: bool,
    },
    /// Remove a dotfile or configuration from sync
    Remove {
        /// Path to the file to remove
        path: String,
        /// Remove the actual file as well
        #[arg(short, long)]
        delete: bool,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Update packages and configurations
    Update {
        /// Update all dependencies and packages
        #[arg(short, long)]
        all: bool,
        /// Update a specific package
        #[arg(short, long)]
        package: Option<String>,
        /// Skip outdated check
        #[arg(short, long)]
        force: bool,
        /// Show changelog when available
        #[arg(short, long)]
        changelog: bool,
    },
    /// Install packages via Homebrew
    Install {
        /// Package name to install
        package: String,
        /// Install without dependencies
        #[arg(short, long)]
        no_deps: bool,
        /// Install from a specific tap
        #[arg(short, long)]
        tap: Option<String>,
        /// Install a specific version
        #[arg(short, long)]
        version: Option<String>,
    },
    /// List managed dotfiles and packages
    List {
        /// Type of items to list
        #[arg(short, long, value_enum, default_value_t = ListType::All)]
        type_: ListType,
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
        /// Output in JSON format
        #[arg(short, long)]
        json: bool,
    },
    /// Manage global configuration
    Config {
        /// Configuration key
        key: Option<String>,
        /// Configuration value
        value: Option<String>,
        /// Reset configuration to defaults
        #[arg(short, long)]
        reset: bool,
        /// Export configuration
        #[arg(short, long)]
        export: bool,
        /// Import configuration from file
        #[arg(short, long)]
        import: Option<PathBuf>,
    },
    /// Check system health and configuration status
    Doctor {
        /// Fix detected issues automatically
        #[arg(short, long)]
        fix: bool,
        /// Generate a report
        #[arg(short, long)]
        report: bool,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        let mut config = Config::load()?;
        let mut homebrew = Homebrew::new(config.dotfiles_dir.join("packages.json"));
        let dotfiles = Dotfiles::new(
            config.dotfiles_dir.clone(),
            config.dotfiles_dir.join("dotfiles.json"),
        );

        // Set up progress indicators
        let multi_progress = MultiProgress::new();
        let spinner_style = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template(SPINNER_TEMPLATE)
            .unwrap();
        let progress_style = ProgressStyle::default_bar()
            .template(PROGRESS_TEMPLATE)
            .unwrap()
            .progress_chars(PROGRESS_CHARS);

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
            Commands::Init { restore, env, env_name, sync_homebrew, yes } => {
                println!("{}", "🥝 Welcome to Kiwi - The Ultimate macOS Environment Manager".green().bold());
                let spinner = multi_progress.add(ProgressBar::new_spinner());
                spinner.set_style(spinner_style.clone());
                spinner.set_prefix("[Init]");
                spinner.enable_steady_tick(Duration::from_millis(100));
                
                spinner.set_message("Initializing environment...");
                
                if let Some(env_type) = env {
                    let env_value = if *env_type == EnvType::Custom {
                        env_name.clone().unwrap_or_else(|| "custom".to_string())
                    } else {
                        env_type.to_string()
                    };
                    spinner.set_message(format!("Setting environment: {}", env_value));
                    config.set("environment", env_value)?;
                    spinner.tick();
                }

                if *sync_homebrew {
                    spinner.set_message("Scanning Homebrew packages...");
                    let packages = homebrew.list_installed()?;
                    
                    if packages.is_empty() {
                        spinner.finish_with_message("No Homebrew packages found to sync.");
                    } else {
                        let pb = multi_progress.add(ProgressBar::new(packages.len() as u64));
                        pb.set_style(progress_style.clone());
                        pb.set_prefix("[Packages]");
                        
                        for package in &packages {
                            pb.set_message(format!("Processing {}", package.name));
                            pb.inc(1);
                            std::thread::sleep(Duration::from_millis(50)); // Simulate work
                        }
                        
                        if !*yes {
                            pb.finish_and_clear();
                            print!("\n{}", "Do you want to sync these packages? [y/N]: ".blue());
                            io::stdout().flush()?;
                            
                            let mut input = String::new();
                            io::stdin().read_line(&mut input)?;
                            
                            if !input.trim().eq_ignore_ascii_case("y") {
                                println!("{}", "Skipping package sync".yellow());
                                return Ok(());
                            }
                        }
                        
                        spinner.set_message("Syncing Homebrew packages...");
                        homebrew.save_packages(&packages)?;
                        spinner.finish_with_message("✓ Homebrew packages synced successfully".green().to_string());
                    }
                }

                if *restore {
                    spinner.set_message("Restoring from backup...");
                    if let Some(sync) = &sync {
                        sync.pull(true).await?;
                        spinner.finish_with_message("✓ Restore completed successfully".green().to_string());
                    }
                }
                
                spinner.finish_with_message("✨ Initialization complete! Your environment is ready.".green().bold().to_string());
            },
            Commands::Sync { pull, push, prefer_local, force, diff } => {
                println!("{}", "Syncing configurations...".blue().bold());
                if let Some(sync) = &sync {
                    if *push {
                        println!("{}", "Preparing to push to remote...".yellow());
                        let packages = homebrew.list_installed()?;
                        
                        if *diff {
                            println!("\n{}", "Changes to be pushed:".blue());
                            // TODO: Implement diff view
                            println!("  {}", "Packages:".yellow());
                            for package in &packages {
                                println!("    + {}", package.name);
                            }
                        }
                        
                        if !*force && !*diff {
                            print!("\n{}", "Continue with push? [y/N]: ".blue());
                            io::stdout().flush()?;
                            let mut input = String::new();
                            io::stdin().read_line(&mut input)?;
                            if !input.trim().eq_ignore_ascii_case("y") {
                                println!("{}", "Push cancelled".yellow());
                                return Ok(());
                            }
                        }
                        
                        println!("\n{}", "Homebrew packages to sync:".yellow());
                        for package in &packages {
                            let version_str = package.version.as_deref().unwrap_or("latest");
                            println!("  {} ({})", package.name, version_str);
                        }
                        
                        homebrew.save_packages(&packages)?;
                        
                        println!("{}", "\nPushing to remote...".yellow());
                        sync.push().await?;
                        println!("{}", "✓ Push complete".green());
                    } else if *pull {
                        if *diff {
                            println!("\n{}", "Fetching remote changes...".blue());
                            // TODO: Implement remote diff view
                        }
                        
                        println!("{} {}", "Pulling from remote...".yellow(), 
                            if *prefer_local { "(preferring local files)" } else { "" });
                        
                        if *force {
                            println!("{}", "Force pulling (overwriting local changes)...".yellow());
                        }
                        
                        sync.pull(*prefer_local).await?;
                        println!("{}", "✓ Pull complete".green());
                    } else {
                        println!("{}", "Please specify --push or --pull".red());
                    }
                } else {
                    println!("{}", "Sync not configured. Please set sync_url and sync_token in config.".red());
                }
            },
            Commands::Add { path, alias, symlink, no_backup } => {
                println!("{} {}", "Adding file:".blue().bold(), path);
                
                let path = PathBuf::from(path);
                if !*no_backup && path.exists() {
                    let backup_path = path.with_extension("backup");
                    println!("{} {}", "Creating backup:".yellow(), backup_path.display());
                    std::fs::copy(&path, &backup_path)?;
                }
                
                dotfiles.add(path.as_path(), alias.clone())?;
                
                if *symlink {
                    println!("{}", "Creating symlink...".yellow());
                    // TODO: Implement symlink creation
                }
                
                println!("{}", "✓ File added successfully".green());
            },
            Commands::Remove { path, delete, force } => {
                println!("{} {}", "Removing file:".blue().bold(), path);
                
                let path = PathBuf::from(path);
                
                if *delete {
                    if !*force {
                        print!("{}", "Are you sure you want to delete the file? [y/N]: ".red());
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("{}", "Deletion cancelled".yellow());
                            return Ok(());
                        }
                    }
                    
                    if path.exists() {
                        std::fs::remove_file(&path)?;
                        println!("{}", "File deleted".yellow());
                    }
                }
                
                dotfiles.remove(path.as_path())?;
                println!("{}", "✓ File removed successfully".green());
            },
            Commands::Update { all: update_all, package, force, changelog } => {
                println!("{}", "Updating packages...".blue().bold());
                
                if *force {
                    println!("{}", "Force updating (skipping checks)...".yellow());
                }
                
                if *update_all {
                    println!("{}", "Updating all packages...".yellow());
                    homebrew.update(None)?;
                } else if let Some(pkg) = package {
                    println!("{} {}", "Updating package:".yellow(), pkg);
                    homebrew.update(Some(pkg))?;
                }
                
                if *changelog {
                    println!("{}", "\nFetching changelogs...".blue());
                    // TODO: Implement changelog fetching
                }
                
                println!("{}", "✓ Update complete".green());
            },
            Commands::Install { package, no_deps, tap, version } => {
                println!("{} {}", "Installing package:".blue().bold(), package);
                
                if let Some(tap_name) = tap {
                    println!("{} {}", "Using tap:".yellow(), tap_name);
                    // TODO: Implement tap handling
                }
                
                if let Some(ver) = version {
                    println!("{} {}", "Installing version:".yellow(), ver);
                    // TODO: Implement version-specific installation
                }
                
                if *no_deps {
                    println!("{}", "Installing without dependencies...".yellow());
                    // TODO: Implement no-deps installation
                }
                
                homebrew.install(package)?;
                println!("{}", "✓ Installation complete".green());
            },
            Commands::List { type_, detailed, json } => {
                if *json {
                    // TODO: Implement JSON output
                    println!("{}", "JSON output not yet implemented".yellow());
                    return Ok(());
                }
                
                println!("{}", "Listing items...".blue().bold());
                match type_ {
                    ListType::Dotfiles => {
                        println!("{}", "Managed dotfiles:".yellow());
                        let dotfiles = dotfiles.list()?;
                        for dotfile in dotfiles {
                            if *detailed {
                                println!("  Path: {}", dotfile.path.display());
                                // TODO: Add more detailed information
                            } else {
                                println!("  {}", dotfile.path.display());
                            }
                        }
                    },
                    ListType::Packages => {
                        println!("{}", "Installed packages:".yellow());
                        let packages = homebrew.list_installed()?;
                        for package in packages {
                            if *detailed {
                                let version = package.version.unwrap_or_else(|| "latest".to_string());
                                println!("  {} ({})", package.name, version);
                                // TODO: Add more package details
                            } else {
                                println!("  {}", package.name);
                            }
                        }
                    },
                    ListType::All => {
                        println!("{}", "Listing all items...".yellow());
                        let dotfiles = dotfiles.list()?;
                        let packages = homebrew.list_installed()?;
                        
                        println!("\n{}", "Dotfiles:".blue());
                        for dotfile in dotfiles {
                            if *detailed {
                                println!("  Path: {}", dotfile.path.display());
                                // TODO: Add more detailed information
                            } else {
                                println!("  {}", dotfile.path.display());
                            }
                        }
                        
                        println!("\n{}", "Packages:".blue());
                        for package in packages {
                            if *detailed {
                                let version = package.version.unwrap_or_else(|| "latest".to_string());
                                println!("  {} ({})", package.name, version);
                                // TODO: Add more package details
                            } else {
                                println!("  {}", package.name);
                            }
                        }
                    },
                }
            },
            Commands::Config { key, value, reset, export, import } => {
                println!("{}", "Managing configuration...".blue().bold());
                
                if *reset {
                    println!("{}", "Resetting configuration to defaults...".yellow());
                    config = Config::default();
                    config.save()?;
                    println!("{}", "✓ Configuration reset".green());
                    return Ok(());
                }
                
                if *export {
                    let config_json = serde_json::to_string_pretty(&config)?;
                    std::fs::write("kiwi-config.json", config_json)?;
                    println!("{}", "✓ Configuration exported to kiwi-config.json".green());
                    return Ok(());
                }
                
                if let Some(import_path) = import {
                    println!("{} {}", "Importing configuration from:".yellow(), import_path.display());
                    let config_json = std::fs::read_to_string(import_path)?;
                    config = serde_json::from_str(&config_json)?;
                    config.save()?;
                    println!("{}", "✓ Configuration imported".green());
                    return Ok(());
                }
                
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
            Commands::Doctor { fix, report } => {
                println!("{}", "🏥 Running system health check...".blue().bold());
                let spinner = ProgressBar::new_spinner();
                spinner.set_style(spinner_style);

                // Check configuration
                spinner.set_message("Checking configuration...");
                let config_issues = self.check_configuration(&config)?;

                // Check Homebrew
                spinner.set_message("Checking Homebrew installation...");
                let homebrew_issues = self.check_homebrew(&homebrew)?;

                // Check dotfiles
                spinner.set_message("Checking dotfiles...");
                let dotfile_issues = self.check_dotfiles(&dotfiles)?;

                // Check sync setup
                spinner.set_message("Checking sync configuration...");
                let sync_issues = self.check_sync(sync.as_ref()).await?;

                spinner.finish_and_clear();

                let all_issues = vec![
                    ("Configuration", config_issues),
                    ("Homebrew", homebrew_issues),
                    ("Dotfiles", dotfile_issues),
                    ("Sync", sync_issues),
                ];

                let total_issues: usize = all_issues.iter()
                    .map(|(_, issues)| issues.len())
                    .sum();

                if total_issues == 0 {
                    println!("{}", "✅ All systems operational!".green().bold());
                } else {
                    println!("\n{} {} issue(s) found:", "⚠️".yellow(), total_issues);
                    
                    for (category, issues) in &all_issues {
                        if !issues.is_empty() {
                            println!("\n{} {}:", "→".blue(), category);
                            for (i, issue) in issues.iter().enumerate() {
                                println!("  {}. {}", i + 1, issue);
                                
                                if *fix {
                                    if let Some(fix_msg) = self.try_fix_issue(category, issue, &config).await? {
                                        println!("     {}", fix_msg.green());
                                    }
                                }
                            }
                        }
                    }

                    if *report {
                        self.generate_health_report(&all_issues)?;
                        println!("\n{}", "📋 Health report generated: kiwi-health-report.md".green());
                    }

                    if !*fix {
                        println!("\n{}", "Run with --fix to attempt automatic repairs".yellow());
                    }
                }
            },
        }
        Ok(())
    }

    fn check_configuration(&self, config: &Config) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        if config.dotfiles_dir.to_string_lossy().is_empty() {
            issues.push("Dotfiles directory not configured".to_string());
        }
        
        if !config.dotfiles_dir.exists() {
            issues.push("Dotfiles directory does not exist".to_string());
        }
        
        // Check for required configuration values
        if config.sync_url.is_none() {
            issues.push("Sync URL not configured".to_string());
        }
        
        if config.sync_token.is_none() {
            issues.push("Sync token not configured".to_string());
        }
        
        Ok(issues)
    }

    fn check_homebrew(&self, homebrew: &Homebrew) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // Check if Homebrew is installed
        if !std::path::Path::new("/usr/local/bin/brew").exists() 
            && !std::path::Path::new("/opt/homebrew/bin/brew").exists() {
            issues.push("Homebrew is not installed".to_string());
        }
        
        // Check if packages.json exists and is valid
        if let Err(_) = homebrew.list_installed() {
            issues.push("Unable to read Homebrew packages".to_string());
        }
        
        Ok(issues)
    }

    fn check_dotfiles(&self, dotfiles: &Dotfiles) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // Check if dotfiles.json exists and is valid
        if let Ok(files) = dotfiles.list() {
            for file in files {
                if !file.path.exists() {
                    issues.push(format!("Dotfile not found: {}", file.path.display()));
                }
            }
        } else {
            issues.push("Unable to read dotfiles configuration".to_string());
        }
        
        Ok(issues)
    }

    async fn check_sync(&self, sync: Option<&Sync>) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        if sync.is_none() {
            issues.push("Sync is not configured".to_string());
            return Ok(issues);
        }
        
        // Check if we can access the remote
        if let Some(sync) = sync {
            if let Err(e) = sync.check_remote_access().await {
                issues.push(format!("Cannot access remote repository: {}", e));
            }
        }
        
        Ok(issues)
    }

    async fn try_fix_issue(&self, category: &str, issue: &str, config: &Config) -> Result<Option<String>> {
        match (category, issue) {
            ("Configuration", "Dotfiles directory does not exist") => {
                std::fs::create_dir_all(&config.dotfiles_dir)?;
                Ok(Some("Created dotfiles directory".to_string()))
            },
            ("Homebrew", "Homebrew is not installed") => {
                // Install Homebrew
                let install_script = "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"";
                std::process::Command::new("bash")
                    .arg("-c")
                    .arg(install_script)
                    .output()?;
                Ok(Some("Installed Homebrew".to_string()))
            },
            _ => Ok(None),
        }
    }

    fn generate_health_report(&self, issues: &[(&str, Vec<String>)]) -> Result<()> {
        let mut report = String::new();
        report.push_str("# Kiwi Health Report\n\n");
        report.push_str(&format!("Generated on: {}\n\n", chrono::Local::now()));
        
        for (category, category_issues) in issues {
            report.push_str(&format!("## {}\n\n", category));
            if category_issues.is_empty() {
                report.push_str("✅ No issues found\n\n");
            } else {
                for issue in category_issues {
                    report.push_str(&format!("- ⚠️ {}\n", issue));
                }
                report.push_str("\n");
            }
        }
        
        std::fs::write("kiwi-health-report.md", report)?;
        Ok(())
    }
} 