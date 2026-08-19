//! Integration smoke tests for the `capto` CLI binary.
//!
//! These exercise the real compiled binary's command-line contract. They only
//! probe clap-level behavior (help/version/usage errors) so they are
//! deterministic in CI: no desktop control plane, no FFmpeg sidecar needed.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_capto"))
}

#[test]
fn version_flag_exits_zero_and_prints_version() {
    let out = bin()
        .arg("--version")
        .output()
        .expect("failed to run capto --version");
    assert!(
        out.status.success(),
        "capto --version failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let expected = concat!("capto ", env!("CARGO_PKG_VERSION"));
    assert!(
        stdout.contains(expected),
        "unexpected version output: {stdout}"
    );
}

#[test]
fn help_lists_all_top_level_commands() {
    let out = bin()
        .arg("--help")
        .output()
        .expect("failed to run capto --help");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for command in [
        "status", "open", "doctor", "record", "shot", "config", "list", "outputs",
    ] {
        assert!(
            stdout.contains(command),
            "--help output missing '{command}':\n{stdout}"
        );
    }
}

#[test]
fn missing_subcommand_is_a_usage_error() {
    let out = bin().output().expect("failed to run capto with no args");
    assert_eq!(
        out.status.code(),
        Some(2),
        "no-args should be a usage error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Usage"), "no-args usage output:\n{stderr}");
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    let out = bin()
        .arg("frobnicate")
        .output()
        .expect("failed to run capto frobnicate");
    assert_eq!(
        out.status.code(),
        Some(2),
        "unknown command should be a usage error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("unrecognized"),
        "unknown-command output:\n{stderr}"
    );
}
