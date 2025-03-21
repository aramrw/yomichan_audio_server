use serde::{Deserialize, Serialize};

use std::env::{current_dir, current_exe};
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
    let binary_path = std::env::current_exe().unwrap();
    // Launch hidden instance
    #[cfg(target_os = "windows")]
    #[allow(clippy::zombie_processes)]
    std::process::Command::new(binary_path)
        .creation_flags(0x00000008) // CREATE_NO_WINDOW
        .args([
            "--audio",
            &audio_path.to_string_lossy(),
            "--log",
            "headless-instance",
        ])
        .spawn()
        .unwrap();

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new(binary_path)
            .args([
                "--audio",
                &audio_path.to_string_lossy(),
                "--log",
                "headless-instance",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
    }
}

pub fn kill_previous_instance() {
    let current_exe = current_exe().unwrap();
    let mut sys = sysinfo::System::new();
    sys.refresh_processes();

    for (pid, proc) in sys.processes() {
        if proc.name() == current_exe.file_name().unwrap().to_string_lossy()
            && pid.as_u32() != std::process::id()
        {
            println!("Killing previous instance with PID: {}", pid);
            proc.kill();
        }
    }
}
