use anyhow::Result;
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct PosConfig {
    pub listen_host: String,
    pub listen_port: u16,
    pub payment_url: String,
    pub accepted_mints: Vec<String>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct AppConfig {
    pub pos: PosConfig,
}

impl AppConfig {
    pub fn new<P>(config_file_name: Option<P>) -> Result<Self, ConfigError>
    where
        P: Into<PathBuf>,
    {
        let mut default_config_file_name = home::home_dir()
            .ok_or(ConfigError::NotFound("Config Path".to_string()))?
            .join(".cashu-pos");

        // Create the directory if it doesn't exist
        if !default_config_file_name.exists() {
            std::fs::create_dir_all(&default_config_file_name).map_err(|e| {
                ConfigError::Message(format!("Failed to create config directory: {}", e))
            })?;
        }

        default_config_file_name.push("config.toml");
        let config_path: PathBuf = match config_file_name {
            Some(value) => value.into(),
            None => default_config_file_name,
        };

        // Create example config if no config file exists
        if !config_path.exists() {
            let example_path = config_path.parent().unwrap().join("example.config.toml");
            if !example_path.exists() {
                let example_content = include_str!("../example.config.toml");
                std::fs::write(&example_path, example_content).map_err(|e| {
                    ConfigError::Message(format!("Failed to write example config: {}", e))
                })?;

                println!(
                    "Created example configuration at: {}",
                    example_path.display()
                );
                println!("Copy and modify this file to: {}", config_path.display());
            }
        }

        let default = &AppConfig::default();

        let builder = Config::builder();
        let config: Config = builder
            // use defaults
            .add_source(Config::try_from(default)?)
            // override with file contents
            .add_source(File::with_name(&config_path.to_string_lossy()))
            .build()?;
        let settings: AppConfig = config.try_deserialize()?;

        Ok(settings)
    }
}
