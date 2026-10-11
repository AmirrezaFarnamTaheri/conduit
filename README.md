# Conduit

Conduit is a local, agent-agnostic runtime for reducing coding-agent context and tool overhead without sacrificing recoverability. It keeps raw observations addressable, applies deterministic transformations first, and makes savings claims measurable rather than aspirational.

## Status

The repository contains executable foundations for Phases 0–6:

- reversible BLAKE3 blob storage and append-only session accounting
- fail-open command output lens with expansion handles
- deterministic repository search with content hashes
- hash-anchored, transactional edits with rollback and path protections
- synchronized context ledger, hard budgets, loop guard, impact-aware verification, and policy profiles
- provenance-bound memory and deterministic parallel fanout
- cross-platform CI, dependency policy, threat model, and release gates

The local container does not include Cargo. Rust compilation, stress/fuzz tests, and cross-platform verification run in GitHub Actions.

## Quick start

```sh
# Run the deterministic corpus benchmark
python3 eval/benchmark.py

# Run the release-input check
python3 eval/release_gate.py

# Build and test when Rust is installed
cargo test --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## CLI

Build the binary with Cargo, then use:

```sh
conduit run -- cargo test
conduit find "target symbol" 20
conduit edit path/to/file.rs EXPECTED_HASH OLD NEW
conduit store "raw text"
conduit expand cx:o:HANDLE
conduit record SESSION MODULE RAW_TOKENS PRESENTED_TOKENS
conduit replay SESSION
conduit stats SESSION
conduit bench
```

`run` preserves the child exit code. Raw output is stored before reduction when possible; reduced output includes an expansion handle whenever content was omitted. If storage or reduction fails, raw output is returned. `CONDUIT_HOME` controls session/blob state, and `CONDUIT_WORKSPACE` controls atlas and edit operations.

## Design principles

1. **Fail open:** Conduit must not become the reason an agent task fails.
2. **Reversible by default:** omitted content remains retrievable until retention removes it.
3. **Measured, not claimed:** reductions are reported with raw and presented counts.
4. **Deterministic first:** parsing, hashing, policy, and selection do not require a model.
5. **Stale data is unsafe:** file facts, search results, and edits carry hashes.
6. **Least privilege:** paths, commands, and budgets are checked at boundaries.
7. **Local first:** no provider or telemetry integration is enabled by default.

## Repository map

| Path | Purpose |
| --- | --- |
| `crates/store` | Content-addressed raw blobs and expansion handles |
| `crates/scope` | Append-only session events, replay, and accounting |
| `crates/lens` | Deterministic output reduction |
| `crates/atlas` | Snapshot search and hash-bearing hits |
| `crates/edit` | Hash-anchored transaction edits |
| `crates/ledger` | Context lifecycle and masking |
| `crates/relay` | Hard token, cost, and wall-time budgets |
| `crates/guard` | Repetition and failure-loop detection |
| `crates/verify` | Impacted-test planning |
| `crates/vault` | Command and workspace policy |
| `crates/memory` | Provenance-bound facts |
| `crates/fanout` | Bounded ordered parallel work |
| `proto` | Wire-schema source of truth |
| `eval` | Offline benchmark and release checks |
| `specs` | Threat model and security constraints |
| `docs` | Architecture, roadmap, and wiki |

## Security

Treat agent input, repository files, child-process output, and provider responses as untrusted. Review `specs/threat-model.md` before adding integrations. Do not commit credentials, raw secrets, or unreviewed policy changes. Report security issues privately to the project maintainers rather than publishing an exploit first.

## License

Apache-2.0. See `LICENSE` when distributed as a release artifact.
