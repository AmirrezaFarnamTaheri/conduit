# Conduit — V3 → V5 incremental technical report

**Assessment:** 12 October 2026  
**Baseline:** original `conduit-main.zip` after application of `conduit_v3_unified.patch`  
**Target:** V5 engineering tree, incorporating all V4 changes and the new V5 improvements  
**Deliverable:** `conduit_v3_to_v5_incremental.patch` (a *single patch*, not a V4→V5 delta)  
**Status:** exact source reconstruction verified; Rust compilation and runtime tests **not** executed.

## 1. Executive decision

V3 established a bounded command-adapter, local hash-addressed storage, structured search and source inspection, hash-checked editing with preflight, session records, and preliminary cross-module tests. Its main unresolved operational problem was that interrupted multi-file renames could leave an inconsistent workspace without persistent recovery instructions. A second limitation was that blob inspection and short byte-window expansion materialized an entire object in memory. Session-tail repair could also discard a JSON event that was syntactically valid but lacked a trailing newline.

**V3→V5 closes several practical reliability and operability gaps while deliberately not claiming transaction atomicity or process isolation.** The delta includes V4's persisted edit receipts and recovery commands, plus V5's independent finalization path, explicit orphan backup cleanup, streaming integrity-checked blob views, and evidence-preserving session repair. It includes tests and documentation describing what is—and is not—guaranteed.

**Deployment decision:** treat the patch as a reproducible source candidate, **not** a certified release. Cargo and rustc are absent from the execution environment; the new Rust tests and the existing platform matrix must run before integration. Reproducible patch application does not imply successful compilation or secure runtime behavior.

## 2. Exact artifact identity and scope

| Property | Value |
|---|---|
| Patch | `conduit_v3_to_v5_incremental.patch` |
| Baseline required | **V3**, not an untouched V1 checkout or an already-applied V4 tree |
| Changed paths | **15** |
| Patch changes | **1,372 insertions / 98 deletions** |
| Patch size | **110,461 bytes** |
| Patch SHA-256 | `c9728d7c83b6ccf70994bbb94074f4df6b35b7819ff42a7ace720df130c467df` |
| Reconstructed target files | **58 / 58**, hash-identical |
| Patch state | `git apply --check` and `git diff --check` passed |
| Test status | Python **2/2** passed; Rust tests **not run** |

The original archive was unpacked independently. Its source was brought to V3 with the previously delivered *V3 unified* patch. The generated delta was then applied to that clean V3 state; all 58 resulting regular project files were compared by SHA-256 to V5, with no missing, extra, or mismatched project files. Transient Python `__pycache__` entries were excluded from the source comparison; they are not part of the patch.

**Complete set of changed paths:**

```text
.github/workflows/checks.yml
Cargo.toml
README.md
crates/edit/Cargo.toml
crates/edit/src/lib.rs
crates/edit/src/recovery.rs          [new since V3]
crates/scope/src/lib.rs
crates/store/src/lib.rs
docs/wiki/architecture.md
docs/wiki/batch-edit.md
docs/wiki/cli-reference.md
docs/wiki/operations.md
docs/wiki/structured-output.md
src/main.rs
tests/cli.rs
```

The report describes the difference **between V3 and V5**; it does not restate the entire older peer-repository audit. The V4 and V5 changes are separated where attribution matters. No code from excluded unrelated archives (`default.zip`, `vox-main.zip`) was included.

## 3. V4 functionality carried into the V3→V5 patch

### 3.1 Persistent edit-recovery receipt and source backups

**Source anchors:** `crates/edit/src/recovery.rs`, `crates/edit/src/lib.rs:51–119`; `crates/edit/Cargo.toml`; `src/main.rs` recovery/doctor branches.

**V3 defect:** The original batch editor staged updates then renamed files individually. If execution stopped between renames, the workspace could contain a mixture of old and new files without a durable description of which were intended or which originals to restore. A cooperative workspace lock did not solve abrupt process exit or power interruption.

