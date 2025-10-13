# Repository Guidelines

## Project Structure & Module Organization
The workspace centers on `src/`, where each module owns a pipeline stage: `input.rs` handles export detection, `model.rs` normalizes conversations, `render_md.rs` and `render_json.rs` write outputs, and `util.rs` orchestrates execution. Configuration defaults live beside the binary in `local_config.toml`; user-level state is written to `.floatctl/state/`. Build artifacts land in `target/`. Sample exports, scratch output, and other large fixtures should stay outside the repo or inside a dedicated `fixtures/` directory when needed for tests.

## Build, Test, and Development Commands
Use `cargo build` for a debug build and `cargo build --release` before distributing binaries. Run the full pipeline locally with `cargo run -- --in conversations.json --out conv_out --format md,json`. Lint continuously with `cargo clippy -- -D warnings`, and format changes via `cargo fmt`. `cargo doc --no-deps --open` is helpful when exploring module internals.

## Coding Style & Naming Conventions
Follow standard Rust style: four-space indentation, 100-character soft wrap, and snake_case for modules, functions, and variables. Structs and enums use UpperCamelCase; constants use SCREAMING_SNAKE. Run `cargo fmt` before submitting and keep `clippy` warnings clean. When adding new CLI flags or config keys, mirror existing naming patterns (`--date-from`, `FilenameStrategy`) and document them in code comments or `CLAUDE.md` when behaviour changes.

## Testing Guidelines
Unit and integration tests belong under `src/` (module-level `#[cfg(test)]`) or `tests/` when cross-module scenarios are required. Prefer deterministic fixtures stored under `tests/data/`, avoiding user exports. Run `cargo test` locally before opening a PR; target complete coverage of new logic and exercise both Anthropic and ChatGPT branches when relevant. Use descriptive `test_can_parse_anthropic_zip()`-style names so failures identify the scenario.

## Commit & Pull Request Guidelines
The public history is thin, so adopt Conventional Commits (`feat: add slug dedupe guard`) to keep logs searchable. Commits should stay focused on one behavioural change plus necessary refactors. Pull requests must describe the motivation, outline validation (commands run, sample inputs), and link any tracking issues. Include screenshots or snippet diffs when UI-facing output such as Markdown layout changes. Tag reviewers on modules they own and wait for green CI before requesting merge.

## Security & Configuration Tips
Avoid checking in personal exports or state; `.floatctl/` should remain in `.gitignore`. When running on shared systems, set `FLOATCTL_TMP_DIR` to a writable scratch path to bypass restrictive home directories. Review third-party ZIPs before ingesting, and prefer local copies over network fetches since builds run in restricted environments.
