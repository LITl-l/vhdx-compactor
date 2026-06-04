//! Embed a Windows application manifest requesting administrator privileges.
//!
//! Compaction (virtdisk + diskpart) requires elevation, so the binary requests
//! it up front via the manifest (one UAC prompt at launch). This runs on the
//! host but keys off `CARGO_CFG_WINDOWS` — the *target* cfg — so it embeds only
//! when building a Windows binary and is a no-op for host builds.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Embed the admin manifest only in release Windows builds. Debug builds skip
    // it so the binary can be run non-elevated during development/testing (no UAC
    // prompt), and so the headless CLI mode is scriptable.
    let is_release = std::env::var("PROFILE").as_deref() == Ok("release");
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() && is_release {
        use embed_manifest::manifest::ExecutionLevel;
        use embed_manifest::{embed_manifest, new_manifest};
        embed_manifest(
            new_manifest("VhdxCompactor")
                .requested_execution_level(ExecutionLevel::RequireAdministrator),
        )
        .expect("failed to embed application manifest");
    }
}
