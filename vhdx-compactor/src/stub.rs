//! Non-Windows stand-in so the GUI builds and runs on the dev host.
//!
//! The real reclaim operations only exist on Windows; on other platforms the
//! app still launches (useful for eyeballing the UI), but every action returns
//! a clear "Windows only" error.
#![cfg(not(windows))]

use crate::platform::Platform;
use vhdx_core::model::{Distro, EngineUsed};

pub struct StubPlatform;

impl Platform for StubPlatform {
    fn list_distros(&self) -> anyhow::Result<Vec<Distro>> {
        Ok(Vec::new())
    }
    fn fstrim(&self, _distro: &str) -> anyhow::Result<()> {
        anyhow::bail!("WSL operations are only available on Windows")
    }
    fn shutdown(&self) -> anyhow::Result<()> {
        anyhow::bail!("WSL operations are only available on Windows")
    }
    fn vhdx_size(&self, _path: &str) -> anyhow::Result<u64> {
        Ok(0)
    }
    fn compact(&self, _path: &str, _progress: &mut dyn FnMut(u32)) -> anyhow::Result<EngineUsed> {
        anyhow::bail!("VHDX compaction is only available on Windows")
    }
}
