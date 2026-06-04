//! Resolve WSL distro → `ext4.vhdx` path from the Lxss registry hive.
//!
//! `wsl -l -v` gives names and state but not the on-disk path. The canonical
//! source is `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss\{guid}`, where
//! each distro key carries `DistributionName` and `BasePath`; the vhdx lives at
//! `BasePath\ext4.vhdx`.

use std::collections::HashMap;

const LXSS: &str = r"Software\Microsoft\Windows\CurrentVersion\Lxss";

/// Map of `DistributionName` → absolute `ext4.vhdx` path.
pub fn distro_vhdx_paths() -> anyhow::Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    let lxss = windows_registry::CURRENT_USER.open(LXSS)?;
    for guid in lxss.keys()? {
        let Ok(key) = lxss.open(&guid) else { continue };
        let Ok(name) = key.get_string("DistributionName") else {
            continue;
        };
        let Ok(base) = key.get_string("BasePath") else {
            continue;
        };
        // BasePath often carries a `\\?\` extended-length prefix; strip it.
        let base = base.strip_prefix(r"\\?\").unwrap_or(&base);
        out.insert(name, format!(r"{base}\ext4.vhdx"));
    }
    Ok(out)
}
