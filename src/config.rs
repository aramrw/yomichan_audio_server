use serde::{Deserialize, Serialize};
use sysinfo::{Pid, Process, System};

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
    let exe = &PROGRAM_INFO.get().unwrap().current_exe;

    #[cfg(target_os = "windows")]
    #[allow(clippy::zombie_processes)]
    let mut handle = std::process::Command::new(exe)
        .creation_flags(0x00000008) // CREATE_NO_WINDOW
        .args([
            "--audio",
            &audio_path.to_string_lossy(),
            "--log",
            "headless-instance",
        ])
        .spawn()
        .unwrap();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[allow(clippy::zombie_processes)]
    let mut handle = std::process::Command::new(exe)
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

    // print out the headless info
    println!("created headless with pid: {}", handle.id());
    // while let Some(l) = handle.stderr.take() {
    //     eprintln!("{l:?}");
    // }
}

fn find_process(sys: &mut System) -> Option<(&Pid, &Process)> {
    let current_exe = current_exe().unwrap();
    let name = current_exe.file_name().unwrap().to_string_lossy();
    sys.refresh_all();
    for (pid, proc) in sys.processes() {
        if proc.name() == name && pid.as_u32() != std::process::id() {
            return Some((pid, proc));
        }
    }
    println!("prev '{}' not found", name);
    None
}

pub fn kill_previous_instance() {
    let mut sys = System::new_all();
    let Some((pid, proc)) = find_process(&mut sys) else {
        return;
    };
    println!(
        "Killing previous instance with PID:{}, NAME:{}",
        pid,
        proc.name()
    );
    proc.kill();
}