**Implementation inherited from V4:** Before the first source rename, the editor writes private backups of original bytes and a versioned JSON manifest recording each canonical target, original digest, intended updated digest, original permission metadata, and staged path. It syncs these artifacts and relevant directories before committing replacements. The resulting receipt blocks further Conduit edit operations until recovery or completion, with `recover-edits --list` / `--json` and an explicit `--rollback ID` procedure. Rollback preflights target content hashes, verifies required backup hashes, refuses conflicting third-party changes, and uses per-file staged replacements.

**Guarantee actually provided:** explicit, integrity-fenced *attempted rollback* of a cooperating, bounded edit batch after interruption. **Not provided:** ACID transaction semantics, atomic visibility to concurrent readers, crash durability on every filesystem/OS, preservation of all file metadata, or safety against active uncooperative filesystem writers and symlink/ancestor swaps. Some file operations may have committed before the crash. A second interruption during rollback can again expose mixed versions until recovery is repeated.

**Method-level design detail:** a receipt is durable prior to source mutation; finishing removes its manifest before deleting old backups. A cleanup crash after manifest removal leaves **orphan backup debris**, not an incomplete pending transaction. Retaining these backups has a confidentiality and operational cost.

**Operational validation required:** execute the integration test with `--features test-fault-injection`; kill the process after its first commit; inspect pending receipt classification; reject new edit attempts; restore originals; verify content, permissions and cleanup on Linux/macOS/Windows and the intended filesystem types.

### 3.2 Bounded expansion / blob metadata commands

**Source anchors:** `crates/store/src/lib.rs`; `src/main.rs:390+`; `tests/cli.rs`.

V4 introduced `inspect` and explicit `expand --offset N --length N`, allowing byte-accurate inspection without dumping the complete result into the agent conversation. `doctor` also aggregates pending transaction count, conflicting target count and orphan recovery directories. Byte-window expansion remained memory-expensive in V4 because it called the full `get()` before slicing the result; V5 changes that internal behavior without changing the byte-oriented CLI output contract.

### 3.3 Crash-hook coverage and delivery pipeline

**Source anchors:** `.github/workflows/checks.yml`; `Cargo.toml`; `tests/cli.rs`.

The V4 portion enables opt-in process-interruption testing through a `test-fault-injection` feature. The standard Rust checks and targeted CLI recovery tests are part of the CI definition. The normal production build does not include the crash hook. Whether this matrix succeeds is unverified locally.

## 4. New V5 implementation and analysis

### V5-01 — Streaming integrity verification for blob inspection and byte windows

**Category:** high-leverage performance and correctness improvement.  
**Priority:** High | **Confidence in source mechanics:** High | **Runtime confidence:** Unverified | **Effort:** M.

**Evidence and root cause.** In V4, `BlobStore::inspect()` called `get(handle)` to obtain a `Vec<u8>`, then calculated its length/digest. `expand HANDLE --offset ... --length ...` also called `get()`, allocating the whole object before taking a small slice. The storage limit was 64 MiB, so a diagnostic metadata query could require a ~64 MiB payload allocation; repeated agent inspection amplified heap churn and memory pressure. The full hash verification was necessary, but retaining the complete object was not.

**V5 change.** `crates/store/src/lib.rs:166–283` now separates handle resolution from verification and adds `verify_blob_window()`: it opens a regular bounded file (with Unix final-component `O_NOFOLLOW`), hashes all bytes through an 8 KiB stack buffer, and materializes either the entire verified object, a requested half-open byte window, or no payload at all for `inspect`. `get_range(handle, offset, length)` validates a nonzero window of at most **1,048,576 bytes**, checks offset addition for overflow and rejects out-of-bounds requests. Both range and metadata queries verify the **whole object's** BLAKE3 digest and observed byte count before returning success. `audit()` uses the same streaming checker rather than accumulating each blob. `src/main.rs:390+` uses `get_range` for windowed expansion.

