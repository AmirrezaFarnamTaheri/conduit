# Phase 0 design note: measurement before optimization

Conduit starts with a small, dependency-light core rather than the full daemon, gateway, and index stack. The first gate is reproducibility: a content-addressed blob must round-trip, an event log must replay byte-for-byte, and the benchmark must report measured token counts. No savings claim is valid until the same task corpus can be run against an unmodified baseline and an instrumented path.

The initial storage API uses BLAKE3 blobs and short `cx:o:` handles. Writes are staged to a temporary file and renamed, so a crash cannot expose a partially written final blob. The event log is append-only and calls `sync_data` after each event. SQLite, gRPC, and OpenTelemetry are intentionally deferred until their first measurable consumer exists; adding them now would increase attack surface and make the first benchmark less reproducible.

The benchmark currently measures deterministic corpus reduction only. Live-agent and RTK comparisons are not fabricated: they require a pinned task set, agent versions, provider configuration, and an authorized execution environment. Phase 0 therefore publishes a local baseline and marks external comparisons as pending.
