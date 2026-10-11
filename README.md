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

The CLI supports `store`, `expand`, `record`, `replay`, `stats`, and `bench`. Set `CONDUIT_HOME` to isolate runtime state. Raw blobs are never discarded when a short handle is returned.

Live-agent and RTK baselines are intentionally not invented; they are a gated follow-up requiring pinned agent versions and an authorized test environment.
