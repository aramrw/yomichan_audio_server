use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub exit_minutes: u64,
    pub debug: bool,
}

pub fn create_config() -> Config {
    let config_path = "config.json";
    if !Path::new(config_path).exists() {
       let config = Config {
            exit_minutes: 30,
            debug: false,
        }; 

        let config_json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(config_path, config_json).unwrap();
    }

    get_config()
}

fn get_config() -> Config {
    let config_path = "config.json";
    let config_json = std::fs::read_to_string(config_path).unwrap();
    let config: Config = serde_json::from_str(&config_json).unwrap();

    config
}
