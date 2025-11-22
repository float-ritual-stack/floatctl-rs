//! Smoke tests to verify command module wiring

use assert_cmd::Command;
use predicates::prelude::*;

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
