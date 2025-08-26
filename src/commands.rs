use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen},
};
use dirs::home_dir;
use std::{
    collections::HashMap,
    env, fs,
    io::{self},
    path::PathBuf,
    process::Stdio,
};
use tokio::process::Command as TokioCommand;

use crate::config::{ConfigItem, ConfigType};

fn cleanup_local_settings() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let local_settings_path = current_dir.join(".claude").join("settings.local.json");
    
    if local_settings_path.exists() {
        fs::remove_file(&local_settings_path)?;
        println!("\r\nCleaned up local settings file: {}", local_settings_path.display());
    }
    
    Ok(())
}

pub async fn switch_configuration(config: &ConfigItem) -> Result<()> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    
    match config.config_type {
        ConfigType::Claude => {
            crate::config::backup_settings_json_if_exists(&home, &config.path)?;
            
            println!("\r\nSwitched to Claude configuration: {}", config.name);
            
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
            
            run_ccr_restart().await?;
            
            launch_claude_with_config(&target_path, &config.config_type).await?;
        }
    }
    
    Ok(())
}

async fn launch_claude_with_config(config_path: &PathBuf, config_type: &ConfigType) -> Result<()> {
    let config_content = fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    let mut env_vars = env::vars().collect::<HashMap<String, String>>();
    match config_type {
        ConfigType::Claude => {
            if let Some(env_obj) = config.get("env").and_then(|e| e.as_object()) {
                for (key, value) in env_obj {
                    if let Some(value_str) = value.as_str() {
                        env_vars.insert(key.clone(), value_str.to_string());
                    }
                }
            }
        }
        ConfigType::CodeRouter => {
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
        }
    }
    
    let claude_path = find_claude_command()?;
    
    execute!(io::stdout(), Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    
    println!("Launching Claude with configuration environment...");
    
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
    
    if matches!(config_type, ConfigType::CodeRouter) {
        let status = child.wait().await?;
        
        let _ = stop_ccr().await;
        
        if !status.success() {
            eprintln!("Claude command exited with status: {}", status);
        }
    } else {
        let status = child.wait().await?;
        if !status.success() {
            eprintln!("Claude command exited with status: {}", status);
        }
        
        // Clean up local settings for Claude configurations
        let _ = cleanup_local_settings();
    }
    
    println!("\nClaude session completed. Press any key to exit...");
    
    enable_raw_mode()?;
    loop {
        if let Event::Key(_) = event::read()? {
            break;
        }
    }
    disable_raw_mode()?;
    
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