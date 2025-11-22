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
