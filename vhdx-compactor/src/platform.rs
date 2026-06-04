//! Abstraction over the OS-specific operations the pipeline performs.
//!
//! The pipeline depends only on this trait, never on the Windows modules
//! directly. That dependency inversion is what makes the orchestration logic
//! in `pipeline.rs` unit-testable on a non-Windows host with a fake.

use vhdx_core::model::{Distro, EngineUsed};

pub trait Platform: Send + Sync {
    /// Discover WSL2 distros with name, running state, vhdx path + size.
    fn list_distros(&self) -> anyhow::Result<Vec<Distro>>;

    /// Run `fstrim` inside the (running) distro to discard freed ext4 blocks.
    fn fstrim(&self, distro: &str) -> anyhow::Result<()>;

    /// `wsl --shutdown` — stops the whole WSL2 VM (all distros). Idempotent.
    fn shutdown(&self) -> anyhow::Result<()>;

    /// On-disk size of a vhdx file, in bytes.
    fn vhdx_size(&self, path: &str) -> anyhow::Result<u64>;

    /// Compact the vhdx; `progress(pct)` is called with 0..=100. Returns which
    /// backend actually performed the compaction.
    fn compact(&self, path: &str, progress: &mut dyn FnMut(u32)) -> anyhow::Result<EngineUsed>;
}

/// Construct the platform implementation for the current OS.
pub fn current() -> Box<dyn Platform> {
    #[cfg(windows)]
    {
        Box::new(crate::windows::WindowsPlatform)
    }
    #[cfg(not(windows))]
    {
        Box::new(crate::stub::StubPlatform)
    }
}
