# YAML Normalization — Hybrid Shape (Write-Time Rust + Nightly Python)

> **Status (2026-05-13)**: The corpus-as-ground-truth audit reframed what was
> originally planned as a Python-to-Rust *migration* into a **two-tier defense
> in depth**. Both normalizers run. They cover different surfaces.

## Why both

Originally the plan was to swap the Python `yaml-normalize.py` cron for a
native `floatctl normalize` subcommand. A real-corpus smoke test rejected
that plan: `serde_yml` (Rust) and `ruamel.yaml` (Python) disagree on
style-preserving emission in ways that are semantically equivalent but
would churn the corpus on every nightly run — defeating the idempotence
the Python script has been quietly providing for weeks.

Specifically:

- `serde_yml` emits block-style for any sequence not in its FLOW_KEYS list,
  while `ruamel.yaml` preserves the input style per-key.
- `serde_yml` force-quotes scalars containing `:` (e.g. timestamps like
  `created: 2025-11-10 @ 01:11 PM`).

Smoke-testing against `rangle-weekly/meetings/` showed 95/97 files would be
needlessly rewritten by the Rust normalizer despite being already-clean
under the Python definition.

Root-cause analysis lives at
`/opt/float/bbs/boards/sysops-log/2026-05-13-corpus-as-ground-truth.md`.
The corpus is the verifier; synthetic unit tests are necessary but never
sufficient.

## The hybrid shape

```
┌─────────────────────────────────────────────────────────────────┐
│  Write-time hook (Rust)                                         │
│  --------------------------------------------------------------│
│  Every floatctl BBS write passes through write_with_frontmatter│
│  → floatctl_core::yaml_normalize::normalize_str                │
│                                                                 │
│  Surface: floatctl-authored content only (BBS posts, bridges,  │
│  any caller of the write_with_frontmatter helper).             │
│                                                                 │
│  Why safe: only sees serde_yml's own output, which round-trips │
│  idempotently. No corpus churn — only normalizes what the      │
│  helper just serialized.                                        │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  Nightly sweep (Python)                                         │
│  --------------------------------------------------------------│
│  /opt/float/bbs/scripts/yaml-normalize.py                      │
│  Run via launchd at 03:17 AM                                   │
│  (tech.float.bbs.yaml-normalize.plist)                         │
│                                                                 │
│  Surface: arbitrary corpus content (manual edits via $EDITOR,  │
│  Write tool, dispatch-side writers, anything not routed        │
│  through floatctl).                                             │
│                                                                 │
│  Why kept: ruamel.yaml's style-preservation is the reason it   │
│  reaches steady state on arbitrary input. Corpus-verified for  │
│  weeks.                                                         │
└─────────────────────────────────────────────────────────────────┘
```

## What each catches

| Failure mode                     | Write-time (Rust) | Nightly (Python) |
|----------------------------------|:-----------------:|:----------------:|
| Doubled-frontmatter at write     |        Y          |        Y         |
| Block→flow on tags/relates       |        Y          |        Y         |
| Canonical-key rename             |        Y          |        Y         |
| Bare-wikilink quoting            |        Y          |        Y         |
| Colon-in-scalar quoting          |        Y          |        Y         |
| Manual-edit corruption           |        —          |        Y         |
| Write-tool / non-floatctl writer |        —          |        Y         |
| Drift in already-clean corpus    |        Y          |        Y (idem.) |

## What's NOT in this picture

There is no `floatctl normalize` CLI subcommand. It was implemented and then
deleted (see commit dropping `floatctl-cli/src/commands/normalize.rs`)
because the corpus-parity divergence made it unsafe as a wholesale
replacement, and the write-time hook covers floatctl-authored content
already.

The Rust core module `floatctl_core::yaml_normalize` stays — it powers the
write-time hook. The CLI surface is the only thing that went away.

## Operational notes

- **Do not** `launchctl unload` the Python plist. It is doing real work.
- **Do not** `rm /opt/float/bbs/scripts/yaml-normalize.py`. It is doing
  real work.
- The write-time hook is automatic for any caller of
  `write_with_frontmatter` — no flags, no opt-in. Tested for idempotence
  in `floatctl_core::yaml_normalize` unit tests and in
  `floatctl-server/tests/yaml_normalize_parity.rs` (corpus-parity
  integration test against the Python reference normalizer).

## Behaviour invariants covered by both

- Canonical key rename: `related | connectsTo | connectTo | connects | backlinks | references` → `relates`
- Flow-style emission for `tags` and `relates`
- Doubled-frontmatter merge with false-positive guard for mid-body
  horizontal rules
- Bare-wikilink pre-quoting for list values and block items
- Colon-in-scalar pre-quoting
- Duplicate top-level list-key merge (union, dedup, order-preserved)
