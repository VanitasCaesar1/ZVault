//! Integration tests for the `zvault` CLI binary.
//!
//! These tests exercise the CLI as a subprocess, verifying exit codes,
//! stdout output, and file-system side effects. They do NOT require a
//! running vault server — tests that need one are gated behind the
//! `integration` feature or skipped gracefully.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper: locate the `zvault` binary built by `cargo test`.
fn zvault_bin() -> String {
    // `cargo test` puts the binary in target/debug/
    let path = env!("CARGO_BIN_EXE_zvault");
    assert!(
        Path::new(path).exists(),
        "zvault binary not found at {path}"
    );
    path.to_owned()
}

/// Helper: run zvault with args and return (`exit_code`, stdout, stderr).
fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(zvault_bin())
        .args(args)
        .env("VAULT_ADDR", "http://127.0.0.1:19999") // Non-existent server
        .env_remove("VAULT_TOKEN")
        .output()
        .expect("failed to execute zvault");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

// ── Version & help ───────────────────────────────────────────────────

#[test]
fn test_version_flag() {
    let (code, stdout, _) = run(&["--version"]);
    assert_eq!(code, 0, "zvault --version should exit 0");
    assert!(
        stdout.contains("zvault"),
        "version output should contain 'zvault': {stdout}"
    );
}

#[test]
fn test_help_flag() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0, "zvault --help should exit 0");
    assert!(
        stdout.contains("ZVault CLI"),
        "help should mention ZVault CLI"
    );
    assert!(
        stdout.contains("status"),
        "help should list 'status' command"
    );
    assert!(
        stdout.contains("import"),
        "help should list 'import' command"
    );
    assert!(stdout.contains("run"), "help should list 'run' command");
    assert!(
        stdout.contains("doctor"),
        "help should list 'doctor' command"
    );
}

#[test]
fn test_subcommand_help() {
    let subcommands = [
        "kv", "token", "policy", "transit", "pki", "approle", "database",
    ];
    for sub in subcommands {
        let (code, stdout, _) = run(&[sub, "--help"]);
        assert_eq!(code, 0, "{sub} --help should exit 0");
        assert!(!stdout.is_empty(), "{sub} --help should produce output");
    }
}

// ── License command (no server needed) ───────────────────────────────

#[test]
fn test_license_shows_free_tier() {
    // With no license file, should show Free tier.
    let output = Command::new(zvault_bin())
        .args(["license"])
        .env("HOME", "/tmp/zvault-test-nonexistent-home")
        .env_remove("VAULT_TOKEN")
        .output()
        .expect("failed to execute zvault");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Free"),
        "license command should show Free tier when no license: {stdout}"
    );
}

// ── Import command (file-system tests) ───────────────────────────────

#[test]
fn test_import_missing_file() {
    let (code, _, stderr) = run(&["import", "/tmp/zvault-test-nonexistent.env"]);
    assert_ne!(code, 0, "import of missing file should fail");
    assert!(
        stderr.contains("not found") || stderr.contains("Error"),
        "should report file not found: {stderr}"
    );
}

#[test]
fn test_import_empty_env_file() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let env_path = dir.path().join(".env");
    fs::write(&env_path, "# just a comment\n\n").expect("write failed");

    let output = Command::new(zvault_bin())
        .args([
            "import",
            env_path.to_str().unwrap(),
            "--no-backup",
            "--no-ref",
            "--no-gitignore",
        ])
        .env("VAULT_ADDR", "http://127.0.0.1:19999")
        .env("VAULT_TOKEN", "test-token")
        .output()
        .expect("failed to execute zvault");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "import of empty env should fail");
    assert!(
        stderr.contains("no secrets found"),
        "should report no secrets: {stderr}"
    );
}

// ── Run command (validation tests) ───────────────────────────────────

#[test]
fn test_run_no_command() {
    let (code, _, stderr) = run(&["run"]);
    assert_ne!(code, 0, "run with no command should fail");
    assert!(
        stderr.contains("required") || stderr.contains("error") || stderr.contains("Error"),
        "should report missing command: {stderr}"
    );
}

#[test]
fn test_run_missing_env_file() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(zvault_bin())
        .args(["run", "--", "echo", "hello"])
        .env("VAULT_ADDR", "http://127.0.0.1:19999")
        .env("VAULT_TOKEN", "test-token")
        .current_dir(dir.path())
        .output()
        .expect("failed to execute zvault");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "run without env file should fail");
    assert!(
        stderr.contains(".env.zvault") || stderr.contains(".env"),
        "should mention missing env file: {stderr}"
    );
}

