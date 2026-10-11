# Threat model and release gates

## Assets

Raw command output, source files, session state, credentials in process environments, and integrity of edits are protected assets.

## Trust boundaries

Agent input and child-process output are untrusted. The workspace is untrusted. The local runtime store is trusted only after hash verification. Provider responses are external input and must not authorize local actions.

## Required controls

- Content-addressed storage and hash-anchored edits prevent stale reads from becoming writes.
- Intended control: policy checks should precede command execution and path checks should precede file access. **Present reality:** `run` is not policy-gated and inherits caller privileges; it is not an isolation boundary.
- Compact stdout reduction is reversible only when a blob was successfully persisted and verified. Output beyond capture limits is explicitly incomplete and cannot be expanded.
- Shared mutable state has one synchronization boundary and deterministic snapshots.
- Network access is absent unless explicitly configured by a later provider adapter.
- Release artifacts require dependency audit, formatting, lint, tests, benchmark execution, and reproducible build metadata.

## Known environment gates

Cross-platform sandbox behavior, PTY stream ordering, fuzzing, live-provider contracts, external audit, and Rust test execution require a Rust toolchain and the appropriate target environment. They are gates, not simulated successes.
