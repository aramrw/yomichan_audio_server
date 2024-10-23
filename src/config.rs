use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{env::current_dir, path::Path};

#[derive(Debug, Serialize, Deserialize, Default)]
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

pub fn handle_debugger(hidden: bool) {
    if hidden {
        // Launch hidden instance
        #[cfg(target_os = "windows")]
        std::process::Command::new("yomichan_audio_server.exe")
            .arg("hidden")
            .creation_flags(0x00000008) // CREATE_NO_WINDOW
            .spawn()
            .unwrap();

        #[cfg(target_os = "macos")]
        {
            let binary_path = current_dir().unwrap().join("yomichan_audio_server");
            std::process::Command::new(binary_path)
                .arg("hidden")
                .spawn()
                .unwrap();
        }
    }
    if !hidden {
        // Launch visible instance
        #[cfg(target_os = "windows")]
        std::process::Command::new("yomichan_audio_server.exe")
            .arg("debug")
            .spawn()
            .unwrap();

        #[cfg(target_os = "macos")]
        {
            let binary_path = current_dir().unwrap().join("yomichan_audio_server");
            std::process::Command::new(binary_path)
                .arg("hidden")
                .spawn()
                .unwrap();
        }
    }
}

pub fn kill_previous_instance() {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes();

    for (pid, proc) in sys.processes() {
        if proc.name().contains("yomichan_audio_server") && pid.as_u32() != std::process::id() {
            println!("Killing previous instance with PID: {}", pid);
            proc.kill();
        }
    }
}
