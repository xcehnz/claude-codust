# Claude Codust

A cross-platform command-line tool for switching Claude Code and Claude Code Router configurations with an interactive interface.

## Features

- 🔄 Switch Claude Code configurations (`~/.claude` directory) and Claude Code Router configurations (`~/.claude-code-router` directory)
- 🎯 Interactive selection interface with arrow key navigation
- 🌍 Cross-platform support (Windows, macOS, Linux)
- ⚡ Automatic environment variable setup based on configuration type
- 🔧 Automatic `ccr restart` for Claude Code Router configurations

## Installation

### Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/your-username/claude-codust/releases):

- **Windows x64**: `claude-codust-windows-x64.exe`
- **Linux x64**: `claude-codust-linux-x64`
- **macOS x64**: `claude-codust-macos-x64`
- **macOS ARM64**: `claude-codust-macos-arm64`

### Build from Source

```bash
# Clone the repository
git clone https://github.com/your-username/claude-codust.git
cd claude-codust

# Build the project
cargo build --release

# The binary will be available at target/release/claude-codust
```

## Usage

### Interactive Configuration Selector

```bash
claude-codust --code
```

This will display an interactive interface where you can:
- Use ↑/↓ arrow keys to navigate between configurations
- Press Enter to select a configuration
- Press Esc or 'q' to quit

Example output:

```
 Claude Code Configuration Selector

 Use ↑/↓ to navigate, Enter to select, Esc/q to quit

❯ anyrouter        C:\Users\user\.claude\anyrouter-settings.json
  gemini-ccr [CCR] C:\Users\user\.claude-code-router\gemini-config.json
  k2               C:\Users\user\.claude\k2-settings.json
  openai-ccr [CCR] C:\Users\user\.claude-code-router\openai-config.json
  inst2api         C:\Users\user\.claude\inst-settings.json
  qwencoder3       C:\Users\user\.claude\qwencoder3-settings.json
```



### Configuration File Structure

The tool looks for configuration files in two directories:

#### Claude Configurations (`~/.claude/`)
- Files ending with `-settings.json`
- Example: `production-settings.json`, `development-settings.json`
- Environment variables are loaded from the `env` field in the JSON

#### Claude Code Router Configurations (`~/.claude-code-router/`)
- Files ending with `-config.json`
- Example: `gemini-config.json`, `openai-config.json`
- Displayed with `[CCR]` indicator and `-ccr` suffix
- Automatically sets:
  - `ANTHROPIC_API_KEY` (from `APIKEY` field) or `ANTHROPIC_AUTH_TOKEN: "test"` if no API key
  - `ANTHROPIC_BASE_URL: http://127.0.0.1:{PORT}