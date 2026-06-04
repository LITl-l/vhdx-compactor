//! Headless CLI mode: `vhdx-compactor list` and `vhdx-compactor compact <path>`.
//!
//! Lets the reclaim operations be scripted — and verified — without launching
//! the GUI. Exercises exactly the same `WindowsPlatform` code the GUI uses.
//!
//! `main.rs` already gates this module with `#[cfg(windows)]`.

use crate::platform::Platform;
use crate::windows::WindowsPlatform;
use vhdx_core::model::ByteSize;

/// Entry point for headless commands. Never returns (calls `process::exit`).
pub fn run(args: &[String]) -> ! {
    // Attach to the launching console so output is visible even in release
    // (GUI-subsystem) builds when invoked from a shell.
    unsafe {
        use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let platform = WindowsPlatform;
    let code = match args.first().map(String::as_str) {
        Some("list") => cmd_list(&platform),
        Some("compact") => match args.get(1) {
            Some(path) => cmd_compact(&platform, path),
            None => {
                eprintln!("usage: vhdx-compactor compact <vhdx-path>");
                64
            }
        },
        other => {
            eprintln!("unknown command: {:?}", other.unwrap_or(""));
            eprintln!("commands: list | compact <vhdx-path>");
            64
        }
    };
    std::process::exit(code);
}

fn cmd_list(p: &WindowsPlatform) -> i32 {
    match p.list_distros() {
        Ok(distros) => {
            for d in distros {
                println!(
                    "{}\t{}\t{}\t{}",
                    d.name,
                    if d.running { "Running" } else { "Stopped" },
                    d.size.map(ByteSize::human).unwrap_or_else(|| "-".into()),
                    d.vhdx_path.unwrap_or_default()
                );
            }
            0
        }
        Err(e) => {
            eprintln!("list failed: {e}");
            1
        }
    }
}

fn cmd_compact(p: &WindowsPlatform, path: &str) -> i32 {
    let before = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    println!("before: {before} bytes ({})", ByteSize(before).human());

    let mut last = u32::MAX;
    let result = p.compact(path, &mut |pct| {
        if pct != last {
            println!("progress: {pct}%");
            last = pct;
        }
    });

    match result {
        Ok(engine) => {
            let after = std::fs::metadata(path).map(|m| m.len()).unwrap_or(before);
            let saved = before.saturating_sub(after);
            println!("after:  {after} bytes ({})", ByteSize(after).human());
            println!(
                "saved:  {saved} bytes ({}) via {engine:?}",
                ByteSize(saved).human()
            );
            0
        }
        Err(e) => {
            eprintln!("compact failed: {e}");
            2
        }
    }
}
