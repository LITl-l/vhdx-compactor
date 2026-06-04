//! Drive the `wsl.exe` CLI: list distros, fstrim inside a distro, shutdown.

use std::os::windows::process::CommandExt;
use std::process::Command;
use vhdx_core::model::{ByteSize, Distro};
use vhdx_core::parse::parse_wsl_list;

/// CREATE_NO_WINDOW — keep `wsl.exe` from flashing a console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn wsl() -> Command {
    let mut c = Command::new("wsl.exe");
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

/// `wsl.exe` writes its console output as UTF-16LE; decode to a Rust `String`.
fn decode_utf16le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// List WSL2 distros, joining `wsl -l -v` state with registry vhdx paths + sizes.
pub fn list_distros() -> anyhow::Result<Vec<Distro>> {
    let out = wsl().args(["-l", "-v"]).output()?;
    let text = decode_utf16le(&out.stdout);
    let paths = super::registry::distro_vhdx_paths().unwrap_or_default();

    Ok(parse_wsl_list(&text)
        .into_iter()
        .map(|row| {
            let vhdx_path = paths.get(&row.name).cloned();
            let size = vhdx_path
                .as_ref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| ByteSize(m.len()));
            Distro {
                name: row.name,
                running: row.running,
                vhdx_path,
                size,
            }
        })
        .collect())
}

/// `fstrim -av` inside the distro as root, to discard freed ext4 blocks before
/// compaction. Requires the distro to be running.
pub fn fstrim(distro: &str) -> anyhow::Result<()> {
    let status = wsl()
        .args(["-d", distro, "-u", "root", "--", "fstrim", "-av"])
        .status()?;
    if !status.success() {
        anyhow::bail!("fstrim in {distro} exited with {status}");
    }
    Ok(())
}

/// `wsl --shutdown` — stops the entire WSL2 VM (all distros + Docker backend).
pub fn shutdown() -> anyhow::Result<()> {
    let status = wsl().arg("--shutdown").status()?;
    if !status.success() {
        anyhow::bail!("wsl --shutdown exited with {status}");
    }
    Ok(())
}