**Effect.** Metadata reads no longer retain the blob; range output retains at most the requested 1 MiB plus fixed I/O buffers. **I/O work is still O(blob size),** since even a 4-byte request must verify the full object; no time or throughput gain is claimed without benchmarks. Full, unwindowed expansion still allocates the full bounded object. The source only validates a final path component on Unix, not a race-proof directory chain or malicious concurrent mutation protection.

**Negative tests added.** A multi-megabyte object is inspected and sliced across an internal chunk boundary; zero-length, oversized, overflowed and out-of-range windows are rejected. Tampering with bytes *outside* the requested window must make the entire request fail, preventing false partial-integrity claims (`crates/store/src/lib.rs:499+`). The Rust tests have been authored, not executed here.

**Acceptance gates.** Rust unit and CLI tests pass; a memory profiler demonstrates inspection and range memory independent of blob size up to the output buffer; deliberate corruption at the beginning/middle/end of a blob fails; native platform tests confirm no accidental binary recoding and no partial stdout on hash mismatch.

### V5-02 — Finalize fully committed but uncleaned edit batches

**Category:** high-leverage recovery workflow.  
**Priority:** High | **Confidence in source mechanics:** High | **Runtime confidence:** Unverified | **Effort:** M.

**Evidence and root cause.** V4 recovered interrupted batches by rollback. That operation was appropriate when some files remained original, but unnecessarily destructive when *every* target had already reached its intended updated digest and only receipt cleanup was interrupted. An operator could either revert a completed successful batch or manually delete state, neither desirable.

**V5 change.** `crates/edit/src/recovery.rs:319+` implements `Journal::finalize()`, exported through `conduit_edit::finalize_edit` (`crates/edit/src/lib.rs:172+`) and CLI `conduit recover-edits --finalize ID` (`src/main.rs:347+`). It acquires the cooperative workspace lock; validates receipt structure; verifies each target matches its intended updated digest; verifies **every** retained original backup against its recorded digest; then invokes the existing cleanup. If any source is only partially updated, missing, externally changed, or any backup corrupt, **no cleanup starts**. It does not perform missing edits, select a winning source version, or change source files.

**Operator decision:** choose `--rollback` if the batch is partially applied and restoring originals is intended. Choose `--finalize` only if *all* target hashes are already updated and should remain updated. If `conflicting > 0`, investigate manually; neither option should be forced. The complete and conflict-free case is an **accept-completed-transaction** operation, not automatic roll-forward.

**Tests added.** A simulated post-final-rename interruption leaves a pending receipt; `--finalize` preserves updated source contents, removes its recovery files, and restores `doctor` to a healthy result. Separate tests refuse partial commit and mismatched backup (`crates/edit/src/recovery.rs:577+`; `tests/cli.rs:306+`). The fault hook is feature-gated (`crates/edit/src/lib.rs:103+`).

**Limitations.** The preflight and cleanup are separate filesystem operations and only serialized against Conduit cooperating editors; they are not immune to external races. Source paths are canonicalized with standard filesystem APIs rather than handle-relative operations resistant to ancestor symlink replacement. The operation does not strengthen multi-file atomic visibility.

### V5-03 — Explicit, validated orphan recovery-backup cleanup

**Category:** incremental operational and confidentiality improvement.  
**Priority:** Medium-High | **Confidence:** High in design, unverified at runtime | **Effort:** M.

**Evidence and root cause.** V4's `doctor` detected recovery directories without a manifest, but provided no bounded operator command to remove them. A crash before receipt publication or after manifest deletion could leave source backup contents indefinitely. Blind `rm -rf` could discard incident evidence or remove unrelated objects.

**V5 change.** `recover-edits --prune-orphans` (`src/main.rs:353+`, `crates/edit/src/recovery.rs:459+`) holds the edit lock, refuses all pending transactions, inspects every candidate directory *before deleting anything*, requires generated `txn-*` IDs, recognizes only canonical `<index>.backup` regular files within the bounded edit-file size, and refuses any unexpected entry. It reports the directory count after successful cleanup. It is neither automatic nor invoked by `doctor`.

