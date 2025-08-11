use anyhow::Result;
use clap::{Arg, Command};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Stdio,
};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeSettings {
    #[serde(flatten)]
    settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeCodeRouterConfig {
    #[serde(flatten)]
    config: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
struct ConfigItem {
    name: String,
    path: PathBuf,
    config_type: ConfigType,
}

#[derive(Debug)]
enum ConfigType {
    Claude,
    CodeRouter,
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("claude-codust")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Claude Code configuration switcher")
        .arg(
            Arg::new("code")
                .help("Show interactive configuration selector")
                .action(clap::ArgAction::Set)
                .required(false),
        )
        .get_matches();

    if matches.contains_id("code") {
        show_interactive_selector().await?;
    } else {
        println!("Use 'code' to show interactive configuration selector");
    }

    Ok(())
}

async fn show_interactive_selector() -> Result<()> {
    let configs = load_configurations()?;
    
    if configs.is_empty() {
        println!("No configuration files found in ~/.claude/ or ~/.claude-code-router/");
        return Ok(());
    }

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let result = run_selector(&configs).await;

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

async fn run_selector(configs: &[ConfigItem]) -> Result<()> {
    let mut selected = 0;

    loop {
        print_selector_ui(configs, selected)?;

        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event::read()?
        {
            match code {
                KeyCode::Up => {
                    if selected == 0 {
                        selected = configs.len() - 1;
                    } else {
                        selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected == configs.len() - 1 {
                        selected = 0;
                    } else {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    switch_configuration(&configs[selected]).await?;
                    return Ok(());
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    println!("\r\nCancelled");
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

fn get_type_indicator(config_type: &ConfigType) -> &'static str {
    match config_type {
        ConfigType::Claude => "",
        ConfigType::CodeRouter => " [CCR]",
    }
}

fn print_selector_ui(configs: &[ConfigItem], selected: usize) -> Result<()> {
    // Move cursor to top and clear from cursor down
    execute!(io::stdout(), crossterm::cursor::MoveTo(0, 0))?;
    execute!(io::stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::FromCursorDown))?;

    println!(" Claude Code Configuration Selector");
    println!(" Use ↑/↓ to navigate, Enter to select, Esc/q to quit");
    println!();

    // Calculate max name length for alignment
    let max_name_len = configs.iter()
        .map(|c| c.name.len() + get_type_indicator(&c.config_type).len())
        .max()
        .unwrap_or(0);

    for (i, config) in configs.iter().enumerate() {
        let prefix = if i == selected { "❯ " } else { "  " };
        let type_indicator = get_type_indicator(&config.config_type);
        let name_with_indicator = format!("{}{}", config.name, type_indicator);
        println!("{}{:<width$} {}", prefix, name_with_indicator, config.path.display(), width = max_name_len);
    }

    io::stdout().flush()?;
    Ok(())
}

fn load_configurations() -> Result<Vec<ConfigItem>> {
    let mut configs = Vec::new();
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let claude_dir = home.join(".claude");
    if claude_dir.exists() {
        for entry in fs::read_dir(&claude_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with("-settings.json") {
                    let name = file_name.strip_suffix("-settings.json").unwrap().to_string();
                    configs.push(ConfigItem {
                        name,
                        path,
                        config_type: ConfigType::Claude,
                    });
                }
            }
        }
    }

    let router_dir = home.join(".claude-code-router");
    if router_dir.exists() {
        for entry in fs::read_dir(&router_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with("-config.json") {
                    let base_name = file_name.strip_suffix("-config.json").unwrap();
                    let name = format!("{}-ccr", base_name);
                    configs.push(ConfigItem {
                        name,
                        path,
                        config_type: ConfigType::CodeRouter,
                    });
                }
            }
        }
    }

    configs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(configs)
}

async fn switch_configuration(config: &ConfigItem) -> Result<()> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    
    match config.config_type {
        ConfigType::Claude => {
            //let target_path = home.join(".claude").join("settings.json");
            //fs::copy(&config.path, &target_path)?;
            println!("\r\nSwitched to Claude configuration: {}", config.name);
            //println!("\r\nCopied {} to {}", config.path.display(), target_path.display());
            
            // Launch claude command with original config env variables
            launch_claude_with_config(&config.path, &config.config_type).await?;
        }
        ConfigType::CodeRouter => {
            let target_path = home.join(".claude-code-router").join("config.json");
            
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            fs::copy(&config.path, &target_path)?;
            println!("\r\nSwitched to Claude Code Router configuration: {}", config.name);
            println!("\r\nCopied {} to {}", config.path.display(), target_path.display());
            
            // Run ccr restart command
            run_ccr_restart().await?;
            
            // Launch claude command with environment variables from config
            launch_claude_with_config(&target_path, &config.config_type).await?;
        }
    }
    
    Ok(())
}

async fn launch_claude_with_config(config_path: &PathBuf, config_type: &ConfigType) -> Result<()> {
    // Read and parse the configuration file
    let config_content = fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    // Extract environment variables from the config
    let mut env_vars = env::vars().collect::<HashMap<String, String>>();
    
    match config_type {
        ConfigType::Claude => {
            // For .claude configs, only use env variables from config.env if present
            if let Some(env_obj) = config.get("env").and_then(|e| e.as_object()) {
                for (key, value) in env_obj {
                    if let Some(value_str) = value.as_str() {
                        env_vars.insert(key.clone(), value_str.to_string());
                    }
                }
            }
        }
        ConfigType::CodeRouter => {
            // For .claude-code-router configs, set ANTHROPIC_API_KEY/AUTH_TOKEN and BASE_URL
            if let Some(api_key) = config.get("APIKEY").and_then(|k| k.as_str()) {
                env_vars.insert("ANTHROPIC_API_KEY".to_string(), api_key.to_string());
            } else {
                env_vars.insert("ANTHROPIC_AUTH_TOKEN".to_string(), "test".to_string());
            }
            
            let port = config.get("PORT")
                .and_then(|p| p.as_str())
                .unwrap_or("3456");
            let base_url = format!("http://127.0.0.1:{}", port);
            env_vars.insert("ANTHROPIC_BASE_URL".to_string(), base_url);
            
            // Also include any additional env variables from config.env if present
            // if let Some(env_obj) = config.get("env").and_then(|e| e.as_object()) {
            //     for (key, value) in env_obj {
            //         if let Some(value_str) = value.as_str() {
            //             env_vars.insert(key.clone(), value_str.to_string());
            //         }
            //     }
            // }
        }
    }
    
    // Find claude command path
    let claude_path = find_claude_command()?;
    
    println!("\r\nLaunching Claude with configuration environment...");
    
    // Launch claude command with environment variables (cross-platform)
    let mut child = if cfg!(target_os = "windows") {
        TokioCommand::new("cmd")
            .args(["/C", &claude_path])
            .envs(&env_vars)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
    } else {
        TokioCommand::new("sh")
            .args(["-c", &claude_path])
            .envs(&env_vars)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
    };
    
    // Handle process exit and cleanup for CodeRouter configs
    if matches!(config_type, ConfigType::CodeRouter) {
        let status = child.wait().await?;
        
        // Always stop CCR when claude process exits (regardless of exit reason)
        let _ = stop_ccr().await;
        
        if !status.success() {
            println!("\r\nClaude command exited with status: {}", status);
        }
    } else {
        let status = child.wait().await?;
        if !status.success() {
            println!("\r\nClaude command exited with status: {}", status);
        }
    }
    
    Ok(())
}

fn find_claude_command() -> Result<String> {
    let which_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    
    if let Ok(output) = std::process::Command::new(which_cmd).arg("claude").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }
    
    Ok("claude".to_string())
}

async fn run_ccr_restart() -> Result<()> {
    println!("\r\nRunning ccr restart...");
    
    let mut child = if cfg!(target_os = "windows") {
        TokioCommand::new("cmd")
            .args(["/C", "ccr", "restart"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
    } else {
        TokioCommand::new("sh")
            .args(["-c", "ccr restart"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
    };
    
    let status = child.wait().await?;
    
    if !status.success() {
        println!("\r\nWarning: ccr restart command exited with status: {}", status);
    } else {
        println!("\r\nccr restart completed successfully");
    }
    
    Ok(())
}

async fn stop_ccr() -> Result<()> {
    println!("\r\nStopping CCR...");
    
    let mut child = if cfg!(target_os = "windows") {
        TokioCommand::new("cmd")
            .args(["/C", "ccr", "stop"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
    } else {
        TokioCommand::new("sh")
            .args(["-c", "ccr stop"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
    };
    
    let status = child.wait().await?;
    
    if status.success() {
        println!("\r\nCCR stopped successfully");
    } else {
        println!("\r\nWarning: CCR stop command exited with status: {}", status);
    }
    
    Ok(())
}