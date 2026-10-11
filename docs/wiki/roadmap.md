# Roadmap and acceptance

## Phase 0 — measurement

Storage, event accounting, protocol schema, and the offline corpus harness exist. Rust tests and live-agent baselines must run before declaring the numeric gate complete.

## Phase 1 — lens

Fail-open execution, reversible output, deterministic reduction, and exit-code preservation exist. Structured command adapters, exact stream ordering, and a pinned answer-preservation corpus remain acceptance work.

## Phase 2 — retrieval and edits

Snapshot search and hash-anchored edits exist. Full tree-sitter/SCIP intelligence, incremental indexing, AST edits, crash-injection tests, and multi-file durability are later acceptance gates.

## Phase 3 — context and relay

Ledger masking and hard budget primitives exist. Provider contracts, cache economics, gateway streaming, and non-inferiority experiments remain separate work.

## Phase 4 — safety and feedback

Loop guard, test planning, and local policy primitives exist. OS sandbox enforcement, taint propagation, coverage collection, and adversarial cross-platform tests remain required.

## Phase 5 — knowledge and scale

Hash-bound memory and ordered fanout exist. LSP/SCIP precision, embeddings, verified model routing, and dashboard observability are not silently substituted by simple search.

## Phase 6 — hardening and release

Threat model, dependency policy, CI matrix, and release-input checks exist. External audit, signed reproducible artifacts, SBOM, and public benchmark report remain release gates.

A phase is complete only when its implementation and its measurable acceptance evidence both exist. Documentation never converts a blocked test into a passing test.