**Boundary:** this is an explicitly destructive maintenance operation for **receipt-less** transaction directories. It does not locate or remove untracked `.conduit-edit-*.tmp` files in source folders and must not be used until forensic retention requirements have been considered. Active non-cooperating writers and hostile directory replacement are still outside its protection.

**Tests added.** Rejects an unexpected file without deleting recognized backups; successfully removes a validated orphan; CI integration verifies `doctor` goes from an orphan-warning exit to a clean result (`crates/edit/src/recovery.rs:618+`; `tests/cli.rs:262+`).

### V5-04 — Evidence-preserving JSONL session-tail repair

**Category:** high-leverage data-integrity correction.  
**Priority:** High | **Confidence in source mechanics:** High | **Runtime confidence:** Unverified | **Effort:** S-M.

**Evidence and root cause.** In V4, a session JSONL record missing its final newline was classified as a torn tail, and `repair_partial_tail()` truncated it regardless of whether its JSON contents were valid. A crash after a full event write but before writing the newline could silently lose a complete accounting event. Malformed fragments were discarded without retaining the original bytes by default.

**V5 change.** `crates/scope/src/lib.rs:229+` adds `TailRepair` with three explicit results:

| Last-record condition | New behavior | Data outcome |
|---|---|---|
| Already terminated / empty | `Unchanged` | No modification |
| Valid JSON event, only newline absent | `CompletedRecord` | Append terminator; retain event in totals |
| Invalid or incomplete trailing fragment | `Quarantined { backup }` | Write private synced `.torn-*.bak` fragment **before** truncating log |

Earlier complete lines must still parse, within the existing size bounds. `repair-session` reports `action=unchanged`, `action=completed_record`, or `action=quarantined backup=PATH`. The original boolean API remains as a compatibility wrapper. A **malformed complete interior line** remains an error rather than being silently repaired. The quarantine write uses exclusive file creation and Unix `0600`; the session directory is also kept private on Unix. Platform ACL validation is still necessary, especially on Windows.

**Regression tests added.** A valid unterminated record must be preserved and included in totals; a malformed fragment must survive byte-for-byte in the backup; a broken earlier complete line must still block repair. Tests also ensure the emitted backup is removed during test cleanup (`crates/scope/src/lib.rs`, `tests/cli.rs:284+`).

**Remaining concern:** quarantine creates extra retained copies of potentially confidential session fragments. It has no automatic expiry or secure-erasure guarantee. Treat these files as sensitive data in backup, access-control and retention policy.

### V5-05 — CI, documentation and operator contract alignment

The V3→V5 patch extends CI to run the explicit process-interruption test feature and updates the reference commands, architecture notes, batch-edit guide, recovery operations, capabilities flags and structured-output descriptions. V5 now advertises `edit_recovery_finalize` and `explicit_orphan_pruning`, while retaining `multi_file_crash_atomic=false` and `os_sandbox=false`. Supported JSON diagnostics and raw byte output remain distinct interfaces. These capability strings are source-declared features, not independent attestation that the current build works.

## 5. Recovery decision matrix

| Observed receipt state | Suggested response | Why |
|---|---|---|
| No pending receipt and doctor healthy | No action | Ordinary settled state |
| Every target has recorded original hash | `--rollback ID` if intentionally abandoning operation | Keeps originals and removes receipt after checks |
| Some original, some updated; no conflicts | `--rollback ID` | Prevents treating partially committed state as successful |
| Every target has updated hash, no conflicts | `--finalize ID` | Preserve intended completed edits; clean recovery state |
| Target differs from both hashes, or missing | Stop and investigate | External mutation or damage; automated choice unsafe |
| Backup missing or hash mismatched | Stop and investigate | Cannot trust recovery source bytes |
| No receipt but orphaned backup directories reported | Preserve evidence, then consider `--prune-orphans` | Explicit controlled disposal of manifest-less debris |
| JSONL tail is valid event without newline | `repair-session` | Completes delimiter; retains event |
| JSONL tail is malformed | `repair-session` after investigation | Saves fragment privately, then truncates |

