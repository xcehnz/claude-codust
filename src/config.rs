use anyhow::Result;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeSettings {
    #[serde(flatten)]
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeCodeRouterConfig {
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct ConfigItem {
    pub name: String,
    pub path: PathBuf,
    pub config_type: ConfigType,
}

#[derive(Debug)]
pub enum ConfigType {
    Claude,
    CodeRouter,
}

impl ConfigType {
    pub fn get_indicator(&self) -> &'static str {
        match self {
            ConfigType::Claude => "",
            ConfigType::CodeRouter => " [CCR]",
        }
    }
}

pub fn load_configurations() -> Result<Vec<ConfigItem>> {
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

    configs.sort_by(|a, b| {
        match (&a.config_type, &b.config_type) {
            (ConfigType::Claude, ConfigType::CodeRouter) => std::cmp::Ordering::Less,
            (ConfigType::CodeRouter, ConfigType::Claude) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    Ok(configs)
}

pub fn backup_settings_json_if_exists(home: &PathBuf, config_path: &PathBuf) -> Result<()> {
    let claude_dir = home.join(".claude");
    let settings_path = claude_dir.join("settings.json");

    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut config: serde_json::Value = serde_json::from_str(&content)?;
        
        // Check if config has env key and remove specific ANTHROPIC keys
        if let Some(env_obj) = config.get_mut("env").and_then(|e| e.as_object_mut()) {
            let anthropic_keys = ["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"];
            let mut removed_keys = Vec::new();
            
            for key in anthropic_keys {
                if env_obj.remove(key).is_some() {
                    removed_keys.push(key);
                }
            }
            
            if !removed_keys.is_empty() {
                println!("\r\nRemoved API keys from settings.json env: {:?}", removed_keys);
                
                // If env object is now empty, remove the entire env key
                if env_obj.is_empty() {
                    if let Some(obj) = config.as_object_mut() {
                        obj.remove("env");
                        println!("\r\nRemoved empty 'env' key from settings.json");
                    }
                }
                
                // Write back the modified config
                let updated_content = serde_json::to_string_pretty(&config)?;
                fs::write(&settings_path, updated_content)?;
            }
        }
    }

    // Extract non-env keys to local settings
    let config_content = fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    if let Some(obj) = config.as_object() {
        // Create a new object with all keys except 'env'
        let mut local_settings = serde_json::Map::new();
        for (key, value) in obj {
            if key != "env" {
                local_settings.insert(key.clone(), value.clone());
            }
        }
        
        // Only write if there are non-env keys
        if !local_settings.is_empty() {
            let current_dir = std::env::current_dir()?;
            let local_claude_dir = current_dir.join(".claude");
            
            // Create .claude directory if it doesn't exist
            fs::create_dir_all(&local_claude_dir)?;
            
            let local_settings_path = local_claude_dir.join("settings.local.json");
            let local_config = serde_json::Value::Object(local_settings);
            let local_content = serde_json::to_string_pretty(&local_config)?;
            
            fs::write(&local_settings_path, local_content)?;
            println!("\r\nCreated local settings at: {}", local_settings_path.display());
        }
    }
    
    Ok(())
}