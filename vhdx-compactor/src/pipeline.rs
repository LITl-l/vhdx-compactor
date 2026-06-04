//! Worker-thread orchestration of the reclaim pipeline.
//!
//! This is pure policy expressed over the `Platform` trait, so it is fully
//! unit-testable on the host with a fake platform. Error policy:
//!   - `fstrim` failure is **non-fatal** (warn + continue; already-zeroed
//!     blocks are still reclaimed).
//!   - `shutdown` failure is **fatal for the whole run** (a locked vhdx cannot
//!     be compacted).
//!   - `compact` failure aborts **only that distro**.

use crate::platform::Platform;
use std::sync::mpsc::Sender;
use vhdx_core::model::{ByteSize, PipelineEvent, PipelineStep};

/// Run trim → shutdown → compact for each selected distro, emitting events.
pub fn run_pipeline(platform: &dyn Platform, selected: &[String], tx: &Sender<PipelineEvent>) {
    // 1. Trim each selected distro while it is still mounted (non-fatal).
    for distro in selected {
        let _ = tx.send(PipelineEvent::TrimStarted {
            distro: distro.clone(),
        });
        match platform.fstrim(distro) {
            Ok(()) => {
                let _ = tx.send(PipelineEvent::TrimDone {
                    distro: distro.clone(),
                });
            }
            Err(e) => {
                let _ = tx.send(PipelineEvent::StepFailed {
                    distro: distro.clone(),
                    step: PipelineStep::Trim,
                    err: e.to_string(),
                });
            }
        }
    }

    // 2. Shut the WSL VM down once so the vhdx files are unlocked.
    if let Err(e) = platform.shutdown() {
        let _ = tx.send(PipelineEvent::StepFailed {
            distro: String::new(),
            step: PipelineStep::Shutdown,
            err: e.to_string(),
        });
        let _ = tx.send(PipelineEvent::Finished);
        return; // cannot compact a locked file
    }
    let _ = tx.send(PipelineEvent::ShutdownDone);

    // 3. Compact each distro's vhdx (compact failure aborts only that distro).
    let listing = platform.list_distros().unwrap_or_default();
    for distro in selected {
        let path = listing
            .iter()
            .find(|d| &d.name == distro)
            .and_then(|d| d.vhdx_path.clone());
        let Some(path) = path else {
            let _ = tx.send(PipelineEvent::StepFailed {
                distro: distro.clone(),
                step: PipelineStep::Compact,
                err: "vhdx path not found".into(),
            });
            continue;
        };

        let before = platform.vhdx_size(&path).unwrap_or(0);
        let distro_for_cb = distro.clone();
        let txc = tx.clone();
        let mut cb = move |pct: u32| {
            let _ = txc.send(PipelineEvent::CompactProgress {
                distro: distro_for_cb.clone(),
                pct,
            });
        };

        match platform.compact(&path, &mut cb) {
            Ok(engine) => {
                let after = platform.vhdx_size(&path).unwrap_or(before);
                let _ = tx.send(PipelineEvent::Done {
                    distro: distro.clone(),
                    saved: ByteSize(before).saturating_sub(ByteSize(after)),
                    engine,
                });
            }
            Err(e) => {
                let _ = tx.send(PipelineEvent::StepFailed {
                    distro: distro.clone(),
                    step: PipelineStep::Compact,
                    err: e.to_string(),
                });
            }
        }
    }

    let _ = tx.send(PipelineEvent::Finished);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::mpsc;
    use vhdx_core::model::{ByteSize, Distro, EngineUsed};

    struct FakePlatform {
        distros: Vec<Distro>,
        fail_fstrim_on: Option<String>,
        fail_compact_on: Option<String>,
        // Per-path size queue: first popped value = before, next = after.
        sizes: Mutex<HashMap<String, Vec<u64>>>,
    }

    impl Platform for FakePlatform {
        fn list_distros(&self) -> anyhow::Result<Vec<Distro>> {
            Ok(self.distros.clone())
        }
        fn fstrim(&self, distro: &str) -> anyhow::Result<()> {
            if self.fail_fstrim_on.as_deref() == Some(distro) {
                anyhow::bail!("fstrim boom");
            }
            Ok(())
        }
        fn shutdown(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn vhdx_size(&self, path: &str) -> anyhow::Result<u64> {
            let mut g = self.sizes.lock().unwrap();
            let q = g.get_mut(path).expect("size queue for path");
            Ok(q.remove(0))
        }
        fn compact(&self, path: &str, progress: &mut dyn FnMut(u32)) -> anyhow::Result<EngineUsed> {
            if self.fail_compact_on.as_deref() == Some(path) {
                anyhow::bail!("compact boom");
            }
            progress(50);
            progress(100);
            Ok(EngineUsed::Virtdisk)
        }
    }

    fn distro(name: &str, path: &str) -> Distro {
        Distro {
            name: name.into(),
            running: true,
            vhdx_path: Some(path.into()),
            size: Some(ByteSize(0)),
        }
    }

    #[test]
    fn fstrim_failure_is_non_fatal_and_compaction_proceeds() {
        let (tx, rx) = mpsc::channel();
        let plat = FakePlatform {
            distros: vec![distro("Ubuntu", "u.vhdx")],
            fail_fstrim_on: Some("Ubuntu".into()),
            fail_compact_on: None,
            sizes: Mutex::new(HashMap::from([("u.vhdx".to_string(), vec![10_000, 4_000])])),
        };
        run_pipeline(&plat, &["Ubuntu".to_string()], &tx);
        let events: Vec<PipelineEvent> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::StepFailed {
                step: PipelineStep::Trim,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::Done { saved, .. } if *saved == ByteSize(6_000)
        )));
    }

    #[test]
    fn compact_failure_is_fatal_for_that_distro_only() {
        let (tx, rx) = mpsc::channel();
        let plat = FakePlatform {
            distros: vec![distro("A", "a.vhdx"), distro("B", "b.vhdx")],
            fail_fstrim_on: None,
            fail_compact_on: Some("a.vhdx".into()),
            sizes: Mutex::new(HashMap::from([
                ("a.vhdx".to_string(), vec![10_000]), // before only; compact fails
                ("b.vhdx".to_string(), vec![10_000, 7_000]),
            ])),
        };
        run_pipeline(&plat, &["A".to_string(), "B".to_string()], &tx);
        let events: Vec<PipelineEvent> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::StepFailed { distro, step: PipelineStep::Compact, .. } if distro == "A"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            PipelineEvent::Done { distro, saved, .. } if distro == "B" && *saved == ByteSize(3_000)
        )));
    }
}
