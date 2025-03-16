use serde::{Deserialize, Serialize};

// needed for Command's 'creation_flags' method.
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::PROGRAM_INFO;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub exit_minutes: u64,
    pub debug: bool,
}

pub fn spawn_headless() {
    let audio_path = &PROGRAM_INFO.get().unwrap().cli.audio;
    // Launch hidden instance
    #[cfg(target_os = "windows")]
    #[allow(clippy::zombie_processes)]
    std::process::Command::new("yomichan_audio_server.exe")
        .creation_flags(0x00000008) // CREATE_NO_WINDOW
        .args(["--audio", &audio_path.to_string_lossy(), "--log", "headless-instance"])
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
