# Conduit

Conduit is a **local coding-agent tool-output and source-navigation prototype**. It aims to minimize noisy tool output while preserving recoverable raw observations and content hashes for subsequent inspection and edits.

## Implementation status (source snapshot)

The CLI currently implements `run`, `find`, `peek`, `edit`, `batch-edit`, `store`, `expand`, `doctor`, `record`, `replay`, `stats`, `repair-session`, and `bench`. The independent `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, and `fanout` crates are **not wired into the CLI**, and their behavior is **not an execution security boundary**. No daemon, provider gateway, UI, MCP server, remote API, OS sandbox, verified multi-file crash transaction, or data-retention controller is supplied.

The CLI launches local commands with the **same privileges and environment as the caller**. It does not enforce the `vault` policy module or isolate the child process. Run only trusted commands within an appropriately isolated operating-system account or managed sandbox. On Unix, timeout signals the child process group, but descendants that leave the group may survive; on other platforms only the direct child is terminated. The wrapper is not a PTY and does not preserve stdout/stderr temporal interleaving.

## Build and checks

Requires a current stable Rust toolchain with Cargo and Python 3.12 for the evaluation harness:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --bin conduit
python3 -m unittest discover -s eval -p 'test_*.py'
python3 eval/benchmark.py --binary target/debug/conduit
python3 eval/release_gate.py
```

The Python release-input check verifies only file presence and fixture sanity. The benchmark executes the **actual Rust CLI output-presentation path** (including recovery-hint overhead and a raw-expansion check) using deterministic fixtures. It counts **whitespace-separated words, not actual model tokens**, and is not evidence of successful agent tasks or production cost savings. The Rust toolchain was unavailable in the environment used to prepare the accompanying audit patch; compilation and cross-platform results must be established independently.

## Commands

```sh
# Default run: bounded output and timeout, but no compression
conduit run -- cargo test
# Explicitly reduce successful UTF-8 stdout if net positive with a valid blob handle
conduit run --preset compact -- cargo test
# Other presets: safe, quick, deep. Flags override preset settings if supplied afterwards.
conduit run --preset quick --timeout-ms 10000 --max-output-bytes 1048576 -- command arg

# Read-only search/inspection; find scans file-by-file without a full snapshot
conduit find 'target symbol' 20
conduit peek path/to/file.rs 10 40
# Edits require the entire file's BLAKE3 hash obtained from find or peek
conduit edit path/to/file.rs EXPECTED_HASH OLD NEW
# Multi-target hash-anchored preflight and best-effort commit from a JSON plan
conduit batch-edit --check batch.json
conduit batch-edit batch.json

conduit store 'raw text'
# Read binary or secret input from stdin instead of leaking it into argv
producer_of_bytes | conduit store -
conduit expand cx:o:OUTPUT_HANDLE
conduit doctor             # verify blob integrity without deleting objects
conduit record SESSION MODULE RAW_WORDS SHOWN_WORDS
conduit replay SESSION
conduit stats SESSION
conduit repair-session SESSION   # explicitly discard only an unterminated tail
conduit bench eval/corpus/sample.log 700
```

`store -` rejects stdin over 64 MiB rather than saving a truncated prefix. `CONDUIT_HOME` selects local blob/session storage (default `.conduit`). `CONDUIT_WORKSPACE` selects the root for `find`, `peek`, `edit`, and `batch-edit`. Both should be protected by normal OS filesystem permissions. On Unix the store/session code attempts restrictive 0700/0600 modes; Windows ACL isolation is **not validated**. Expansion handles are identifiers, not authorization tokens; treat them as sensitive if the blob is sensitive.

`run` retains a bounded amount from stdout and stderr **independently**, drains the rest, returns an explicit incomplete-output error when either limit is exceeded, and returns `124` when the direct child times out. Such incomplete output is **not losslessly recoverable**. When successful stdout is reduced, the original stdout bytes are persisted first and a recovery handle is emitted to stderr. If storage fails or reduction has no net value after counting the recovery hint, stdout passes through unchanged. For non-successful child exits the program leaves stdout uncompressed; the effective exit status is propagated for ordinary exits, with platform limitations for signal termination. Stderr is never reduced.

`edit` and `batch-edit` refuse stale BLAKE3 hashes, duplicate canonical targets, escape paths, protected metadata paths, hardlinked targets, and edits over 8 MiB per file. A batch accepts 1–128 operations up to a combined 64 MiB of original and replacement text. `--check` performs the same validations but does not modify source files; it still creates the cooperative lock. The commit writes with per-file staging. It **does not** guarantee all-or-nothing multi-file transactions across crashes, concurrent writes from other tools, or power loss. See `docs/wiki/batch-edit.md` for a manifest example.

Session records are bounded to 8 KiB each and session logs to 64 MiB. `stats` streams and validates records without materializing the history. If a crash interrupts a JSONL append, `record` and `stats` refuse to gloss over the incomplete tail. `repair-session` preserves a valid unterminated event by appending its newline; a malformed final fragment is saved to a private `.torn-*.bak` file before truncation. Earlier malformed records are never repaired, and quarantine backups require explicit retention decisions. `doctor` provides an on-demand, read-only blob integrity scan; it does not perform retention or GC.

## Repository map

| Path | Responsibility | Current CLI integration |
| --- | --- | --- |
| `src/main.rs`, `src/run.rs` | CLI and bounded direct-child subprocess adapter | Yes |
| `crates/store` | Integrity-checked content-addressed blobs | Yes |
| `crates/scope` | Synchronized and bounded JSONL session events, explicit torn-tail repair | Yes |
| `crates/lens` | Conservative diagnostic-preserving output reduction | Yes |
| `crates/atlas` | File-by-file bounded search, optional snapshot API, and `peek` | Yes |
| `crates/edit` | Shared edit preflight, typed batch manifests, staged edits and cooperative lock | Yes |
| `crates/ledger` | In-memory context state | No |
| `crates/relay` | Stateless authorization *calculation*, not enforcement | No |
| `crates/guard` | In-memory action repetition guard | No |
| `crates/verify` | Basic impacted-test selection | No |
| `crates/vault` | String command policy prototype, not an OS sandbox | No |
| `crates/memory` | In-memory hash-bound facts | No |
| `crates/fanout` | Ordered worker fanout with unbounded channels | No |
| `proto` | Design-only wire schema | No |
| `eval` | Python harness and offline fixtures | CI/gate only |
| `docs`, `specs` | Documentation and threat model | Documentation only |

## Operating and security limits

- **No untrusted command isolation:** unrestricted caller privileges; no sandbox enforcement or provider egress policy.
- **No durable content-retention policy:** the local blob store can grow without garbage collection. Sensitive observations may persist even after a command finishes.
- **No certified multi-file crash recovery:** stage/rename rollback is best-effort and protects only cooperative Conduit instances.
- **No exact-token accounting:** `record` accepts caller-supplied numbers; `stats` reports sums of estimates or claims, including negative net savings, not verified model usage.
- **No workload non-inferiority evidence:** no live-agent success/accuracy or production p95 performance comparisons have been supplied.

See `specs/threat-model.md`, `docs/roadmap.md`, and the external due-diligence assessment before promoting this prototype to production use.

## License

Apache-2.0 in this source snapshot. Any external peer source or third-party dependency must have its actual license checked before redistributing copied code; design-pattern comparison alone does not establish a licensing grant.
