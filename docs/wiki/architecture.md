# Architecture

Conduit is organized as deterministic local libraries behind a thin CLI. The intended production daemon can later expose the same contracts over IPC without changing the core safety rules.

## Data flow

1. A command is authorized by policy.
2. The child runs with its exit status preserved.
3. Raw bytes are captured and stored by content hash.
4. The lens classifies and reduces output while retaining decision-relevant signals.
5. The presented result includes a handle if anything was omitted.
6. Scope records accounting and replay events.
7. Atlas and ledger provide hash-aware retrieval and context state.
8. Edit and verify operate only on fresh, bounded inputs.

## Module responsibilities

- **Store:** immutable blobs, deduplication, expansion handles.
- **Scope:** append-only event records and typed totals.
- **Lens:** output presentation; it must never alter child behavior.
- **Atlas:** coherent read snapshot with file hashes.
- **Edit:** stale-read rejection and all-or-nothing staged writes.
- **Ledger:** live/stubbed/evicted context lifecycle and pinned items.
- **Relay:** token, money, and wall-clock authorization.
- **Guard:** repeated action and consecutive failure detection.
- **Verify:** impacted-test selection with widening when coverage is incomplete.
- **Vault:** command, path, and workspace authorization.
- **Memory:** facts bound to source hashes.
- **Fanout:** bounded parallel execution with stable result ordering.

## Invariants

- A failed optimizer returns raw behavior where possible.
- A handle never substitutes for the underlying retained content without an expansion path.
- A hash mismatch prevents an edit or stale fact from being used.
- A budget check fails closed before a bounded operation proceeds.
- Parallel execution may change completion order, never returned order.
- Policy decisions occur before side effects.

## Deliberate boundaries

The current implementation does not pretend to be a full LSP/SCIP index, provider gateway, OS sandbox, or PTY multiplexer. Those require separate contracts, platform tests, and threat-model review; the current crates are deterministic foundations rather than fake implementations of those systems.
