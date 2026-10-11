# Architecture — implemented behavior versus proposed planes

Conduit currently is a local CLI and independent Rust library crates. The following is the **implemented pipeline** for `run --compact`:

1. Accept caller-provided executable path and arguments (no policy enforcement; process inherits caller privileges).
2. Run the direct child with a duration limit; drain stdout and stderr with independent capture limits.
3. If capture overflows, report incomplete output and exit with an error (no lossless expansion available).
4. If the child exits successfully with valid text stdout, attempt a diagnostic-preserving reduction.
5. Estimate hint-inclusive reduction, then persist original stdout to an integrity-checked BLAKE3 store before displaying a shorter body.
6. Emit the expansion hint on stderr. If persisting fails, pass stdout through unchanged.
7. If `CONDUIT_SESSION` is set, record a JSONL word-estimate event (not actual model tokens).

`find` now scans bounded files without retaining a repository-wide snapshot; it follows gitignore rules plus `.conduitignore`, skips hidden files by default, and can fail closed with `--require-complete` when its bounded scan cannot support a complete-result claim. The library still exposes a snapshot API. `peek` is a bounded hash-anchored read with clipped display lines. Neither implements syntax trees or a persistent index. `edit` and `batch-edit` share preflight validation and perform per-file staged renames with cooperative locking, but not crash-atomic transactions across files. `doctor` verifies storage integrity on demand; `repair-session` preserves valid unterminated events and quarantines malformed tails before truncation.

The following crates exist but are **not integrated with CLI dispatch**: `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, `fanout`. The protobuf schema is not generated or consumed by the CLI. No OS sandbox, PTY, provider gateway, daemon or distributed state plane is available.

## Target architecture (proposal only)

```text
agent adapters / typed tool requests
    -> authorization and capability reservation [not implemented]
    -> supervised execution with bounded raw spool [partially implemented]
    -> verified content store and output adapters [partially implemented]
    -> correlated accounting and deterministic retrieval [partially implemented]
    -> version-checked edit and recovery journal [not implemented]
    -> evaluation/promotion gates and optional memory [not implemented]
```

Key requirements before any production trust claim are independent operating-system containment for hostile commands, typed non-bypassable authorization, explicit resource exhaustion behavior, transactional recovery, privacy/retention controls, and live-agent non-inferiority experiments.
