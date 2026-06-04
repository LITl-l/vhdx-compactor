//! Compact a vhdx: virtdisk.dll (`CompactVirtualDisk`) primary, `diskpart`
//! fallback.
//!
//! After `fstrim` has discarded the guest's free blocks (zeroing them at the
//! virtual-disk level), a block-level `CompactVirtualDisk` reclaims them without
//! needing to understand the ext4 filesystem.

use std::process::Command;
use vhdx_core::model::EngineUsed;
use windows::Win32::Foundation::{CloseHandle, ERROR_IO_PENDING, ERROR_SUCCESS, HANDLE};
use windows::Win32::Storage::Vhd::{
    COMPACT_VIRTUAL_DISK_FLAG_NONE, COMPACT_VIRTUAL_DISK_PARAMETERS,
    COMPACT_VIRTUAL_DISK_VERSION_1, CompactVirtualDisk, GetVirtualDiskOperationProgress,
    OPEN_VIRTUAL_DISK_FLAG_NONE, OpenVirtualDisk, VIRTUAL_DISK_ACCESS_METAOPS,
    VIRTUAL_DISK_PROGRESS, VIRTUAL_STORAGE_TYPE, VIRTUAL_STORAGE_TYPE_DEVICE_VHDX,
    VIRTUAL_STORAGE_TYPE_VENDOR_MICROSOFT,
};
use windows::Win32::System::IO::OVERLAPPED;
use windows::core::PCWSTR;

/// Compact `path`, reporting 0..=100 progress. Falls back to diskpart on any
/// virtdisk failure.
pub fn compact(path: &str, progress: &mut dyn FnMut(u32)) -> anyhow::Result<EngineUsed> {
    match compact_virtdisk(path, progress) {
        Ok(()) => Ok(EngineUsed::Virtdisk),
        Err(e) => {
            log::warn!("virtdisk compact failed ({e}); falling back to diskpart");
            compact_diskpart(path)?;
            progress(100);
            Ok(EngineUsed::Diskpart)
        }
    }
}

/// RAII guard so the disk handle is always closed.
struct HandleGuard(HANDLE);
impl Drop for HandleGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn compact_virtdisk(path: &str, progress: &mut dyn FnMut(u32)) -> anyhow::Result<()> {
    // NUL-terminated wide string for the PCWSTR path argument.
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let storage_type = VIRTUAL_STORAGE_TYPE {
            DeviceId: VIRTUAL_STORAGE_TYPE_DEVICE_VHDX,
            VendorId: VIRTUAL_STORAGE_TYPE_VENDOR_MICROSOFT,
        };

        let mut handle = HANDLE::default();
        let rc = OpenVirtualDisk(
            &storage_type,
            PCWSTR(wide.as_ptr()),
            VIRTUAL_DISK_ACCESS_METAOPS,
            OPEN_VIRTUAL_DISK_FLAG_NONE,
            None,
            &mut handle,
        );
        if rc != ERROR_SUCCESS {
            anyhow::bail!("OpenVirtualDisk failed: {rc:?}");
        }
        let _guard = HandleGuard(handle);

        // Async compaction with an OVERLAPPED we poll for progress. hEvent is
        // left null; the VHD driver updates the progress fields regardless.
        let overlapped: OVERLAPPED = std::mem::zeroed();
        let params = COMPACT_VIRTUAL_DISK_PARAMETERS {
            Version: COMPACT_VIRTUAL_DISK_VERSION_1,
            ..Default::default()
        };

        let rc = CompactVirtualDisk(
            handle,
            COMPACT_VIRTUAL_DISK_FLAG_NONE,
            Some(&params),
            Some(&overlapped),
        );
        if rc != ERROR_SUCCESS && rc != ERROR_IO_PENDING {
            anyhow::bail!("CompactVirtualDisk failed: {rc:?}");
        }

        loop {
            let mut prog = VIRTUAL_DISK_PROGRESS::default();
            let grc = GetVirtualDiskOperationProgress(handle, &overlapped, &mut prog);
            if grc != ERROR_SUCCESS {
                anyhow::bail!("GetVirtualDiskOperationProgress failed: {grc:?}");
            }
            // OperationStatus stays ERROR_IO_PENDING while running, 0 when done.
            if prog.OperationStatus == ERROR_SUCCESS.0 {
                progress(100);
                break;
            }
            if prog.CompletionValue > 0 {
                let pct = (prog.CurrentValue.saturating_mul(100) / prog.CompletionValue) as u32;
                progress(pct.min(100));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        Ok(())
    }
}

/// diskpart fallback: attach read-only, compact, detach via a script file.
fn compact_diskpart(path: &str) -> anyhow::Result<()> {
    let script = format!(
        "select vdisk file=\"{path}\"\n\
         attach vdisk readonly\n\
         compact vdisk\n\
         detach vdisk\n\
         exit\n"
    );
    let tmp = std::env::temp_dir().join("vhdx_compactor_diskpart.txt");
    std::fs::write(&tmp, script)?;
    let status = Command::new("diskpart").arg("/s").arg(&tmp).status();
    let _ = std::fs::remove_file(&tmp);
    let status = status?;
    if !status.success() {
        anyhow::bail!("diskpart exited with {status}");
    }
    Ok(())
}
