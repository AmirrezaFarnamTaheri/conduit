# Conduit Phase 0

This repository contains the first executable vertical slice of Conduit.

## Run the local benchmark

```sh
python3 eval/benchmark.py
```

## Build and test

```sh
cargo test
```

The CLI supports `run -- <command>`, `find <query> [limit]`, `edit <path> <expected-hash> <old> <new>`, `store`, `expand`, `record`, `replay`, `stats`, and `bench`. Set `CONDUIT_HOME` to isolate runtime state and `CONDUIT_WORKSPACE` for retrieval and edits. `run` preserves the child exit code, stores raw output, and applies the deterministic lens only when storage and filtering succeed; any failure falls back to raw output. Atlas results include content hashes, and edits reject stale or ambiguous input and use a workspace transaction lock.

Live-agent and RTK baselines are intentionally not invented; they are a gated follow-up requiring pinned agent versions and an authorized test environment.
