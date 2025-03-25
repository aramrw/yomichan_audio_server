use clap::builder::OsStr;
use color_print::{cformat, cprintln, cwrite};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, Process, System};

use std::{
    env::{current_dir, current_exe},
    ffi::OsString, fs::canonicalize,
};
// needed for Command's 'creation_flags' method.
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::{cli::CliLog, PROGRAM_INFO};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub exit_minutes: u64,
    pub debug: bool,
}

#[allow(unused_mut)]
#[allow(clippy::zombie_processes)]
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
    cprintln!(
        "<g>+</> new background server:\n  pid: <b>{}</> name: <b>{:?}</>",
        handle.id(),
        exe.file_name()
            .unwrap_or(&OsString::from(cformat!("<r>'UNKNOWN'</>")))
    );
    // while let Some(l) = handle.stderr.take() {
    //     eprintln!("{l:?}");
    // }
}

fn find_process(sys: &mut System) -> Option<(&Pid, &Process)> {
    let current_pid = std::process::id();
    println!("current pid: {current_pid}");
    let current_exe = current_exe().unwrap();
    let exename = current_exe;
    sys.refresh_all();
    for (pid, proc) in sys.processes() {
        if let Some(exe) = proc.exe()  {
            if pid.as_u32() != current_pid && exe == exename {
                return Some((pid, proc));
            }
        }
    }
    if let Some(pi) = PROGRAM_INFO.get() {
        if pi.cli.log == CliLog::Dev || pi.cli.log == CliLog::Full {
            let str = cformat!(
                "no prev <r>{:?}</> executable found running",
                exename.file_name().unwrap()
            );
            tracing::info!(str);
        }
    }
    None
}

pub fn kill_previous_instance() {
    let mut sys = System::new_all();
    let Some((pid, proc)) = find_process(&mut sys) else {
        return;
    };
    cprintln!(
        "killing previous instance | <b>PID</>=<b>{pid}</>, <b>NAME</>=<b>{}</>",
        proc.name()
    );
    let cpid = std::process::id();
    if cpid != pid.as_u32() {
        proc.kill();
    }
}
