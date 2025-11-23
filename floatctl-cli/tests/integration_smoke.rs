//! Smoke tests to verify command module wiring

use assert_cmd::Command;
use predicates::prelude::*;

// === Script Command Tests ===

#[test]
fn test_script_list_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("script").arg("list").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Output format"));
}

#[test]
fn test_script_register_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("script").arg("register").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Path to the script"));
}

// === Claude Command Tests ===

#[test]
fn test_claude_list_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("claude").arg("list").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Number of sessions"));
}

#[test]
fn test_claude_show_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("claude").arg("show").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Session ID"));
}

// === Bridge Command Tests ===

#[test]
fn test_bridge_index_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("bridge").arg("index").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Input file or directory"));
}

#[test]
fn test_bridge_append_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("bridge").arg("append").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Read content from stdin"));
}

// === Evna Command Tests ===

#[test]
fn test_evna_status_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("evna").arg("status").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Show evna MCP server status"));
}

#[test]
fn test_evna_boot_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("evna").arg("boot").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Brain boot"));
}

// === System Command Tests ===

#[test]
fn test_system_health_check_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("system").arg("health-check").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Run system health diagnostics"));
}

#[test]
fn test_system_cleanup_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("system").arg("cleanup").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Preview cleanup actions"));
}

// === Ctx Command Test ===

#[test]
fn test_ctx_help() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("ctx").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Capture context markers"));
}