**Failure-mode distinction:** a receipt allows detection and recovery; it does not make a set of per-file renames a single atomic commit. The current edit system is inappropriate for invariants requiring readers to observe an all-or-nothing cross-file snapshot without additional worktree/version-switch infrastructure.

## 6. Executed validation and evidence standard

| Check | Status | Interpretation |
|---|---|---|
| Original V3 reconstruction from the uploaded archive and V3 patch | **Passed** | Known baseline |
| `git apply --check` of V3→V5 patch | **Passed** | Patch applies without conflicts |
| `git diff --check` | **Passed** | No whitespace-error findings |
| SHA-256 comparison after independent patch application | **Passed — 58/58** | Every project file matches V5 target bytes |
| Python benchmark-validation unit tests | **Passed — 2/2** | Python harness remains parseable and enforces its tested invariants |
| Static release-input check | **Passed** | Required files and corpus fixtures present |
| Cargo manifest TOML parse | **Passed — 14 files** | TOML syntactically valid, not dependency-resolution verified |
| GitHub Actions workflow YAML parse | **Passed** | Workflow YAML parses, not evidence of successful CI execution |
| Rust `cargo fmt`, `cargo test`, `cargo clippy`, `cargo build` | **Not run** | No local `cargo` / `rustc` toolchain |
| New V5 recovery unit and CLI tests | **Authored; not run** | Behavior requires native compilation and execution |
| Real crash/power-loss, multi-OS tests | **Not run** | Durability is still an engineering claim to validate |
| Memory, throughput, agent-task benchmarks | **Not run** | No empirical speedup or outcome claim |

The exact patch-to-tree replay is a **strong reproducibility result**. It is **not** evidence of compiler success, race-free behavior, crash durability, production security, or throughput gains. The V5 code has been reviewed at the source level, but its runtime guarantees remain conditional on the release gates below.

## 7. Highest unresolved risks after V5

### P0 — Compilation and CI assurance

There is no verified Rust build from this environment. Run formatting, all workspace unit/integration tests, the opt-in fault-injection suite, Clippy with warnings denied, all supported OS builds, and dependency checking. If any check fails, fix the source and regenerate this delta; do not merge on patch-applicability evidence alone.

### P0 — Sandbox and subprocess containment

`conduit run` remains a process wrapper with bounded capture and best-effort process-group cleanup, **not an OS sandbox**. It inherits the caller's privileges; Windows process-tree containment and daemonized child isolation remain incomplete. Sensitive repositories and untrusted commands require an external OS isolation mechanism before use.

### P0 — Multi-file atomic visibility and hostile filesystem races

Receipt-backed edits improve recovery but cannot guarantee cross-file all-or-nothing visibility, preserve all ACL/xattr ownership metadata, or protect against adversarial symlink swaps on intermediate path components. Consider a versioned worktree with atomic active-root switching for readers requiring consistent snapshots, and handle-relative platform APIs for stronger path binding.

### P1 — Recovery and evidence lifecycle

Untracked pre-manifest staging files outside the receipt directory can survive crashes. Backup files and quarantined session fragments may expose sensitive source or output content. Add an explicit retention and forensic-handling policy; consider a private, application-owned staging directory with inventory and secure cleanup controls. Avoid automatic deletion without a recovery safety proof.

### P1 — Large-object operation and storage lifecycle

`get_range()` is bounded in **memory**, not disk I/O: every request still hashes the complete object. Bulk reads can contend with builds on slow disks, and there is no installed blob retention/garbage collection policy, cross-process storage quota, or resumable integrity index. A future access pattern analysis should determine whether immutable generation indexes or verified chunk-level hashes are justified.

