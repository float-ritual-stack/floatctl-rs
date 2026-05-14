# Migrating from `yaml-normalize.py` to `floatctl normalize`

The Python normalizer at `/opt/float/bbs/scripts/yaml-normalize.py` (run daily
via launchd at 03:17 via `tech.float.bbs.yaml-normalize.plist`) has been ported
to a native Rust subcommand: `floatctl normalize`.

The Python script and its launchd plist remain in place — this migration is
**opt-in** and **plist-only**. The Rust port is byte-compatible for the
documented normalization invariants.

## Verify parity before flipping the cron

```bash
# 1. Dry-run a few diffs on a sample board to eyeball the changes
floatctl normalize --diff --limit 5 /opt/float/bbs/boards/sysops-log

# 2. Compare against the Python script on a larger sample
diff \
  <(/opt/float/bbs/scripts/yaml-normalize.py /opt/float/bbs/boards/sysops-log 2>&1) \
  <(floatctl normalize /opt/float/bbs/boards/sysops-log 2>&1)
```

Expected differences (acceptable):

- Single vs double-quote style on emitted scalars (`serde_yml` defaults to
  single quotes; `ruamel` to double). Semantically equivalent.
- Trailing-newline handling at end-of-file — both emit a final newline.

Material differences worth investigating:

- Key drift not renamed → bug in port (check `KEY_CANONICAL` set).
- `tags:` / `relates:` left in block form → bug in `block_to_flow_for_flow_keys`.
- Doubled-frontmatter merged when it shouldn't be → false-positive guard regression.

## Swap the launchd plist

Edit `~/Library/LaunchAgents/tech.float.bbs.yaml-normalize.plist`. Replace the
`ProgramArguments` array that points to `yaml-normalize-run.sh` with:

```xml
<key>ProgramArguments</key>
<array>
    <string>/Users/evan/.cargo/bin/floatctl</string>
    <string>normalize</string>
    <string>--apply</string>
    <string>/opt/float/bbs/boards</string>
</array>
```

Reload:

```bash
launchctl unload ~/Library/LaunchAgents/tech.float.bbs.yaml-normalize.plist
launchctl load   ~/Library/LaunchAgents/tech.float.bbs.yaml-normalize.plist
```

## Rollback

Revert the plist change and reload. The Python script is unchanged on disk;
the swap is plist-only and reversible.

## Behaviour notes

`floatctl normalize` ports the following invariants from `yaml-normalize.py`:

- Canonical key rename: `related | connectsTo | connectTo | connects | backlinks | references` → `relates`
- Flow-style emission for `tags` and `relates`
- Doubled-frontmatter merge (up to 10 marker-line gap window) with
  false-positive guard for mid-body horizontal rules
- Bare-wikilink pre-quoting for list values and block items
- Colon-in-scalar pre-quoting (`title: Dispatch: BBS Sweep` → quoted)
- Duplicate top-level list-key merge (union, dedup, order-preserved)

The CLI flag surface is the same: `--apply`, `--diff`, `--limit`, plus a new
`--mtime DAYS` filter for restricting to recently-modified files.

The same module (`floatctl_core::yaml_normalize`) is also wired into the
BBS server's `write_with_frontmatter` helper so files are born clean —
no separate cleanup pass needed for newly-authored content. The launchd job
remains useful for back-end cleanup of legacy hand-edited files.
