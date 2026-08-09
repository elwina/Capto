use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const LOCK_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerLock {
    pub pid: u32,
    pub port: u16,
    pub token: String,
    pub version: u32,
}

pub fn lock_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Capto")
        .join("cli-server.json")
}

pub fn write_server_lock(lock: &ServerLock) -> std::io::Result<PathBuf> {
    let path = lock_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(lock)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)?;
    Ok(path)
}

pub fn read_server_lock() -> std::io::Result<ServerLock> {
    read_server_lock_at(&lock_path())
}

pub fn read_server_lock_at(path: &Path) -> std::io::Result<ServerLock> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn clear_server_lock() {
    let path = lock_path();
    let _ = fs::remove_file(path);
}

/// Best-effort check whether `pid` still refers to a live process.
pub fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `tasklist` is slow; OpenProcess is better but needs winapi.
        // /FI and findstr: exit 0 if listed.
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                text.contains(&pid.to_string())
            }
            Err(_) => true, // assume alive if we can't check
        }
    }
    #[cfg(unix)]
    {
        Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn lock_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cli-server.json");
        let lock = ServerLock {
            pid: 42,
            port: 12345,
            token: "tok".into(),
            version: LOCK_VERSION,
        };
        fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
        let loaded = read_server_lock_at(&path).unwrap();
        assert_eq!(loaded.port, 12345);
        assert_eq!(loaded.token, "tok");
    }
}
