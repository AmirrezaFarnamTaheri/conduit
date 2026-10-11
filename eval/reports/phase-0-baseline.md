# Phase 0 baseline report

Date: 2026-10-11

## Local deterministic corpus

| Corpus | Raw tokens | Presented tokens | Reduction |
| --- | ---: | ---: | ---: |
| sample.log | 24 | 9 | 62.50% |

The sample preserves an error location, the test summary, and exit state while collapsing repeated warning noise. This is a mechanics check, not a production savings claim.

## Execution status

- `python3 eval/benchmark.py`: passed.
- `cargo test`: not run successfully because the execution environment does not provide `cargo`.
- Live raw-agent and RTK baselines: pending; no pinned agent versions or authorized provider environment were supplied.

## Gate decision

Phase 0 is **not complete**. The repository slice is implemented, but the exit gate remains blocked until Rust tests run in an environment with the Rust toolchain and the live comparison protocol is supplied. Phase 1 must not begin based on this report alone.
