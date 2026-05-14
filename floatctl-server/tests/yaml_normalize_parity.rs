//! Integration test: verify `write_with_frontmatter` output is idempotent
//! according to the Python `yaml-normalize.py` reference normalizer.
//!
//! This is the corpus-as-verifier doctrine applied to the write-time hook.
//! See `/opt/float/bbs/boards/sysops-log/2026-05-13-corpus-as-ground-truth.md`.
//!
//! The test:
//!  1. Builds a realistic frontmatter struct + body.
//!  2. Runs it through `write_with_frontmatter` (which internally calls
//!     `floatctl_core::yaml_normalize::normalize_str`).
//!  3. Writes the output to a tempfile.
//!  4. Shells out to `/opt/float/bbs/scripts/yaml-normalize.py --diff <tmp>`.
//!  5. Asserts the Python script reports "would change: 0 files".
//!
//! If the Python script isn't installed (uv missing, script path missing),
//! the test is skipped — we don't want a missing-script to break the
//! workspace build for downstream consumers.

use std::path::Path;
use std::process::Command;

use floatctl_server::bbs::write_with_frontmatter;
use serde::Serialize;

const PY_SCRIPT: &str = "/opt/float/bbs/scripts/yaml-normalize.py";

#[derive(Debug, Serialize)]
struct TestFrontmatter {
    title: String,
    author: String,
    created: String,
    imprint: String,
    tags: Vec<String>,
    relates: Vec<String>,
}

fn python_script_available() -> bool {
    if !Path::new(PY_SCRIPT).exists() {
        return false;
    }
    // The script's shebang is `#!/usr/bin/env -S uv run --script`. If uv
    // isn't on PATH, the script can't run.
    Command::new("uv")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[test]
fn write_with_frontmatter_output_is_corpus_clean() {
    if !python_script_available() {
        eprintln!(
            "SKIP: {} not available or `uv` missing on PATH — \
             cannot verify corpus parity in this environment.",
            PY_SCRIPT
        );
        return;
    }

    // Construct a realistic frontmatter — mirrors the shape of BBS posts
    // that hit the write_with_frontmatter helper in practice.
    let fm = TestFrontmatter {
        title: "Test: Corpus Parity Check".to_string(),
        author: "kitty".to_string(),
        created: "2026-05-13".to_string(),
        imprint: "sysops-log".to_string(),
        tags: vec![
            "yaml-normalize".to_string(),
            "corpus-as-verifier".to_string(),
            "hybrid-shape".to_string(),
        ],
        relates: vec![
            "[[2026-05-13-corpus-as-ground-truth]]".to_string(),
            "[[FLO-700]]".to_string(),
        ],
    };

    let body = "Body content with [[wikilinks]] and a `code reference`.\n\n\
                Multiple paragraphs.\n";

    let output =
        write_with_frontmatter(&fm, body).expect("write_with_frontmatter must succeed");

    // Write to tempfile with .md suffix so the Python script picks it up.
    let tmp = tempfile::Builder::new()
        .prefix("yaml-parity-")
        .suffix(".md")
        .tempfile()
        .expect("tempfile creation");

    std::fs::write(tmp.path(), &output).expect("tempfile write");

    let py_output = Command::new(PY_SCRIPT)
        .arg("--diff")
        .arg(tmp.path())
        .output()
        .expect("invoke python yaml-normalize.py");

    // Script writes "would change: N files" to stdout/stderr depending on
    // its mode. Check both.
    let stdout = String::from_utf8_lossy(&py_output.stdout);
    let stderr = String::from_utf8_lossy(&py_output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    // Successful no-op signal: "would change: 0 files".
    let is_clean = combined.contains("would change: 0 files");

    if !is_clean {
        eprintln!("--- write_with_frontmatter output ---");
        eprintln!("{output}");
        eprintln!("--- python normalizer stdout ---");
        eprintln!("{stdout}");
        eprintln!("--- python normalizer stderr ---");
        eprintln!("{stderr}");
        panic!(
            "write_with_frontmatter output is NOT corpus-clean per the \
             Python reference normalizer. See diff above. This is the \
             actual signal that the write-time hook needs more work — \
             do not paper over."
        );
    }
}
