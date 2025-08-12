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

pub fn backup_settings_json_if_exists(home: &PathBuf) -> Result<()> {
    let claude_dir = home.join(".claude");
    let settings_path = claude_dir.join("settings.json");

    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut config: serde_json::Value = serde_json::from_str(&content)?;
        
        // Check if config has env key and remove it
        if config.get("env").is_some() {
            if let Some(obj) = config.as_object_mut() {
                obj.remove("env");
                println!("\r\nRemoved 'env' key from existing settings.json");
                
                // Write back the modified config
                let updated_content = serde_json::to_string_pretty(&config)?;
                fs::write(&settings_path, updated_content)?;
            }
        }
    }
    
    Ok(())
}