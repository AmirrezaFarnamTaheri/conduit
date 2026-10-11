# Conduit revised roadmap

## Decisions that change the draft

1. **Shrink the first milestone.** Phase 0 proves storage, replay, accounting, and deterministic benchmark mechanics before introducing gRPC, SQLite, PTYs, indexing, or provider APIs.
2. **Separate claims from capability.** The project reports corpus reduction and preservation tests independently; it does not claim agent success, cost savings, or RTK parity without live controlled runs.
3. **Use a vertical slice.** The first usable release is `conduit run`/`expand` plus the event log, not twelve partially connected modules.
4. **Make fail-open executable.** Every interception boundary must have a raw fallback test before an optimizer is enabled.
5. **Defer high-risk surfaces.** Sandbox, policy, LLM gateway, and integrations require threat models and contract tests before implementation; they are not Phase 0 dependencies.

## Revised gates

| Phase | Deliverable | Exit gate |
| --- | --- | --- |
| 0 | Store, replay log, token accounting, corpus benchmark, protocol | Tests pass; benchmark is reproducible; external baselines are explicitly pending |
| 1 | Lens vertical slice for generic output plus git/test adapters | Failure signals preserved on golden corpus; expand works; fail-open tests pass |
| 2 | Atlas read-only index and surgical edit transactions | Retrieval and atomicity benchmarks meet recorded thresholds |
| 3 | Ledger masking and provider-neutral relay contracts | Non-inferiority harness exists before any optimization is enabled |
| 4 | Verify, guard, and vault policy boundaries | Adversarial suite passes; loop detector has measured precision/recall |
| 5 | Integrations, memory, fanout, optional semantic layers | Each integration has a coverage report and rollback path |
| 6 | Hardening and release | Audit findings closed; signed reproducible artifacts; public benchmark report |

## Phase 0 status

Implemented in this repository: Rust workspace, BLAKE3 content-addressed store, deterministic handles, crash-safe staged writes, append-only replay log, protobuf source schema, local corpus benchmark, and initial design note. Pending by design: live raw-agent/RTK comparison, because no agent versions or authorized provider test environment were supplied.

## Non-goals for Phase 0

No daemon, network listener, shell interception, model routing, semantic cache, sandbox, external telemetry, or claims about production task success.
