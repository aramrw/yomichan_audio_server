use serde::{Deserialize, Serialize};
use std::path::Path;
use std::os::windows::process::CommandExt;

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

#[cfg(target_os = "windows")]
pub fn handle_debugger(config: &Config) {
    let args: Vec<String> = std::env::args().collect();
    let is_secondary_instance = args.len() > 1 && args[1] == "debug";

    if !is_secondary_instance && !config.debug {
        // main terminal 
        std::process::Command::new("yomichan_audio_server.exe")
            .arg("debug")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .unwrap();

        std::process::exit(0);
    }
}

pub fn kill_previous_instance() {
    let mut sys = sysinfo::System::new_all();

    sys.refresh_all();

    for (pid, proc) in sys.processes() {
        if proc.name().contains("yomichan_audio_server") && pid.as_u32() != std::process::id() {
                println!("Killing previous instance with PID: {}", pid);
                proc.kill();
            }
    }
        
}