### P1 — Performance benchmark credibility

The existing Python benchmark and static release gate exercise narrow text/corpus behavior. They do not independently establish token savings, latency, task success, error-rate impact, or real coding-agent reliability. Maintain separate performance datasets for build logs, test diagnostics, symbol lookup, structural changes, multi-file edits, and recovery incidents. Track baseline throughput and failure rates before claiming measurable improvements.

### P2 — Other peer-project convergence

Earlier peer work identified AST-based navigation (ast-grep, diffsitter), structured retrieval (CodeSeek, Ragit), reversible reduction (RTK), and durable indexing/journaling patterns. Those remain candidate **future** integrations, not V5-implemented features. Large-scale merger without validating use cases, memory budget, compatibility and complexity costs is not recommended. This patch deliberately improves one bounded core vertical slice rather than introducing another unverified subsystem.

## 8. Required acceptance tests before merging

1. Build native Rust code on a fresh checkout. Fix any parser/type errors and Clippy warnings; re-run diff and hash validation afterward.
2. Run complete workspace tests and targeted fault-injection tests on at least Linux, macOS and Windows CI. Independently verify which crash hooks execute in the integration-test binary.
3. Exercise a multi-file batch interrupted after the first commit; confirm `--finalize` rejects it and `--rollback` restores originals.
4. Exercise a batch interrupted after the last commit; confirm `--finalize` preserves the edited files and removes the receipt.
5. Tamper with each backup and source hash independently; confirm the operator commands fail closed and do not modify earlier targets when preflight fails.
6. Introduce an unexpected or symlinked object into an orphan transaction; confirm `--prune-orphans` refuses it without removing recognized backups. Test races where an uncooperative process edits the directory and document behavior.
7. Create a valid JSONL record without a newline; ensure repair retains the event. Create a malformed fragment; ensure its exact bytes are synced to a private backup before truncation.
8. Corrupt a large stored blob outside a requested byte window; verify `expand` outputs **no bytes**, returns failure, and metadata inspection fails.
9. Measure peak RSS/allocator behavior for `inspect` and 4 KiB window access against 1/16/64 MiB blobs. Compare V4 and V5 with identical inputs.
10. Validate permission and file-descriptor behavior on target filesystems, including Windows ACL handling, case folding, antivirus locks, network filesystems and symlink behavior as applicable.

## 9. Applying this exact patch

**Prerequisite:** a clean V3 working tree; *do not* apply this delta to a V4 tree, since it already contains most recovery code. If starting with the untouched archive, first apply the prior V3 unified patch.

```bash
# In a clean Conduit source tree at the V3 state:
git apply --check /path/to/conduit_v3_to_v5_incremental.patch
git apply /path/to/conduit_v3_to_v5_incremental.patch

# Required verification (must run in a Rust-enabled environment):
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo test -p conduit --features test-fault-injection --test cli
cargo clippy --workspace --all-targets -- -D warnings
cargo build --bin conduit
python3 -m unittest discover -s eval -p 'test_*.py'
python3 eval/benchmark.py --binary target/debug/conduit
python3 eval/release_gate.py
```

For a clean original archive, the ordered sequence is `conduit_v3_unified.patch` **then** `conduit_v3_to_v5_incremental.patch`; nothing from V4 needs to be applied separately. This is an incremental patch and not an independent V5 unified diff.

## 10. Final verdict

**Strongest result:** reproducible V3→V5 integration with meaningful recovery, data-integrity and bounded-memory improvements. The delta has been independently replayed against the exact supplied baseline, including the recovery source file originally absent from V3.

**Largest remaining uncertainty:** native Rust compilation, real crash recovery and cross-platform filesystem behavior. No claim of a fully tested build, atomic transaction, sandbox security or measured 10× improvement is supported yet. The next highest-return work is to certify the existing implementation with executable tests and a reliable release pipeline, then add transactional worktree semantics and enforceable subprocess isolation only if the requirements justify them.
