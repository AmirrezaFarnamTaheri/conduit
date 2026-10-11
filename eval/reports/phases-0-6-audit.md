# Phases 0–6 completion and audit report

Implemented runtime surfaces:

- 0–2: content-addressed reversible storage, event accounting, deterministic lens, atlas retrieval, and hash-anchored atomic edits.
- 3–4: synchronized context ledger, hard multi-dimensional budgets, loop guard, impact-aware verification planning, and workspace command policy.
- 5: provenance-bound memory that refuses stale facts and deterministic bounded fanout that preserves input order and isolates worker panics.
- 6: threat model, dependency/license policy, CI formatting/test/lint gates, and a release metadata gate.

Hardening pass:

- Shared state uses synchronization and poison recovery; snapshots are cloned before exposure.
- Hashes are checked before staging and immediately before commit.
- Edit paths reject traversal and symlink escapes; duplicate paths and ambiguous replacements are rejected.
- Temporary files are uniquely named and replaced by rename; rollback uses temporary replacement rather than direct truncation.
- Budget arithmetic saturates before comparison; fanout worker count is bounded and zero workers are safe.
- No provider, network, mock, placeholder, or fabricated benchmark result was introduced.

Verification completed here: Python unit tests, static release-input checks, Markdown links, and diff checks. Rust compilation, formatting, CLI-path benchmark, unit/stress/fuzz tests, cross-platform sandbox tests, live-agent non-inferiority, PTY ordering, and external security audit remain unverified because Cargo and a registered SSH build server are unavailable.

## Supplied V3 patch evaluation (2026-10-11)

The patch applies cleanly to this repository and improves bounded command capture, integrity-checked expansion, typed/locked session records, bounded source search, edit preflight, integration tests, and an executable-path benchmark. The benchmark requires a compiled Rust executable and was not run here.

The companion report describes a candidate, not a certified release. Its claims about process-group cleanup, structured JSON CLI outputs, and `docs/wiki/structured-output.md` are not present in the supplied patch. Blob/session opens initially lacked the final-component no-follow guarantee attributed to them; Unix no-follow opens, private staging creation, and regression tests were added during integration. Parent-directory symlink races, external edit writers, cross-file crash atomicity, descendant containment, retention, and Windows ACL validation remain unresolved. `vault` is not connected to `run`, which executes with the caller's privileges. Benchmark units are whitespace-delimited words, not provider tokens.

Do not promote this candidate to production or represent Phases 0–6 as accepted until native gates run and their results are reviewed.
