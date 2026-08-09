use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locate and spawn Capto desktop. Single-instance plugin ensures at most one process.
pub fn spawn_capto() -> Result<()> {
    let exe = find_capto_exe().context("Capto desktop executable not found")?;
    tracing::info!(path = %exe.display(), "launching Capto desktop");
    let mut cmd = Command::new(&exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }
    cmd.spawn()
        .with_context(|| format!("failed to spawn {}", exe.display()))?;
    Ok(())
}

fn find_capto_exe() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("CAPTO_APP_PATH") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Ok(path);
        }
        bail!("CAPTO_APP_PATH does not exist: {}", path.display());
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Installed / side-by-side names (desktop is NOT the CLI binary).
            candidates.push(dir.join("Capto.exe"));
            candidates.push(dir.join("capto-app.exe"));
            candidates.push(dir.join("../debug/capto-app.exe"));
            candidates.push(dir.join("../release/capto-app.exe"));
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("../../target/debug/capto-app.exe"));
    candidates.push(manifest.join("../../target/release/capto-app.exe"));
    candidates.push(
        manifest.join("../../apps/desktop/src-tauri/target/debug/capto-app.exe"),
    );
    candidates.push(
        manifest.join("../../apps/desktop/src-tauri/target/release/capto-app.exe"),
    );

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(&local).join("Capto/Capto.exe"));
        candidates.push(PathBuf::from(local).join("Capto/capto-app.exe"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        candidates.push(PathBuf::from(pf).join("Capto/Capto.exe"));
    }

    for c in &candidates {
        if let Ok(canon) = c.canonicalize() {
            if canon.is_file() {
                return Ok(canon);
            }
        } else if c.is_file() {
            return Ok(c.clone());
        }
    }

    bail!(
        "could not find Capto desktop — set CAPTO_APP_PATH or build with `cargo build -p capto-app`"
    );
}

#[allow(dead_code)]
fn _exists(p: &Path) -> bool {
    p.is_file()
}
