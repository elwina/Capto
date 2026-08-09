use std::path::PathBuf;

fn main() {
    let mut dirs = Vec::new();
    dirs.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/binaries"),
    );
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
            dirs.push(parent.join("binaries"));
        }
    }

    for dir in &dirs {
        if let Some(p) = capto_encode::FfmpegEncoder::resolve_binary(Some(dir)) {
            println!("FOUND {}", p.display());
            return;
        }
    }
    println!("NOT_FOUND — run scripts/download-ffmpeg.ps1");
}
