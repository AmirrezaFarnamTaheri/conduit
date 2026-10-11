# Development

## Checks

Run the same checks used by CI:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
python3 eval/benchmark.py
python3 eval/release_gate.py
```

The GitHub workflow repeats Rust checks on Ubuntu, macOS, and Windows and runs `cargo-deny` against `deny.toml`.

## Test expectations

New deterministic behavior should include unit tests for normal input, empty input, malformed input, stale hashes, path boundaries, duplicate operations, concurrency, and failure recovery. Tests must assert observable contracts rather than implementation details. Do not replace filesystem, process, or policy tests with mocks when the behavior under test is the boundary itself.

## Benchmark discipline

A reduction is not a saving claim by itself. Record raw tokens, presented tokens, latency, and task success against an unchanged baseline. Keep corpus fixtures small, versioned, and representative. A benchmark that cannot reproduce its input and configuration is not an acceptance result.

## Code guidelines

Prefer small deterministic functions, explicit ownership, bounded resource use, and fail-open behavior at presentation boundaries. Reject ambiguity rather than guessing. Do not add network calls, provider dependencies, or background processes without a threat-model update and a contract test.

## Change workflow

1. Read the relevant design note and acceptance gate.
2. Add a failing test for the contract.
3. Implement the smallest complete change.
4. Run local checks available in the environment.
5. Record blocked checks honestly in `eval/reports/`.
6. Commit one logical unit with a precise message.
