use clap::builder::OsStr;
use color_print::{cformat, cprintln, cwrite};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, Process, System};

use std::{
    env::{current_dir, current_exe},
    ffi::OsString,
    fs::canonicalize,
};
// needed for Command's 'creation_flags' method.
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::{
    cli::{Cli, CliLog},
    PROGRAM_INFO,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub exit_minutes: u64,
    pub debug: bool,
}

#[allow(unused_mut)]
#[allow(clippy::zombie_processes)]
pub fn spawn_headless(cli: &Cli) {
    let audio_path = &cli.audio;
    let exe = current_exe().expect("Failed to get current executable path");

    #[cfg(target_os = "windows")]
    let handle = std::process::Command::new(&exe)
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
    let handle = std::process::Command::new(&exe)
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

    cprintln!(
        "<g>+</> new background server:\n  pid: <b>{}</> name: <b>{:?}</>",
        handle.id(),
        exe.file_name()
            .unwrap_or(&OsString::from(cformat!("<r>'UNKNOWN'</>")))
    );
}

fn find_process(sys: &mut System) -> Option<(&Pid, &Process)> {
    let current_pid = std::process::id();
    
    // We still use canonicalize as it's good practice.
    let Ok(current_exe) = current_exe().and_then(canonicalize) else {
        return None; 
    };

    // We only need process info, not all system info.
    sys.refresh_processes();

    for (pid, proc) in sys.processes() {
        // 1. Skip the current process by PID.
        if pid.as_u32() == current_pid {
            continue;
        }

        // 2. Check for a matching executable path.
        if let Some(exe_path) = proc.exe() {
            if let Ok(canonical_exe) = canonicalize(exe_path) {
                if canonical_exe == current_exe {
                    // 3. ⭐ KEY CHANGE: Verify it's the server process
                    // by checking for the unique "headless-instance" argument.
                    if proc.cmd().iter().any(|arg| arg == "headless-instance") {
                        // This is the correct background process, return it.
                        return Some((pid, proc));
                    }
                }
            }
        }
    }
    
    // No running background server was found.
    None
}


// ✅ This function is now safe to call early because `find_process` is fixed.
pub fn kill_previous_instance() {
    let mut sys = System::new_all();
    let Some((pid, proc)) = find_process(&mut sys) else {
        return; // No process to kill
    };
    cprintln!(
        "killing previous instance | <b>PID</>=<b>{pid}</>, <b>NAME</>=<b>{}</>",
        proc.name()
    );
    proc.kill();
}

// this is not used for some reasn
// fn find_process(sys: &mut System) -> Option<(&Pid, &Process)> {
//     let current_pid = std::process::id();
//     println!("current pid: {current_pid}");
//     let current_exe = current_exe().unwrap();
//     let exename = current_exe;
//     sys.refresh_all();
//     for (pid, proc) in sys.processes() {
//         if let Some(exe) = proc.exe() {
//             if pid.as_u32() != current_pid && exe == exename {
//                 return Some((pid, proc));
//             }
//         }
//     }
//     if let Some(pi) = PROGRAM_INFO.get() {
//         if pi.cli.log == CliLog::Dev || pi.cli.log == CliLog::Full {
//             let str = cformat!(
//                 "no prev <r>{:?}</> executable found running",
//                 exename.file_name().unwrap()
//             );
//             tracing::info!(str);
//         }
//     }
//     None
// }

// this is not used for some reasn
// pub fn kill_previous_instance() {
//     let mut sys = System::new_all();
//     let Some((pid, proc)) = find_process(&mut sys) else {
//         return;
//     };
//     cprintln!(
//         "killing previous instance | <b>PID</>=<b>{pid}</>, <b>NAME</>=<b>{}</>",
//         proc.name()
//     );
//     let cpid = std::process::id();
//     if cpid != pid.as_u32() {
//         proc.kill();
//     }
// }
