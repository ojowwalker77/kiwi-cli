# Kiwi Init

A powerful CLI tool for managing your macOS environment, dotfiles, and system configurations.

## Features

- Manage dotfiles with automatic symlinking
- Install and update packages via Homebrew
- Synchronize configurations across machines
- Environment-specific setup templates
- Backup and restore functionality

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/kiwi-init.git
cd kiwi-init

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

## Usage

### Initialize Environment

```bash
# Initialize with default settings
kiwi init

# Initialize with environment type
kiwi init --env dev

# Restore from backup
kiwi init --restore
```

### Manage Dotfiles

```bash
# Add a dotfile
kiwi add ~/.zshrc

# Add with custom alias
kiwi add ~/.vimrc --alias vimrc

# Remove a dotfile
kiwi remove ~/.zshrc

# List managed dotfiles
kiwi list --type dotfiles
```

### Package Management

```bash
# Install a package
kiwi install git

# Update all packages
kiwi update --all

# Update specific package
kiwi update --package git

# List installed packages
kiwi list --type packages
```

### Synchronization

```bash
# Sync with remote storage
kiwi sync

# Force push to remote
kiwi sync --force

# Prefer local files
kiwi sync --prefer-local
```

### Configuration

```bash
# Set configuration
kiwi config set sync_url https://api.example.com
kiwi config set sync_token your-token

# Get configuration
kiwi config get sync_url

# List all configurations
kiwi config list
```

## Configuration

The tool stores its configuration in `~/.kiwi/config.json`. You can manage the following settings:

- `dotfiles_dir`: Directory for storing dotfiles
- `sync_url`: URL for remote synchronization
- `sync_token`: Authentication token for remote sync
- `environment`: Current environment type

## Development

### Prerequisites

- Rust 1.70 or later
- macOS 10.15 or later
- Homebrew

### Building

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### Project Structure

- `src/cli.rs`: Command-line interface implementation
- `src/config.rs`: Configuration management
- `src/dotfiles.rs`: Dotfile management
- `src/homebrew.rs`: Homebrew package management
- `src/sync.rs`: Remote synchronization
- `src/error.rs`: Error handling

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details. 