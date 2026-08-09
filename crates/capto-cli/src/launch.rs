use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Locate Capto desktop and open it via the OS shell (fire-and-forget).
///
/// On Windows this uses `cmd /C start "" <exe>` so Capto is not a child of the CLI
/// (avoids handle-inheritance hangs when agents redirect stdout/stderr).
pub fn open_desktop() -> Result<PathBuf> {
    let exe = normalize_spawn_path(find_capto_exe().context("Capto desktop executable not found")?);
    tracing::info!(path = %exe.display(), "opening Capto desktop");
    shell_open(&exe).with_context(|| format!("failed to open {}", exe.display()))?;
    Ok(exe)
}

/// Same as [`open_desktop`] — used by auto-launch before waiting for the control plane.
pub fn spawn_capto() -> Result<PathBuf> {
    open_desktop()
}

fn shell_open(exe: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        // `start` requires an (possibly empty) window title when the target is quoted.
        let status = Command::new("cmd.exe")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg("/D")
            .arg(exe.parent().unwrap_or_else(|| Path::new(".")))
            .arg(exe.as_os_str())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("cmd start")?;
        if !status.success() {
            bail!("cmd start exited with {status}");
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn")?;
        Ok(())
    }
}

fn find_capto_exe() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("CAPTO_APP_PATH") {
        let path = normalize_spawn_path(PathBuf::from(p));
        if !path.is_file() {
            bail!("CAPTO_APP_PATH does not exist: {}", path.display());
        }
        if is_cli_binary_name(&path) {
            bail!(
                "CAPTO_APP_PATH points at the CLI ({}); set it to capto-app.exe",
                path.display()
            );
        }
        return Ok(path);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Dev layout: target/debug/capto.exe → sibling capto-app.exe.
            // Do NOT look for Capto.exe next to this CLI — on case-insensitive Windows
            // Capto.exe and capto.exe are the same file, so we would re-open the CLI.
            candidates.push(dir.join("capto-app.exe"));
            // Installed: <install>/cli/capto.exe → <install>/Capto.exe | capto-app.exe
            candidates.push(dir.join("../Capto.exe"));
            candidates.push(dir.join("../capto-app.exe"));
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
        candidates.push(PathBuf::from(&pf).join("Capto/Capto.exe"));
        candidates.push(PathBuf::from(pf).join("Capto/capto-app.exe"));
    }

    let self_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map(normalize_spawn_path);

    for c in &candidates {
        let resolved = if let Ok(canon) = c.canonicalize() {
            if !canon.is_file() {
                continue;
            }
            canon
        } else if c.is_file() {
            c.clone()
        } else {
            continue;
        };
        let resolved = normalize_spawn_path(resolved);
        if let Some(self_path) = &self_exe {
            if paths_equal_ci(&resolved, self_path) {
                continue;
            }
        }
        if is_cli_binary_name(&resolved) {
            continue;
        }
        return Ok(resolved);
    }

    bail!(
        "could not find Capto desktop — install Capto, or set CAPTO_APP_PATH, or ask the user to open Capto"
    );
}

fn normalize_spawn_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

fn is_cli_binary_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("capto.exe") || n.eq_ignore_ascii_case("capto"))
}

fn paths_equal_ci(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn normalize_spawn_path_strips_verbatim_prefix() {
        let p = normalize_spawn_path(PathBuf::from(
            r"\\?\C:\Users\elwin\AppData\Local\Capto\capto-app.exe",
        ));
        assert_eq!(
            p,
            PathBuf::from(r"C:\Users\elwin\AppData\Local\Capto\capto-app.exe")
        );
    }

    #[test]
    fn cli_binary_name_detected() {
        assert!(is_cli_binary_name(Path::new(r"C:\Capto\cli\capto.exe")));
        assert!(!is_cli_binary_name(Path::new(
            r"C:\Capto\capto-app.exe"
        )));
    }
}