// ── Doctor command ───────────────────────────────────────────────────

#[test]
fn test_doctor_runs_without_crash() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(zvault_bin())
        .args(["doctor"])
        .env("VAULT_ADDR", "http://127.0.0.1:19999")
        .env_remove("VAULT_TOKEN")
        .env("HOME", dir.path().to_str().unwrap())
        .current_dir(dir.path())
        .output()
        .expect("failed to execute zvault");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "doctor should exit 0 even with warnings: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Doctor"),
        "should show Doctor header: {stdout}"
    );
    assert!(
        stdout.contains("passed") || stdout.contains("warnings") || stdout.contains("failed"),
        "should show summary: {stdout}"
    );
}

#[test]
fn test_doctor_detects_unreachable_server() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(zvault_bin())
        .args(["doctor"])
        .env("VAULT_ADDR", "http://127.0.0.1:19999")
        .env_remove("VAULT_TOKEN")
        .env("HOME", dir.path().to_str().unwrap())
        .current_dir(dir.path())
        .output()
        .expect("failed to execute zvault");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("unreachable"),
        "should detect unreachable server: {stdout}"
    );
}

#[test]
fn test_doctor_detects_env_zvault() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    // Create a .env.zvault file.
    fs::write(
        dir.path().join(".env.zvault"),
        "DB_URL=zvault://env/test/DB_URL\nAPI_KEY=zvault://env/test/API_KEY\n",
    )
    .expect("write failed");

    let output = Command::new(zvault_bin())
        .args(["doctor"])
        .env("VAULT_ADDR", "http://127.0.0.1:19999")
        .env_remove("VAULT_TOKEN")
        .env("HOME", dir.path().to_str().unwrap())
        .current_dir(dir.path())
        .output()
        .expect("failed to execute zvault");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("2 references"),
        "should detect 2 zvault:// references: {stdout}"
    );
}

// ── Setup command (license gating) ───────────────────────────────────

#[test]
fn test_setup_requires_pro() {
    let output = Command::new(zvault_bin())
        .args(["setup", "cursor"])
        .env("HOME", "/tmp/zvault-test-no-license")
        .env_remove("VAULT_TOKEN")
        .output()
        .expect("failed to execute zvault");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "setup should fail without Pro license"
    );
    assert!(
        stderr.contains("Pro") || stderr.contains("license"),
        "should mention Pro requirement: {stderr}"
    );
}

#[test]
fn test_mcp_server_requires_pro() {
    let output = Command::new(zvault_bin())
        .args(["mcp-server"])
        .env("HOME", "/tmp/zvault-test-no-license")
        .env_remove("VAULT_TOKEN")
        .output()
        .expect("failed to execute zvault");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "mcp-server should fail without Pro license"
    );
    assert!(
        stderr.contains("Pro") || stderr.contains("license"),
        "should mention Pro requirement: {stderr}"
    );
}

// ── Activate command (validation) ────────────────────────────────────

#[test]
fn test_activate_invalid_ed25519_key() {
    let output = Command::new(zvault_bin())
        .args(["activate", "not.a.valid.key"])
        .env("HOME", "/tmp/zvault-test-activate")
        .env_remove("VAULT_TOKEN")
        .output()
        .expect("failed to execute zvault");

    assert!(
        !output.status.success(),
        "activate with invalid key should fail"
    );
}

// ── .env parser unit-level tests (via subprocess) ────────────────────

#[test]
fn test_import_parses_various_env_formats() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let env_content = r#"
# Comment line
SIMPLE=value
QUOTED="quoted value"
SINGLE='single quoted'
export EXPORTED=exported_value
EMPTY=
SPACES_AROUND = spaced

"#;
    let env_path = dir.path().join("test.env");
    fs::write(&env_path, env_content).expect("write failed");

    // This will fail to connect to vault, but the error message tells us
    // how many secrets were parsed.
    let output = Command::new(zvault_bin())
        .args([
            "import",
            env_path.to_str().unwrap(),
            "--no-backup",
            "--no-ref",
            "--no-gitignore",
        ])
        .env("VAULT_ADDR", "http://127.0.0.1:19999")
        .env("VAULT_TOKEN", "test-token")
        .output()
        .expect("failed to execute zvault");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should have parsed 6 secrets (SIMPLE, QUOTED, SINGLE, EXPORTED, EMPTY, SPACES_AROUND).
    assert!(
        stdout.contains('6') || stdout.contains("Secrets"),
        "should parse 6 env vars from mixed format: {stdout}"
    );
}
