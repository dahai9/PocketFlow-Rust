use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    pub providers: HashMap<String, ProviderConfig>,
    pub models: HashMap<String, ModelConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    #[serde(default = "default_auto_compact")]
    pub auto_compact: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self { auto_compact: default_auto_compact() }
    }
}

fn default_auto_compact() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    pub api_base: String,
    pub api_key_env: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub context_window: usize,
    pub compact_threshold: usize,
}

impl AppConfig {
    pub fn load<P: AsRef<Path>>(workspace: P) -> Result<Self> {
        let workspace = workspace.as_ref();
        let mut config_dir = workspace.to_path_buf();
        config_dir.push(".pi");
        
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        
        let mut config_path = config_dir.clone();
        config_path.push("config.toml");

        if !config_path.exists() {
            let default_toml = r#"[general]
auto_compact = true

[providers.openai]
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"

[providers.cerebras]
api_base = "http://127.0.0.1:4000/v1"
api_key_env = "OPENAI_API_KEY"

[models."gpt-4o"]
provider = "openai"
context_window = 128000
compact_threshold = 100000

[models."cerebras/qwen-3-235b-a22b-instruct-2507"]
provider = "cerebras"
context_window = 8192
compact_threshold = 6000
"#;
            fs::write(&config_path, default_toml.trim())?;
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path))?;
        
        let config: AppConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse config.toml")?;
            
        Ok(config)
    }

    pub fn config_dir<P: AsRef<Path>>(workspace: P) -> std::path::PathBuf {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(".pi");
        path
    }

    pub fn logs_dir<P: AsRef<Path>>(workspace: P) -> std::path::PathBuf {
        let mut path = Self::config_dir(workspace);
        path.push("logs");
        path
    }
}
