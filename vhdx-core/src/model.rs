//! Portable domain types shared by the GUI and the pipeline.

/// A size in bytes with human-readable formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteSize(pub u64);

impl ByteSize {
    /// Saturating difference, used for "before − after = saved".
    pub fn saturating_sub(self, other: ByteSize) -> ByteSize {
        ByteSize(self.0.saturating_sub(other.0))
    }

    /// Render as B / KiB / MiB / GiB / TiB, with one decimal above 1 KiB.
    pub fn human(self) -> String {
        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
        let mut value = self.0 as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        if unit == 0 {
            format!("{} {}", self.0, UNITS[unit])
        } else {
            format!("{value:.1} {}", UNITS[unit])
        }
    }
}

/// A WSL2 distribution discovered on the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distro {
    pub name: String,
    pub running: bool,
    /// Absolute path to the distro's `ext4.vhdx`, if resolved from the registry.
    pub vhdx_path: Option<String>,
    pub size: Option<ByteSize>,
}

/// One stage of the reclaim pipeline (used for progress + error reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStep {
    Discover,
    Trim,
    Shutdown,
    Compact,
}

/// Which compaction backend actually ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineUsed {
    Virtdisk,
    Diskpart,
}

/// Events emitted by the worker pipeline to the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEvent {
    Discovered(Vec<Distro>),
    TrimStarted {
        distro: String,
    },
    TrimDone {
        distro: String,
    },
    ShutdownDone,
    CompactProgress {
        distro: String,
        pct: u32,
    },
    StepFailed {
        distro: String,
        step: PipelineStep,
        err: String,
    },
    Done {
        distro: String,
        saved: ByteSize,
        engine: EngineUsed,
    },
    Finished,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_human() {
        assert_eq!(ByteSize(0).human(), "0 B");
        assert_eq!(ByteSize(1023).human(), "1023 B");
        assert_eq!(ByteSize(1024).human(), "1.0 KiB");
        assert_eq!(ByteSize(1536).human(), "1.5 KiB");
        assert_eq!(ByteSize(5 * 1024 * 1024 * 1024).human(), "5.0 GiB");
    }

    #[test]
    fn saved_delta_is_before_minus_after() {
        let saved = ByteSize(10 * 1024).saturating_sub(ByteSize(4 * 1024));
        assert_eq!(saved, ByteSize(6 * 1024));
    }

    #[test]
    fn saturating_sub_never_underflows() {
        assert_eq!(ByteSize(4).saturating_sub(ByteSize(10)), ByteSize(0));
    }
}
