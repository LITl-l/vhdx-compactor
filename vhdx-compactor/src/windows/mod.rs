//! Windows platform implementation: wires the `Platform` trait to the real
//! `wsl.exe`, registry, and virtdisk/diskpart modules.
//!
//! `main.rs` already gates this module with `#[cfg(windows)]`, so no inner cfg
//! is needed here.

mod compact;
mod registry;
mod wsl;

use crate::platform::Platform;
use vhdx_core::model::{Distro, EngineUsed};

pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn list_distros(&self) -> anyhow::Result<Vec<Distro>> {
        wsl::list_distros()
    }
    fn fstrim(&self, distro: &str) -> anyhow::Result<()> {
        wsl::fstrim(distro)
    }
    fn shutdown(&self) -> anyhow::Result<()> {
        wsl::shutdown()
    }
    fn vhdx_size(&self, path: &str) -> anyhow::Result<u64> {
        Ok(std::fs::metadata(path)?.len())
    }
    fn compact(&self, path: &str, progress: &mut dyn FnMut(u32)) -> anyhow::Result<EngineUsed> {
        compact::compact(path, progress)
    }
}
