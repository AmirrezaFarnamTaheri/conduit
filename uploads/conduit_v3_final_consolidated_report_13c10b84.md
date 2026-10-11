# Conduit V3 — Unified Technical Due Diligence, Peer Code Review and Complete Remediation Patch

**Assessment date:** 2026-10-11  
**Integration target:** the byte-exact original `conduit-main.zip`, not a previously patched workspace.  
**Companion artifact:** `conduit_v3_unified.patch` is **the only patch required**.  
**Assurance label:** source-grounded and patch-roundtrip verified; **not Rust-compiled or production certified**.  
**Decision:** conditionally appropriate for trusted local evaluation after native CI; **not approved as an untrusted-agent sandbox or crash-atomic edit engine**.

> **Reading instructions.** This is a standalone report: no previous audit, peer addendum, or prior patch is required. The detailed source-review sections below explicitly describe the **original unmodified Conduit baseline**, while the V3 implementation inventory describes the **patched candidate**. Historical baseline defects may be remediated in V3; never interpret a baseline finding as necessarily outstanding after patch application. All peer source references refer to internal paths inside the originally supplied ZIPs.

## 1. Executive decision and highest-value changes

Conduit is a local coding-agent command/output and source-navigation prototype, not a complete platform. The original executable called five of its 12 library crates directly while several policy, memory, verification and budget modules were isolated primitives. Source-level weaknesses were concentrated in process execution bounds, data integrity, crash recovery, policy enforcement, edit aliasing, inaccurate benchmarks, and missing operational feedback.

**V3 meaningfully narrows the correctness gap**, building on the original and the two previous remediation rounds. It adds a bounded child-process adapter, typed and synchronized session events, content-hash-checked blob storage, fail-closed recovery of torn session logs, ignore-aware deterministic search with bounded excerpts, hash-anchored `peek` and batch edits, recovery-aware compaction, machine-readable diagnostic contracts, and an actual Rust-CLI-path benchmark gate. The patch also adds targeted tests and capability declarations that explicitly state missing safety features.

**It does not solve the two most fundamental architectural safety gaps:** arbitrary commands still run with caller privileges without a verified OS sandbox; multi-file edits still consist of staged per-file renames and lack a durable cross-file journal, fencing or crash recovery. The existing `vault` module is not enforced on the `run` path. Windows process-tree containment, quotas/retention, secret-sensitive observability and real task-outcome proof also remain absent. Consequently, suitability for security-sensitive or autonomous production operations remains **NO-GO** without external compensating controls.

### V3 implementation and readiness scorecard

| Dimension | V3 candidate status | Evidence | Decision |
| --- | --- | --- | --- |
| Rust build correctness | **Unverified** | Cargo/rustc unavailable locally; fresh Rust CI must run | P0 release gate |
| CLI runtime integration | Improved; command, store, scope, lens, atlas and edit wired | `src/main.rs`, `src/run.rs` | Conditional local evaluation |
| Source search and inspection | Bounded deterministic lexical scan; typed JSON; clipped `peek`; hashes | `crates/atlas/src/lib.rs`, `src/main.rs` | Use with incompleteness flags |
| Command resource bounds | Direct-child deadline, independent output caps, early stop on overflow; Unix process-group cleanup best effort | `src/run.rs` | Not process-tree isolation |
| Raw output integrity | Hash-verified retrieval, private Unix staging mode and health scan | `crates/store/src/lib.rs` | Retention/ACL work remains |
| Context reduction | Net-positive only including visible stdout recovery hint; raw persisted before shortening | `src/main.rs`, `crates/lens/src/lib.rs` | No tokenizer/task quality proof |
| Editing | Hash preflight, canonical dedup, hardlink rejection on Unix, staged best-effort rollback | `crates/edit/src/lib.rs` | No crash-atomic batch promise |
| Session records | Typed JSONL events, size caps, OS file locks, explicit torn-tail repair | `crates/scope/src/lib.rs` | Not a durable audit ledger |
| Diagnostics and operations | Structured `capabilities`, `find`, `peek`, `doctor`, `stats` | `src/main.rs`, `docs/wiki/structured-output.md` | Useful for adapters |
| Security/policy | No OS sandbox; module-only policy not called | `crates/vault/src/lib.rs`, `src/main.rs` | NO-GO for untrusted execution |
| Deployment and performance | Source validation/CI declarations; no installed artifacts or real-agent replay | `.github/workflows/checks.yml`, `eval` | Native CI + field evidence needed |

**Most valuable short-term investment:** finish one empirically verified source-to-command-to-capture-to-raw-store-to-review-to-edit vertical slice. Do not first absorb optional servers, semantic indexes, hives, editor UIs or cryptographic proofs. The second-order architecture below describes how such components can be added after proving the simpler plane.

## 2. Scope, direct archive evidence and inventory fidelity

Every supplied in-scope ZIP was reopened for V3 verification rather than relying solely on previous summaries. The original Conduit ZIP has **51 regular entries** and exactly matches all 51 entries committed in the patch-generation Git baseline byte-for-byte. The 24 distinct peer ZIPs have **9,067 regular entries**, for **9,118 distinct in-scope regular archive members** across **25 distinct archives**. This corrects the earlier *9,115* figure: a recount found two additional ast-bro members and one additional ast-master member. Every listed ZIP member is unique within its archive in the inspected samples; this count measures ZIP regular members, not behavioral certification.

The peer recheck validated **34 exact source-file/method anchors** against original ZIP bytes and performed a broad lexical scan of **5,072 source files** (Rust, Python, JS/TS, Go, Svelte, Ruby, Lean, Java and related). A loose declaration matcher found **75,023 candidate function declarations**; that figure includes false positives from macros/examples/comments and is only an inventory aid. It does **not** claim 75,023 methods were understood, benchmarked, or proved superior. High-risk and high-transferability methods were then read directly. The original RTK duplicate archives have identical SHA-256; `rtk-ldp` is a documentation site, not a second RTK runtime.

**Explicitly excluded:** `default.zip` (statistical dataset) and `vox-main.zip` (voice-agent product), pursuant to the user's scope instruction. **Retained:** Homebrew tap (distribution), Graphite (state/history patterns), Ragu (proof/evidence concept only) and other peer sources even when no direct module import is justified.

### Grounding table — source locations verified against original ZIP members

Each line number identifies a method, test or design declaration found in the indicated archive. It is not an assertion that all source elsewhere in that archive is production-safe.

| Original peer ZIP | Regular files | Verified method/test or primary mechanism | Transferability decision |
| --- | ---: | --- | --- |
| `homebrew-tap-main.zip` | 5 | `Formula/rtk.rb:1` — `class Rtk < Formula` | Later: binary checksum/package provenance, after releases exist |
| `diffsitter-main.zip` | 148 | `src/diff.rs:798` — `fn myers_diff<'a, T>(a: &'a [T], b: &'a [T]) -> Vec<EditType<&'a T>>` | Later: optional syntax-aware diff with byte-diff fallback |
| `ast-grep-main.zip` | 324 | `crates/outline/tests/python_go_outline_rules.rs:70` — `fn python_rules_extract_signatures() {` | Later: typed AST queries/rewrite with hash-anchored approval |
| `ast-bro-main.zip` | 262 | `src/context.rs:145` — `pub fn run_context(` | Later: symbol context with strict output budget and raw fallback |
| `RAG-Enterprise-main.zip` | 122 | `src/extensions/retrieval.rs:36` — `impl RetrievalStrategy for DefaultRetrieval {` | Inspire provenance, restore checks and explicit retrieval contracts |
| `ai-jail-master.zip` | 145 | `tests/sandbox_escape.rs:262` — `// ── Seccomp tests (normal mode) ─────────────────` | Later: platform-proven isolation; no portable sandbox claims |
| `codeseek-main.zip` | 149 | `rust-core/src/watcher.rs:32` — `pub fn start_watcher(project_root: &Path) -> Result<FileWatcherGuard, String> {` | Later: incremental watch/index, preserving local lexical fallback |
| `grit-master.zip` | 100 | `src/db/azure_store.rs:46` — `impl AzureLockStore {` | Later: concurrent edit reservations only with fencing/revisions |
| `pg_vectorize-main.zip` | 198 | `extension/src/search.rs:190` — `pub fn full_text_search(` | Conditional: hybrid retrieval only at demonstrated scale |
| `project-rag-main.zip` | 70 | `src/mcp_server.rs:45` — `impl RagMcpServer {` | Later: typed agent tool adapter and cancellation contracts |
| `grep-main.zip` | 45 | `tests/test_grep.rs:10` — `unsafe { std::env::set_var("UUTESTS_BINARY_PATH", env!("CARGO_BIN_EXE_grep")) };` | Adopt: binary/ignore/context correctness fixtures |
| `ast-master.zip` | 30 | `derive/src/lib.rs:14` — `pub fn derive_FromPest(input: proc_macro::TokenStream) -> proc_macro::TokenStream {` | Reference: typed grammar structures, not a new parser framework |
| `rtk-develop (1).zip` | 617 | `src/core/guard.rs:21` — `pub fn never_worse<'a>(raw: &'a str, filtered: &'a str) -> &'a str {` | Adopted principle: never-worse output, true raw recovery, hint-visible stdout |
| `icm-main.zip` | 318 | `crates/icm-core/src/fusion.rs:87` — `pub fn rrf_fuse(lists: &[RankedList<'_>], k: f32) -> Vec<FusedHit> {` | Later: scoped memory and deterministic rank fusion |
| `tinyhivemind-main.zip` | 1,340 | `crates/tinyhivemind-hives/src/storage/sqlite.rs:78` — `fn commit<'a>(&'a self, commit: Commit<'a>) -> StorageFuture<'a, ()> {` | Later: version-checked storage and bounded orchestration |
| `ragu-main.zip` | 756 | `README.md:11` — `> ⚠️ **Ragu is under heavy development and has not undergone auditing.** Do not use th` | Reject proof-system import; adopt only evidence-verification discipline |
| `Graphite-master.zip` | 1,125 | `document/graph-storage/src/history.rs:34` — `pub fn from_ordered(deltas: Vec<Delta>) -> Self {` | Later: revision DAG and deterministic undo, not editor UI |
| `memvid-main.zip` | 217 | `src/footer.rs:76` — `pub fn find_last_valid_footer(bytes: &[u8]) -> Option<FooterSlice<'_>> {` | Later: checksummed checkpoint/generation recovery contract |
| `iwe-master.zip` | 467 | `crates/liwe/src/graph/index.rs:27` — `pub fn remove_edges_from_sources(&mut self, graph: &Graph, sources: &HashSet<Key>) {` | Later: source-linked backlinks and stale-edge invalidation |
| `ragit-main.zip` | 502 | `src/index/file.rs:41` — `pub trait FileReaderImpl {` | Later: bounded ingestion/chunk provenance |
| `hydradb-main.zip` | 232 | `crates/telemetry/src/redact.rs:62` — `pub fn is_redacted(name: &str) -> bool {` | Later: redacted telemetry and durability taxonomy; reject distributed DB |
| `tinyrsi-main.zip` | 582 | `README.md:4` — `patches, private shadow evaluations and explicit human approval, plus a` | Later: preregistered paired evaluation with explicit approval |
| `ai-main.zip` | 1,050 | `server/src/pipelines.rs:72` — `pub fn validate_pipelines_with_store_wiring(` | Later: typed pipeline stage validation and readiness |
| `rtk-ldp-master.zip` | 263 | `scripts/build-search-index.mjs:39` — `function extractFrontmatterTitle(content) {` | Adopt docs/release UX patterns, not unrelated Astro frontend |

**Additional checked anchors** include the RTK raw-retrieval helper (`src/core/tee.rs`), ICM context snapshots, Graphite registry deltas, HydraDB writer validation, CodeSeek hybrid-search configuration, RAG Enterprise backup creation, and `ai-main` readiness. The rechecked source hashes are recorded in the working evidence extraction and can be independently recomputed from these archived paths. The first pass's detailed peer comparisons below use more exact source references and explain negative transfer decisions.

### Reproducibility appendix — exact original archive fingerprints

SHA-256 digests identify the exact ZIP bytes used for the method checks. This enables independent verification that an alternate branch, current repository head, or a different release was not substituted for the uploaded source. Re-running an archive scan should use **regular ZIP members** as the counting unit, not directories or decompressed generated files.

| Original in-scope archive | ZIP SHA-256 | Regular members |
| --- | --- | ---: |
| `conduit-main.zip` | `94161f7d055f35d8eee1ee6a316ead6e89a4952a723e578f1737f0647958b77e` | 51 |
| `homebrew-tap-main.zip` | `97878d5111191b43b776cc26bea505f66a9bad53d594b0fc3ee51689724bcc8b` | 5 |
| `diffsitter-main.zip` | `432a615549a041040fb7a1710e53fda1c64943d4cbdec65609aeecd65eb65384` | 148 |
| `ast-grep-main.zip` | `560fb9d42ca198a784150f9fb06f1e459bc0836373f6dabf87ec945eefa18541` | 324 |
| `ast-bro-main.zip` | `e20bff5bcdb42f1fc73b14c3975fb0ea68f51e4a390858ad72f8c5e19973266e` | 262 |
| `RAG-Enterprise-main.zip` | `51e494b119235afbd5ed4d038dd503ef8707cbf6c1c391d6288b4eeebe92c897` | 122 |
| `ai-jail-master.zip` | `09f7b5a5f79984d26a8c7d5b71e94f6f3a527eeb44fbfe22c0498ba6966c0b84` | 145 |
| `codeseek-main.zip` | `5b7f580c584bc644c5bac2f87695f700bf09b68e3a91af44f868bc136ef4faab` | 149 |
| `grit-master.zip` | `0fe1b5c4cb5bdcb2a4dd4a7d83c548073cdaf942ad1d935a9850841b163518e5` | 100 |
| `pg_vectorize-main.zip` | `ff4d71c8ec3ecc3a96ed2e8cb2570dfbe2fb4d490de4b4c8fee50b4262756686` | 198 |
| `project-rag-main.zip` | `d0eb5d19bc275c1f3237a5a522d057980f12e4043eeb77a1ac079dc47e7bff12` | 70 |
| `grep-main.zip` | `a84f8ac8bf76227ba1a920e5e96c33f05cfe1357a128f52d51dba7bcbf207c3f` | 45 |
| `ast-master.zip` | `5d0c9636a96d3e0fb1a14db616019d1d2c41601daaa9183ad95638b272a0dbe0` | 30 |
| `rtk-develop (1).zip` | `494aca088ed22b148c1769d7ec984d538bb8a06795a4a5c3b77245cf62d3a595` | 617 |
| `icm-main.zip` | `025f4fa092c26b36399907de4c5c488291f000da8c100ee3032872e3566fb6b8` | 318 |
| `tinyhivemind-main.zip` | `11a60863a9907e380766014f90306329613fa9d800ad2d38f4e1cf35bfc0a7b6` | 1,340 |
| `ragu-main.zip` | `f9bbca3154b95458232199beab79b271d0529efcecedb479627323896f300049` | 756 |
| `Graphite-master.zip` | `cc219417fe9798e086fb8bdacbc1ae30099761adfd587a2001bd10255d7a3b7b` | 1,125 |
| `memvid-main.zip` | `0b5595e80626de1dac555f0d9487b0faee546f362824efa7f4261ab5f9d131b2` | 217 |
| `iwe-master.zip` | `a12c71531c0160be1297a9e204273bc47eda051fda487efcc5780bc88a4cc432` | 467 |
| `ragit-main.zip` | `e240e28af9ec5b6e6df4848adfbabf019814563bfea87cbb921682a6e90d88a8` | 502 |
| `hydradb-main.zip` | `755a12190af8d068556e17c7b20bfea99cc06142853798be9e16e2ab2230b2a2` | 232 |
| `tinyrsi-main.zip` | `6c887721ca94e75c05336bf814d09b596986a1e87bc585e45f654011a115e611` | 582 |
| `ai-main.zip` | `ef7073684513af953acfc1ff6eddcb23c6cf67f2d58191ea5d3982f27197782b` | 1,050 |
| `rtk-ldp-master.zip` | `9163e6ac026daaf50ef234ea776518a4c86c0adb31130e782e4dcc92f0a3f61e` | 263 |

The 13th later-batch ZIP, `rtk-develop (1)(1).zip`, is **byte-identical** to `rtk-develop (1).zip`: both have SHA-256 `494aca088ed22b148c1769d7ec984d538bb8a06795a4a5c3b77245cf62d3a595`. It is therefore not an additional peer implementation. The explicitly excluded dataset and voice ZIPs are not counted.

### File-level fingerprints for additional directly inspected peer anchors

These paths and SHA-256 digests are for individual **decompressed source files** within the original ZIPs, not for the repository as a whole. They substantiate the method-level comparison and can be independently hashed after extraction.

| Peer | Original source location | Source-file SHA-256 |
| --- | --- | --- |
| `diffsitter-main.zip` | `src/parse.rs:138` | `85be98cbece9ec030501d8f1770ac2cb1bc28a9475a0ad1140bb66128dcb95ce` |
| `ast-bro-main.zip` | `src/squeeze/mod.rs:90` | `ee47b0b134c08420cd3dcfc0ba3742567ac6032d6b76a93a91bd17eda0203551` |
| `RAG-Enterprise-main.zip` | `src/backup/service.rs:27` | `bd678ca91d7471324f5b15ca61ac95553935c3e82fff3c9028ff01d511a960cc` |
| `codeseek-main.zip` | `rust-core/src/config.rs:29` | `62ced8da34ef2139d5e5c7f60a38de9ba27b47665d7579b984b4cc7afaa31cb5` |
| `rtk-develop (1).zip` | `src/core/tee.rs:61` | `e17e5bf700d114d4a6b6f1f8ca4f7950a06c1b2ef2568345bf1c1e625ba1b0ba` |
| `icm-main.zip` | `crates/icm-core/src/context_snapshot.rs:104` | `bd5d47bb1d845bd919e1644c98a4bb164c0adcd0fd0e71dd38d7dd29259fb7ec` |
| `Graphite-master.zip` | `document/graph-storage/src/delta.rs:17` | `1fa56d04727a60f4676cb5bacd6124ac34134848bebaa91c119999398331ed12` |
| `hydradb-main.zip` | `src/shard/write_pipeline.rs:54` | `86be9c0f0c11ddd5e68d11dfe470e33a7ab7867c777c35917dfa5c6a2f4de443` |
| `tinyrsi-main.zip` | `crates/tinyrsi-host/src/lib.rs:40` | `caa078661b30d7a2369233b295a3bfbeca4ec7a9cdb71e9c694db3145e2abdf6` |
| `ai-main.zip` | `server/src/readiness.rs:41` | `e9db5ca5c0499263a3ae348da1d088114cf3a2ca30619dde01c9b6ed491a7a26` |


### Important cross-project non-equivalences

- RTK's `never_worse` applies to a presented string; an agent-facing result must include any additional hint, and raw recovery must work at the receiving adapter. V3 explicitly counts the visible hint and preserves original child stdout first.
- An AST hit or structural diff can supersede substring search for a *grammar-recognized* question but not for malformed source, unknown grammars, arbitrary bytes or an independent authoritative file hash. Keep lexical search as a fallback.
- Hash-addressed storage guarantees data identity when the digest is checked; it does not authorize callers, prove semantic truth or provide retention.
- Graphite revision history, Memvid checksum footers, SQLite `BEGIN IMMEDIATE` and per-file POSIX `rename` are not interchangeable. A durable multi-file transaction needs a precisely defined recovery and visibility contract.
- A policy evaluator is not a process sandbox. The ai-jail peer's Linux-only restriction tests highlight why capabilities must be proved per host platform.
- Hybrid retrieval, server gateways and coordinated hives introduce operational liabilities (model dependencies, state drift, cancellation, retention, secrets, consensus). Their inclusion requires measured workload benefit.

## 3. Actual V3 implementation — cumulative from the original archive

The single V3 patch contains all previous rounds. It is not an incremental patch. Its net contribution is **27 modified or added paths** including new `src/run.rs`, `tests/cli.rs`, and `docs/wiki/structured-output.md`. The following are specifically implemented in source, not merely proposed:

| Source path | Implemented mechanism and externally observable contract | Key limit |
| --- | --- | --- |
| `src/run.rs` | Own-child timeout, separate stdout/stderr bounded captures, overflow-triggered kill, best-effort Unix group cleanup, bounded pipe-close wait; new cleanup on reader/`try_wait` errors | No OS isolation, Windows Job Objects or escaped-descendant containment |
| `src/main.rs` | CLI presets and strict flags; `run` non-silent overflow; `capabilities`; structured `find/peek/doctor/stats`; `--require-complete`; stdout-visible raw expansion reference; accurate hint-inclusive accounting | Not a typed remote-agent protocol or tokenizer measurement |
| `crates/store/src/lib.rs` | Private staging at creation on Unix, atomic hard-link publication, shard/name validation, open-descriptor digest verification with `O_NOFOLLOW` for final components, bounded full audit | Parent symlink races/ACLs and retention/GC need more work |
| `crates/scope/src/lib.rs` | Bounded typed JSONL append/read, synchronized totals, validated session IDs at CLI, strict tail checks, explicit manual repair, no-follow final session file on Unix | Not tamper-evident audit retention; ancestor path race limitations |
| `crates/atlas/src/lib.rs` | Ignore-aware file-by-file bounded case-insensitive search, deterministic sorted entry order, full-file source hash, 240-character hit context, ≤512-character peek lines, private/secret-name exclusion, opened-descriptor checks | No language-accurate AST, no persistent index, denylist not confidentiality control |
| `crates/edit/src/lib.rs` | Shared preview/apply validation, typed manifests, hash preconditions, canonical duplicate and hardlink checks, cooperative OS locking, per-file staged durable writes | Multi-file crash/recovery and adversarial concurrent-writer fencing not solved |
| `crates/lens/src/lib.rs` | Prioritized diagnostic signal retention with soft budgets, conservative no-change branch | Can still lose context; no command grammar correctness proof |
| `crates/relay` / `crates/guard` | Improved standalone budget and repetition primitives | Not enforced by CLI |
| `eval/benchmark.py`, `.github/workflows/checks.yml` | Executable-path corpus benchmarks and proposed Rust/Python/CI matrix | CI not run here, no live agent/real token economy evidence |
| `README.md`, `docs/`, `specs/` | Explicit capabilities, operational runbooks, CLI schema and failures | Documentation is no proof of host controls |

The added `capabilities --json` intentionally declares `os_sandbox=false`, `multi_file_crash_atomic=false`, `ast_search=false`, `semantic_index=false`, `tokenizer_metering=false`. This avoids presenting roadmap language as delivered functionality. Read-only JSON surfaces are **one versioned object per invocation** with `schema_version=1`; unknown or lossy path encoding is flagged by `path_utf8=false`. `find --require-complete` returns 75 if file skips or hit/scan limits compromise a complete search. A complete scan still deliberately excludes gitignored files.

### The third expansion's discrete findings and resolutions

These additional observations were reexamined against V2 implementation and relevant peer code, then either fixed or explicitly bounded. Severity reflects consequence under a declared threat model, not unproved exploitability.

| Ref | Root cause / evidence (V2 → V3) | Impact and v3 action | Severity / Confidence | Effort / ROI | Verification |
| --- | --- | --- | --- | --- | --- |
| V3-01 | Confidential blob staging created without creation-time mode (`store.publish`) | Possible exposure before chmod on permissive hosts. **Fixed** Unix `mode(0o600)` before write; parent ACLs remain | High / High | S / High | Symlink/private-mode tests, platform ACL audit |
| V3-02 | Path metadata checked before reopening (`store.verify_blob`) | Symlink replacement between check/read. **Reduced** with `O_NOFOLLOW`, descriptor metadata and bounded read | High / High | S / High | Symlink test; adversarial ancestor race remains |
| V3-03 | Session and edit locks opened by pathname (`scope.open_file`, `edit.Lock`) | Final-component symlink redirection. **Reduced** by creation-time restrictive modes and no-follow Unix descriptor opens | High / High | S / High | Negative symlink tests, Windows ACL testing required |
| V3-04 | `run` drained excess output until child termination | Excessive output could retain readers/CPU. **Fixed** on overflow by requesting prompt process-group stop; failures explicitly incomplete | High / High | M / High | 5-second infinite-writer test added; no descendant guarantee |
| V3-05 | Reader/`try_wait` errors propagated without child cleanup | Error path could leave live subprocess. **Fixed** best-effort stop/wait before return | High / Medium | S / High | Inject reader errors and process table observation |
| V3-06 | Compact expansion hint appeared only on stderr | Stdout-only agents silently lose raw access. **Fixed** recovery marker emitted with presented stdout, measured in savings | High / High | S / High | Unit test for raw exact recovery and visible marker; adapter tests needed |
| V3-07 | Search result order dependent on filesystem enumeration | Different first-N hits and nondeterministic agent context. **Fixed** sort-by-path walker | Medium / High | S / High | New deterministic two-file integration test |
| V3-08 | `peek` printed arbitrarily long lines (≤1 MiB file) | Context/output explosion and hidden budget bypass. **Fixed** 512-character per-line cap with clip counter | Medium / High | S / High | Bounded navigation test |
| V3-09 | Scan and peek could follow final-component changes after initial metadata | Local search could read unexpected bytes. **Reduced** with opened-descriptor size/type and Unix no-follow reads | High / Medium | S / High | Swap-race/platform tests pending |
| V3-10 | No machine-readable navigation/diagnostic output | Fragile grep/regex tool adapters. **Fixed** `--json` across read-only commands with schema version | Medium / High | M / High | JSON parse and schema integration tests added |
| V3-11 | Empty/incomplete searches looked like authoritative results | False negatives and unsafe downstream decisions. **Reduced** `complete`, `skipped`, `truncated`, `limit_reached`, fail-closed optional exit 75 | High / High | S / High | Hit-cap regression integration test |
| V3-12 | Roadmap policy/sandbox claims could be misunderstood as implemented | Operational misuse and governance drift. **Reduced** explicit CLI capability reporting and docs disclaimers | High / High | S / High | Assert negative flags in CLI test; verify across platforms |

**Proof status:** The changed source and tests exist and full-patch replay was verified. Rust-specific test descriptions in this table describe **tests added**, not tests executed successfully; Cargo/rustc were not available. V3 does not claim remaining parent path races, arbitrary external writers, audit-log tampering, Windows ACLs, or child-tree isolation are fully solved.

## 4. System architecture, completeness and threat model

### Runtime truth, not aspirational naming

```text
  trusted caller / agent adapter
              |
       Conduit CLI
              |---------- capabilities (declared limitations)
              +-- run ---------- bounded direct child ---------- presentation
              |                      |                               |
              |                separate stdout/stderr                  v
              |                      +----------> hashed raw store -> expand
              |                                                 |
              +-- find / peek ---- ignored/bounded text ---- file BLAKE3
              |                                                 |
              +-- edit / batch-edit -- cooperative lock <-------+
              |
              +-- scope -- bounded typed JSONL / totals / repair
              +-- doctor -- read-only blob digest scan

  PRESENT BUT NOT CLI WIRED: ledger, relay, guard, verify, vault, memory, fanout
  DESIGN ONLY: proto/conduit.proto, durable transaction journal, AST index,
               MCP/provider adapters, access-controlled memory, token meter
```

A command executed under `conduit run` inherits the caller's permissions and environment. `--preset safe` means conservative timeout/output settings **not OS safety**. Conduit does not enforce authorization policy or restrict filesystem and network access. OS sandboxing must be provided by the host and validated on the target platform. Unix process-group killing is a best-effort cleanup facility, not an isolation guarantee. Failure to distinguish this is a release-blocking misrepresentation for a multi-tenant or untrusted code-execution product.

Search/file paths are a different boundary: file roots and typed edits constrain accidental mistakes and some file injection, but pathname-to-descriptor races and Windows/platform-specific ACLs are not fully eliminated. Neither ignored path patterns nor UTF-8 validation is comprehensive secret scanning. Stored stdout may contain credentials, PII, private source or other sensitive output; no retention, consent lifecycle, time-to-live, quota or safe garbage collection exists.

### Durability decisions

Single-file staging and rename can make one replacement relatively safe on a local filesystem, but a sequence of renames is not a durable atomic multi-file transaction. Preflight is intentionally not a reservation: an external editor can race after hash checking, and a power failure can leave a mixed generation. The next implementation should choose a checksummed durable journal with explicit `PREPARED`, `COMMITTING`, `COMPLETE`, `RECOVERY_REQUIRED` states and sync boundaries—or a single committed manifest/worktree reference—then test interruption after each transition. Never mark a transaction recovered while it may still be partially applied.

### Operational and product opportunity

The near-term product is an excellent **headless local CLI and typed tool adapter**, not a graphical editor or full RAG server. High-value usability surfaces are stable verbs, truthful capability flags, source-linked evidence, preview/error reason codes, terse presets, typed JSON, direct raw expansion, and repair/health commands. TUI widgets such as a run timeline, output-diff preview, structured source inspector and pending edit list become useful **after** provenance, durability and lifecycle are enforceable. Porting an entire peer UI, server, vector DB or orchestration platform would increase risk without establishing demand.


## 5. Deep source audit of original Conduit (baseline evidence)

The following material is retained from the detailed original-source audit, with full architecture/module inventory and the individually justified findings. Source line numbers are anchored to `conduit-main.zip` **before** V3 and are not asserted to be line-identical in the patched project. V3 status and outstanding issues are in Sections 1, 3, 7 and 8 of this report.

### 2. System overview and scope

#### What Conduit intends to be

A provider-neutral local runtime sitting between a coding agent and developer tools, with specialized modules for output reduction, recoverable raw data, search/edit assistance, accounting, budgets, loop detection, memory, and parallel work (`README.md:1-15`, `docs/wiki/architecture.md`).

#### What is actually shipped

- A Rust command-line binary with nine user-facing verbs: `run`, `find`, `edit`, `store`, `expand`, `record`, `replay`, `stats`, `bench` (`src/main.rs:17-97`).
- Twelve small library crates, many usable as independent primitives but not connected into a governed runtime.
- A six-line benchmark fixture and Python reduction/check scripts.
- A minimal threat model, forward-looking protocol schema, architecture wiki, roadmap, and source-check CI workflow.
- No daemon/HTTP endpoint, frontend application, model-provider adapter, MCP server, LSP/SCIP service, secret manager, telemetry exporter, durable edit journal, package installer, executable artifacts, or published live-agent experiment in the supplied archive.

#### Trust boundaries

**Current principal:** the same operating-system user who invokes the CLI. `run` inherits that user's process privileges and typically its environment. There is **no demonstrated isolation** between untrusted agent decisions and local files/commands. The repo threat model properly labels agent input, workspace files and process output as untrusted, but the enforcement structure is incomplete.

**Protected assets:** workspaces, private source, execution environment variables, raw output blobs, session accounting, transaction integrity, and agents' downstream decisions. **Potential adversaries:** malicious source/log content, a mistaken or compromised agent, a lower-privilege local user when state permissions allow access, or a parallel process changing files. Network adversary scenarios are not directly applicable to this offline binary until a listener/provider adapter is introduced.

### 3. Architecture, dependency, asset and workflow analysis

#### 3.1 Runtime dependency map

```text
                          CLI: src/main.rs
                           |        |       
         +-----------------+--------+--------------------+
         |          |             |         |            |
    conduit-scope  conduit-store conduit-lens conduit-atlas conduit-edit
         |          |             |         |            |
     session JSONL  BLAKE3    whitespace   full-tree    path + hash
     manual stats   blobs     heuristics    snapshot    staged renames

    CLI Cargo deps, NOT called by src/main.rs:
       conduit-ledger    conduit-relay     conduit-guard
       conduit-verify    conduit-vault     conduit-memory
       conduit-fanout

    Separate inputs: proto/conduit.proto (not generated or consumed)
                     eval/*.py (not the Rust execution path)
                     .github/workflows/checks.yml (CI checks only)
```

The CLI has all twelve libraries as Cargo dependencies, but importing a crate into the build is not evidence that its runtime guarantees are active. For example, documentation says a command is authorized before execution (`docs/wiki/architecture.md`), yet the only production `run` branch calls `Command::new` directly.

#### 3.2 Comprehensive component inventory

| Asset | Purpose and main dependency | Actual maturity | Primary risk / criticality | Improvement path |
|---|---|---|---|---|
| `src/main.rs` | CLI orchestrator; imports scope/store/lens/atlas/edit | Working-source prototype, compilation unverified | **Critical** - central trust boundary and process fidelity | Typed command dispatch and integrated execution pipeline |
| `crates/store` | BLAKE3 blob persistence and short handles; depends on `blake3`, filesystem | Prototype | **Critical** - confidential data and integrity | Authenticated reads, bounded indexed lookup, private ACL, lifecycle |
| `crates/scope` | Savings-event JSONL and totals; stdlib filesystem | Prototype with syntax defect in test module | **High** - audit/accounting correctness | Strict schema, safe IDs, synchronized writer, linked runtime events |
| `crates/lens` | Signal-first text reduction; stdlib | Prototype | **High** - wrong/missing diagnostic context | Typed parsers, hard budget modes, correctness-based eval |
| `crates/atlas` | Deterministic whole-workspace text search and content hashes; `blake3` | Prototype | **High** - source exposure and whole-tree cost | Ignore-aware search and incremental index, caps |
| `crates/edit` | Hash-anchored search-replace/range editing; filesystem + `blake3` | Prototype with non-atomic commits | **Critical** - source loss/corruption | Canonical identity, journal/worktree, crash and concurrency tests |
| `crates/ledger` | In-memory context item dedup/masking; Mutex + `blake3` | Isolated primitive | **Medium** - memory lifecycle not wired | Prompt assembly, externalized storage, hard budget status |
| `crates/relay` | Pure budget decision function | Isolated primitive | **High** if advertised as enforced | Atomic reservation, timeouts, actual usage reconciliation |
| `crates/guard` | BLAKE3 repetition fingerprints/failure counter | Isolated primitive | **Medium** - loop behavior unmeasured | Integration, tunable windows, precision/recall eval |
| `crates/verify` | Exact-file-set impacted-test planner | Isolated primitive | **High** if used to omit required tests | Dependency/coverage graph, safe widening and baseline full suite |
| `crates/vault` | Textual command profile allowlists | Isolated primitive and **not a sandbox** | **Critical** if trusted for authorization | Capability API, canonical paths and OS enforcement |
| `crates/memory` | In-memory facts bound to caller-supplied source hashes | Isolated primitive | **Medium** - weak provenance semantics | Namespaced source identity, revision and verification |
| `crates/fanout` | Ordered thread pool with unbounded channels | Isolated primitive | **Medium** - hangs, resource growth | Backpressure, typed failures, cancel/deadline, workload limits |
| `proto/conduit.proto` | Declares session, event and tool-result wire messages | Design-only | **Medium** - schema/code drift | Generate runtime types and version compatibility tests |
| `eval/benchmark.py`, corpus | Toy text-reduction exercise | Passing but weak | **High** - invalid extrapolation to agent value | Paired real-task benchmark and CLI parity test |
| `eval/release_gate.py` | Checks presence of 4 files and signal marker | Minimal input sanity check | **High** if used as release evidence | Verifiable comprehensive release gates |
| `.github/workflows/checks.yml` | Matrix for Rust/Python, cargo-deny | Configured; remote outcomes unknown | **High** - build must first be repaired | Lockfile, SHA pins, coverage/fuzz, packaging & attestations |
| `deny.toml` | Advisory/license/source checks | Configured, execution unverified | **Medium** | Reproducible resolved dependency graph; policy review |
| `.gitignore` | Excludes target/state/python caches from VCS | Minimal | **Low** | Secret policy, fixture hygiene, build artifact review |
| `README.md`, `docs/`, `specs/` | Goals, design, operations, threat model | More mature than runtime integration | **Medium** - claims/gates drift | Source-generated capability matrix and operator runbooks |

**Absent rather than silently invented:** database, network/API server, provider credentials, external storage and monitoring integration, Kubernetes/containers, deployment topology, browser/UI, persistent queue, secrets inventory, and service-oriented production operations. Treat any proposed solution in these areas as future architecture, not current inventory.

#### 3.3 Critical workflow traces and exact assumptions

**A. Command and reduction path - `src/main.rs:20-44`:**

1. Parse program and raw string arguments, with optional `--`.
2. Spawn arbitrary executable using `std::process::Command::output()`; no `vault`, `relay` or `guard` callback.
3. Buffer `stdout` and `stderr` independently, concatenate as `stdout || stderr`, then lossy-decode to UTF-8.
4. Try to write original concatenated bytes into the blob store.
5. Distill text at a hardcoded *700 whitespace-token* budget.
6. If dropped and blob exists, print filtered text and expansion footer on stdout; if dropped and store fails, print decoded raw; otherwise print filtered passthrough.
7. Exit with child's integer exit status, or 1 when `status.code()` is absent. No session event is emitted.

This is **not** byte-identical transparent process execution, not an enforced command policy, not a streaming proxy, and not a bounded executor. Its fail-open property is narrower: failures to store or reduce output are designed to show a raw **text rendering**, rather than crash the child.

**B. Retrieval - `src/main.rs:45-51`; `crates/atlas/src/lib.rs`:** Read every eligible file recursively into a process-local vector; skip only `.git`, `.conduit`, `target`, symlink directory entries and files >2 MiB/NUL-containing; abort if a candidate is otherwise non-UTF-8; search ASCII-case-insensitively and return line/path/hash. No `.gitignore` or sensitive-file semantics. Returned hash is from snapshot build, not revalidated on display.

**C. Editing - `src/main.rs:52-60`; `crates/edit/src/lib.rs`:** Canonicalize workspace -> create exclusive lock file -> read and hash each named target -> prepare replacement -> write staged temps -> re-read and compare hashes -> rename staged files over originals one by one -> best-effort in-memory rollback if a rename fails. It serializes cooperating `apply()` calls, but not other editors, external processes, aliases of one canonical file, process termination mid-commit, or power loss.

**D. Storage and expansion - `src/main.rs:61-72`; `crates/store/src/lib.rs`:** Store a blob under `<root>/<first-two-hash-chars>/<full-blake3-hex>`; derive a truncated base32 handle from first 12 digest bytes; `expand` scans the entire blob hierarchy to find a filename producing that handle, then returns bytes without digest verification. No TTL, GC, encryption, authorization or quota exists here.

**E. Session accounting - `src/main.rs:73-90`; `crates/scope/src/lib.rs`:** `record` accepts caller-provided numbers and writes `measured:false`; `stats` sums fields parsed from JSONL lines; `replay` prints serialized lines. There is no automatic linkage from an executed command to its saved blob, reduced output, timing, model tokenization, cost, policy decision or session lineage.

**F. Auxiliary modules:** `ledger`, `relay`, `guard`, `verify`, `vault`, `memory` and `fanout` have APIs/unit tests but no CLI entrypoints or integration path in this snapshot. Their invariants cannot be projected onto `conduit run`.

#### 3.4 Failure propagation / resource lifecycle

| Failure or stressor | Current likely response | Consequence |
|---|---|---|
| Child never exits | `Command::output()` blocks | CLI stuck; no budget wall-clock enforcement |
| Child floods stdout/stderr | Both captured in memory | Memory exhaustion possible; raw store also unbounded |
| Child emits binary/invalid UTF-8 | Lossy UTF-8 rendering | Raw text passthrough is not byte-exact; raw blob still has original concatenated bytes if persisted |
| Blob store unavailable | Print decoded raw | Optimization degrades, but secrecy/fidelity caveats remain |
| Blob altered after write | `get()` reads by filename | No content-hash check; caller receives corrupted bytes |
| Edit process crashes mid-commit | Some renames already visible, no durable undo journal | Partial workspace update and stale lock |
| External editor modifies file after final hash check | Rename can overwrite changed target | TOCTOU lost update risk |
| One malformed text file in Atlas search tree | `collect()` returns error | Whole search fails rather than skipping/reporting file |
| Session log has malformed/truncated record | String parser skips missing keys | Accounting can silently undercount instead of surfacing corruption |
| Two processes write session log | Only per-instance Mutex exists | Cross-process serialization and clean record replay not guaranteed |
| Same target edited through two symlink aliases | Relative path dedup misses canonical identity | Silent overwrite of one operation by another |
| Task inside fanout hangs | Parent joins all worker threads | No cancellation or deadline; entire invocation can block |
| Pinned ledger content exceeds budget | `mask_to` cannot remove pinned content | Budget request can remain unmet without error |


### 4. Detailed findings and improvement opportunities

**Notation:** Critical = release/trust-boundary blocker or risk of corrupting protected data; High = substantial integrity, exposure, correctness or decision risk; Medium = meaningful but less immediate. Confidence measures evidence certainty, not the probability of exploitation. Effort: S (days), M (up to a few weeks), L (multi-week), XL (multi-milestone), subject to team capacity. ROI is an ordinal hypothesis awaiting empirical measurement. Impact considers technical, operational, security, reliability, scalability and decision-maker consequences where relevant.

#### 4A. Architecture and contracts

##### A-01. Advertised runtime controls are not orchestrated by the executable

- **Description / evidence:** The CLI imports `scope`, `store`, `lens`, `atlas` and `edit` (`src/main.rs:1-5`) but never invokes `relay`, `vault`, `guard`, `verify`, `ledger`, `memory` or `fanout`. Yet the architecture describes policy-before-execution, context budgeting, loop detection and impacted verification (`docs/wiki/architecture.md`). The Cargo manifest lists all 12 as dependencies (`Cargo.toml:11-23`), which can obscure the distinction between *compiled* and *active*.
- **Root cause:** Horizontal implementation of isolated primitives before a tested vertical orchestration path, with documentation prematurely generalizing library behavior.
- **Impact:** Engineering interfaces drift; security and budget assurances are inactive; operators cannot reason about end-to-end failures; business planning can mistake proof-of-concept modules for delivered features.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / L / High.**
- **Recommendation:** Define one `ExecutionContext` holding validated workspace, session ID, policy, budgets, cancellation, lineage and event sink. Execute a *single path* `preflight -> reserve -> spawn -> stream/capture -> persist -> classify -> publish -> reconcile -> audit`; provide a typed error/fallback table per stage. Keep optional modules behind feature/capability flags until their acceptance contracts pass.
- **Validation:** Integration tests assert enforcement and recorded state for allowed/denied commands, retries, budgets exhausted mid-execution, disconnected telemetry, and failed storage. Verify that no spawn path bypasses preflight.

##### A-02. Protobuf is aspirational, not a wire-contract source of truth

- **Description / evidence:** `proto/conduit.proto:1-9` declares `Session`, `ToolCall`, `ToolResult`, `Savings` and `Event` with sequence/timestamp fields. Runtime `EventLog` writes only a custom savings JSONL object with no session ID, sequence or timestamp (`crates/scope/src/lib.rs:3-29`). No build script, generated bindings, or serialization compatibility test is present.
- **Root cause:** A design schema was created before deciding whether IPC or structured interchange has consumers.
- **Impact:** False assurance of schema discipline, incompatible future clients, migrations requiring guesswork; duplicated contracts without an owner.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; Medium / High / M / High.**
- **Recommendation:** Either explicitly mark the proto as a future proposal, or generate and enforce serialized contracts with stable IDs, versioning, bounded fields, timestamps, input validation and golden compatibility fixtures. Do not add gRPC just to justify the schema.
- **Validation:** Encode/decode tests, unknown-field and old/new-reader compatibility fixtures, and a generated-code drift gate.

##### A-03. No agent-independent integration contract or compatibility matrix exists

- **Description / evidence:** There is no MCP server, agent hook, tool manifest, JSON tool schema, plugin adapter or provider integration. The user must call `conduit` explicitly (`README.md:34-48`). `run` assumes shell-like arguments but is not a shell or transparent stdio transport.
- **Root cause:** Optimization primitives have been prioritized over an adoption surface and negotiated capabilities.
- **Impact:** Users cannot deploy reliably across agents; semantically different tool outputs may be mutated by a generic reducer; broad portability claims remain untested.
- **Category / severity / confidence / effort / ROI:** **Transformational Opportunity; High / High / L / High.**
- **Industry reference:** RTK implements specialized coding-command adapters and agent initialization; MCP standardizes tool schemas and lifecycle. Port the **adapter/capability design**, not external benchmark percentages or unnecessary network authentication.
- **Recommendation:** Add a minimal stdio/MCP or agent-hook adapter with explicit capabilities, structured `raw_ref`, MIME/content type, provenance, error, and `presentation_only` semantics. Keep child stdout byte-exact and place compact agent-facing output in a separate channel when callers need machine-readable output. Publish a command/agent/OS compatibility matrix.
- **Validation:** End-to-end tool executions on at least two independent agents/clients, contract conformance, raw re-expansion, and no-op/rollback tests when an adapter is disabled.

#### 4B. Engineering correctness and test quality

##### E-01. A malformed Rust test block blocks reliable workspace verification

- **Description / evidence:** In `crates/scope/src/lib.rs:52-62`, a bare block begins at line 58 directly inside `mod tests`, rather than a Rust item such as `#[test] fn ...`. It appears to be a lost `totals` test function declaration. This is not a valid module-level item. The same source uses many unformatted one-line definitions; rustfmt compliance is separately unverified.
- **Root cause:** Source was changed without a succeeding Rust parse/format/test gate, or the distributed snapshot was assembled after the last passing checks.
- **Impact:** Fails compilation of the test module and undermines CI readiness; advertised validation is not reproducible from the archive.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; Critical / High / S / High.**
- **Recommendation:** Convert the bare block into a named `#[test] fn sums_typed_events()` and run formatter, workspace tests, all-target Clippy and dependency checks before any further refactoring. Block merge if these checks do not pass. Record binary/test outputs with commit and toolchain versions.
- **Validation:** `cargo fmt --all -- --check`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`; capture actual runner logs. The present assessment cannot assert they pass because Cargo is absent.

##### E-02. Tests cover toy functions, not adversarial interfaces or integrated recovery

- **Description / evidence:** Twenty-five `#[test]` annotations exist across 12 crates; no integration test directory, property/fuzz corpus, process-level test or crash-injection suite is supplied. Representative tests validate a single edit/stale hash, two short lens cases, basic file search, simple fanout order and similar happy-path contracts.
- **Root cause:** Tests were written around local API shapes rather than a system-level failure model. The requested concurrency, path, replay and durability guarantees lack destructive/fault-state testing.
- **Impact:** Regressions are likely around filesystem races, cross-process mutation, non-UTF-8 data, partial output and platform differences; gate confidence is disproportionate to evidence.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Recommendation:** Add process-level CLI golden tests, property-based tests for path normalization and reduction invariants, concurrency stress, signal and PTY fixtures, disk-full/permission fault injection, deliberate process termination at every edit-commit phase, fuzzers for log parsing and arbitrary command output, and per-target tests on Windows/Linux/macOS. Separate test quality from coverage percentage.
- **Validation:** CI matrix with reproducible randomized seeds, instrumentation of filesystem operations, nightly stress/fuzz and weekly fault-injection reports; measure escaped defects.

##### E-03. Alias deduplication does not establish a unique edited object

- **Description / evidence:** `apply` deduplicates `relative` paths (`crates/edit/src/lib.rs:17-23`) but resolves each through `checked_path(...).canonicalize()` (`:69-72`). A file named `real.txt` and a symlink `alias.txt -> real.txt` are different relative paths but resolve to the **same canonical destination**. Each operation can independently stage a result from the same old digest; all pre-commit hash checks run before any rename (`:33-42`); the second rename then overwrites the first. Both `Applied` records can be returned although only the second result remains.
- **Root cause:** Identity checked at user input syntax level rather than at actual filesystem target identity; multi-file validation separated from mutations.
- **Impact:** Silent lost edit, incorrect audit result and false atomicity guarantee; concurrency need not be involved. The alias fixture was recreated at the filesystem level; compiled-Rust execution remains unverified.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; Critical / High / M / High.**
- **Recommendation:** Normalize and deduplicate the **canonical target** (and where supported stable file identity) before staging. Reject symlink aliases and hard-link ambiguity or explicitly model one target with a combined ordered edit. Recheck target identity immediately before committing.
- **Validation:** Two operations referencing a symlink and its target must fail before write (or apply as a single explicit edit), with every source file unchanged on rejection. Add directory rename and file-replacement race fixtures.

##### E-04. File mutation is not bound to its validated descriptor, and metadata is degraded

- **Description / evidence:** Hash checks use `fs::read(path)` then commit uses a separate `fs::rename(temp, path)` (`crates/edit/src/lib.rs:33-40`), leaving a check-to-rename window for external writers. `fs::set_permissions` copies only Rust permission attributes (`:24-28`); hard-link identity, ACLs, extended attributes, ownership, timestamp metadata and Windows-specific file attributes are not expressly preserved.
- **Root cause:** Path-based optimistic concurrency without descriptor-level safeguards or a coherent snapshot/transaction model.
- **Impact:** External edits can be lost; security descriptors or file identity can change; tooling that observes modified files can break. Risk grows on shared workspaces and platform-specific filesystems.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Recommendation:** Treat canonical file identity, expected bytes and metadata as preconditions; document limitations on unsupported filesystems. Use safe handle-based traversal/creation where available, commit against a versioned worktree/snapshot when multi-file integrity matters, and require an explicit conflict instead of overwriting a changed target. Preserve and verify essential metadata on each supported OS.
- **Validation:** Competing writer process mutates/replaces file between checks and renames, symlink-race tests, ACL/xattr/hard-link regression fixtures, Windows/macOS/Linux platform checks.

##### E-05. Failure and state APIs are too weak for a composed runtime

- **Description / evidence:** Most primitive APIs return `io::Error`, `Allow/Reject`, free-text output, `Option`, or `String` (`relay`, `guard`, `fanout`, `scope`). A worker panic is serialized as the ordinary successful-looking text `worker panicked` (`crates/fanout/src/lib.rs:17`). CLI exits with generic error code 1 for many distinct errors (`src/main.rs:101`).
- **Root cause:** Local function prototypes were optimized for brevity rather than typed contract composition.
- **Impact:** Recovery automation and incident triage cannot reliably distinguish denied, unavailable, expired, stale, partial, canceled, corrupt and internal-error states; retries may be unsafe.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Medium / High / M / High.**
- **Recommendation:** Introduce typed, versioned error enums with retryability, partial-side-effect indicator, stage/correlation ID and user-safe explanation; separate `Result<Success, Failure>` from textual content. Add feature compatibility guards at the public CLI boundary.
- **Validation:** Failure-injection tests assert stable machine-readable codes, proper rollback or `recovery_required`, and no ambiguous success strings.

#### 4C. Security, privacy and trust boundaries

##### S-01. Untrusted command execution bypasses the policy module

- **Description / evidence:** `conduit run` invokes `Command::new(&command).args(...).output()` at `src/main.rs:24` with no `conduit_vault::authorize` call. The policy library exists only in `crates/vault/src/lib.rs`. No OS sandbox/profile is installed by Conduit in the supplied code.
- **Root cause:** A side library is presented as a safety capability without being mandatory at the execution boundary.
- **Impact:** A compromised or mistaken agent can run anything the invoking OS user can run if allowed to call the CLI; secrets, source, build tools and network may be reachable under that user's existing privileges. There is no evidence of a remotely reachable service, so this is principally a **local-agent privilege risk**.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / L / High.**
- **Industry reference:** MCP tool annotations help describe risk but are expressly not trusted policy enforcement; use actual authorization and isolation.
- **Recommendation:** Define execution authorities explicitly (trusted-user passthrough vs. sandboxed agent); require preflight policy and OS-level confinement for the latter, including executable identity, argv, cwd, env, writable roots, network privileges and resource limits. Keep passthrough as a clearly named opt-in mode, never silently call it restricted.
- **Validation:** Denied network/filesystem/command attempts must fail before side effects; use real process-level tests inside sandboxed and unsandboxed profiles.

##### S-02. The policy module itself is bypassable even if connected

- **Description / evidence:** `vault::authorize()` splits a raw string on whitespace, checks only the executable name and rejects literal `..`, `$(` and backticks (`crates/vault/src/lib.rs:6-11`). `ReadOnly` explicitly allows `git` and `cat`; `Dev` allows Python, Node, Cargo and npm. It neither parses argv nor confines programs. A permitted `git clean` mutates a repo, `cat /outside/path` reads beyond the workspace, and interpreters/build scripts can run arbitrary code. `cwd.starts_with(workspace)` is lexical and does not verify symlinks or the runtime cwd.
- **Root cause:** A string filter is used as a stand-in for a capability/security model.
- **Impact:** Read-only and confined profiles cannot be promised; arguments, subprocesses, plugins, shell semantics and symlinked directories defeat intended restrictions.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / L / High.**
- **Recommendation:** Change the API to typed program identity plus vector argv, canonical workspace descriptors, deny-by-default capability grants and a sandboxed process launch. Disallow broad interpreter/build-system operations under a read-only profile, or require isolated workspaces and separate permissions. Policy libraries should decide *intent* while the OS enforces effects.
- **Validation:** Execute adversarial cases for destructive git commands, interpreter `-c`, shell wrappers, external path reads, symlinks, inherited environment and spawned descendants. Assert no protected side effect.

##### S-03. Arbitrary session labels can escape the intended log directory

- **Description / evidence:** `session` is accepted directly from the CLI and embedded in `sessions/<session>.jsonl` for `record`, `replay` and `stats` (`src/main.rs:73-89`). `EventLog::open` makes parent directories without validating that they remain inside `CONDUIT_HOME` (`crates/scope/src/lib.rs:19-23`). A session string containing `../` path components can reference paths outside the session directory.
- **Root cause:** User-controlled identifiers are treated as filesystem paths.
- **Impact:** An agent able to supply session IDs can append to, read from, or create `*.jsonl` paths accessible to the invoking OS account outside the intended state directory. The filename suffix and OS permissions limit scope, but do not eliminate traversal.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; High / High / S / High.**
- **Recommendation:** Validate a narrow opaque session-ID grammar and length; separate display labels from storage IDs; canonicalize/securely create a fixed session directory; never construct arbitrary file paths from display text. Use descriptor-relative open with no-follow semantics where feasible.
- **Validation:** Property tests for `..`, separators, drive prefixes, symlinks, Unicode normalization and path length; all operations must be provably confined to state root.

##### S-04. Confidential output is collected with default filesystem permissions and unlimited retention

- **Description / evidence:** `BlobStore::open` uses `fs::create_dir_all` and `put` uses `fs::write` (`crates/store/src/lib.rs:10-30`), with no explicit private modes, ACL or owner check. `run` stores captured bytes before any redaction (`src/main.rs:24-29`). `docs/wiki/operations.md` admits retention/GC is planned. In an ordinary POSIX umask 022 environment, a reproduced default creation produced directory mode 0755 and file mode 0644; **actual deployed modes depend on OS, umask and parent ACLs**.
- **Root cause:** Content addressing and reversibility were implemented before an explicit data-protection model and lifecycle.
- **Impact:** Child logs may contain tokens, source snippets, personal data or proprietary build output; other local users may gain read access under permissive state-directory conditions, and sensitive blobs can accumulate indefinitely.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / M / High.**
- **Recommendation:** Create/check private state roots (0700/0600 semantics or equivalent Windows ACLs), reject symlinks/shared roots by default, isolate sessions/users/workspaces, encrypt at rest when threat model requires it, implement short TTL/quota/secure deletion policy, and expose redacted previews. Raw storage can be permitted only with explicit data classification and opt-in.
- **Validation:** Cross-user permission tests, ACL inspections on each OS, sensitive-fixture retention/GC tests and denial tests for shared or symlink state directories.

##### S-05. Atlas ignores common sensitive-file and generated-file boundaries

- **Description / evidence:** `collect` excludes just `.git`, `.conduit`, `target`, symlink entries, NUL-containing and >2 MiB files (`crates/atlas/src/lib.rs:37-52`). It does not interpret `.gitignore`, `.ignore`, hidden paths or secret-content rules. `find` prints matching source lines verbatim (`src/main.rs:45-50`). A local `.env`, private credential fixture or vendor directory can be indexed or exposed if text matches the query.
- **Root cause:** Homegrown unrestricted walker rather than a scoped search plan.
- **Impact:** Secrets exposed through agent tool responses; expensive irrelevant indexing; misleading results from excluded-by-Git files; brittle privacy expectations.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Industry reference:** ripgrep's `ignore` behavior honors `.gitignore`/`.ignore` and ignores hidden content by default; retain opt-in overrides for explicit forensic searches.
- **Recommendation:** Adopt an audited ignore-aware walker (or invoke ripgrep safely), sensitive-path defaults, symlink non-following, maximum total bytes/depth, diagnostics for skipped files, and allow-listed workspace roots. Add a privacy filter before returning results to an agent.
- **Validation:** Golden fixtures containing `.env`, ignored vendors, nested ignores, Unicode names, symlink trees, malformed UTF-8 and deliberate override permissions.

##### S-06. Prompt injection and data taint are acknowledged but not implemented

- **Description / evidence:** Threat model calls agent inputs/repository files/child output untrusted (`specs/threat-model.md:7-17`), but no source-origin tags, instruction/data separation, taint propagation, sensitive-result flag or user approval path exists in the runtime. `lens` recognizes only string patterns (`crates/lens/src/lib.rs:52-58`); it cannot distinguish malicious instructions embedded in an error log from authoritative tool output.
- **Root cause:** Trust boundaries are documented at conceptual level, not represented in serialized messages or agent integration.
- **Impact:** If Conduit is wired to a coding agent, untrusted text can be elevated into model instructions or cause excessive expansion/storage, irrespective of hashes.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Recommendation:** Tag all content with origin, trust level, data type, byte length and transformation provenance; never allow tool-output content to grant authority. Provide structured display rather than instruction-like formatting, a safe expansion UX, and explicit approvals for privileged actions.
- **Validation:** Adversarial repository/log fixtures, authority-escalation tests, secret exfiltration attempts within a sandbox, and provenance round-trip checks.


#### 4D. Reliability, durability and failure recovery

##### R-01. Multi-file edit transactions are not crash-atomic and can deadlock permanently on a stale lock

- **Description / evidence:** `apply` commits staged paths sequentially and holds only old bytes in process memory (`crates/edit/src/lib.rs:32-42`). If a process dies after the first rename, previous filenames remain modified, later ones are not, and there is no journal or startup recovery. The lock file is made with `create_new` and unconditionally removed only in `Drop` (`:86-88`); process crashes can leave it behind and prevent all future cooperating edits. Rollback error handling deliberately discards its own I/O errors (`:77-83`).
- **Root cause:** A best-effort rename batch is characterized as a transaction without durable write-ahead state, commit manifest or failure-independent atomic publication.
- **Impact:** Partial repository changes, ambiguous agent state and manual operator intervention. Claims of all-or-nothing edits fail under crash, I/O failure during rollback or power loss.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / L / High.**
- **Industry reference:** SQLite transaction/journal design illustrates durability and recovery mechanics; filesystems do **not** natively provide atomic rename of an arbitrary set of independent files.
- **Recommendation:** State the guarantee precisely. Short term, fail all operations before commit if canonical targets collide, persist a versioned transaction manifest and undo/redo images with `fsync` where supported, and reconcile unfinished transactions before new edits. For strong all-or-nothing visibility, stage a complete isolated worktree/snapshot and publish one atomic pointer/swap to a new revision instead of renaming N independent source files.
- **Validation:** Kill at every boundary before/after temp write, file sync, manifest sync, first/last rename, directory sync and rollback; restart and assert either the old complete revision or an explicitly recoverable new one. Test that an orphaned lock is safely recoverable without allowing two writers.

##### R-02. Blob integrity and lookup semantics do not enforce the content-addressed invariant

- **Description / evidence:** `put` derives a full BLAKE3 path, but if the path already exists it does not verify its contents (`crates/store/src/lib.rs:16-31`). `get` scans filenames, uses `handle_for_hash(name)` on **every filename** and returns file bytes without rehashing (`:34-47`). `handle_for_hash` only consumes the first 24 hexadecimal characters of a name (`:52-58`). This also means an in-flight temporary file named `<fullhash>.tmp-PID-NONCE` shares a handle with the final `<fullhash>`: an expansion concurrent with a put can see incomplete staged contents or reject an otherwise valid handle as ambiguous. This conclusion follows directly from the naming and lookup rules; a live concurrency test is still required.
- **Root cause:** A short display handle is treated as filesystem identity; temporary/invalid filenames are not excluded; digest authentication exists only on write.
- **Impact:** Corrupt or partially written output can be presented as valid; concurrent stores may temporarily break expansion; unexpected files/symlinks in the store increase exposure; future incidents can misdiagnose failed handle resolution as eviction.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / M / High.**
- **Recommendation:** Map handles to canonical complete 64-hex digests in an indexed metadata table; expose only committed entries; filter temporary paths; reject symlinks; verify full BLAKE3 digest on every retrieved blob; quarantine mismatches; cap output length. During put, verify an existing blob or replace it safely after conflict handling.
- **Validation:** Corrupt a stored blob and require a distinct integrity failure; concurrently put/expand the same hash under a barrier and require exact bytes, never partial or ambiguous; fuzz filename/path construction; verify restart recovery after interrupted put.

##### R-03. The CLI does not transparently preserve stdout, stderr, binary data or signal semantics

- **Description / evidence:** `Command::output()` captures the two pipes separately, then the CLI concatenates `stdout` and `stderr` (`src/main.rs:24-26`). Output is subsequently converted with `String::from_utf8_lossy()` (`:27,71`), so non-UTF-8 bytes cannot be emitted intact via CLI, including `expand`. Display metadata is added to stdout when reduced (`:35-36`), potentially breaking parsers. `status.code().unwrap_or(1)` does not preserve a Unix signal termination (`:43`).
- **Root cause:** Text presentation has been conflated with subprocess transport fidelity.
- **Impact:** Tool results may be misparsed, temporal cause-and-effect of failures disappears, structured output is corrupted, interactive/binary workflows fail, agents receive misleading diagnostics.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Recommendation:** Distinguish **machine transport** (exact byte streams, exit reason, signal, offsets) from **agent presentation** (optional compact text). Stream with bounded capture and multiplexed timestamps; preserve separate stdout/stderr byte buffers, and carry raw refs in metadata or side channel rather than adding text to machine stdout. `expand` should support a binary-exact mode. For interactive work, support inherited stdin or optional PTY with documented tradeoffs.
- **Validation:** Golden byte-for-byte subprocess fixtures covering UTF-8/invalid bytes/NUL, interleaved stdout/stderr, large writes, broken pipes, interactive stdin, ANSI sequences and signal termination on each OS.

##### R-04. Child process lifetime and capture resources are unbounded

- **Description / evidence:** There is no deadline, cancellation token, process-group termination, max output size, stdin strategy, disk quota or graceful downgrade around `Command::output()` (`src/main.rs:24-43`). Memory cost scales with captured output, then further copies are made during decoding and lens selection.
- **Root cause:** Synchronous process capture intended for a minimal demonstration, not a production agent-execution mediator.
- **Impact:** A single hung, chatty or adversarial child can stall the agent, exhaust memory/disk, block orchestration, or leave descendants running.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Critical / High / L / High.**
- **Recommendation:** Implement bounded streaming to private spool, backpressure, per-command wall/CPU/memory/output quotas, cancellation/kill-tree and explicit truncation/expansion semantics. Keep a bypass/passthrough strategy for normal trusted commands. No timeout should be described as a hard budget unless it is actually enforced on the process tree.
- **Validation:** Hanging child, never-ending output, forked descendants, full filesystem, blocked downstream consumer and cancellation race tests; bound RSS, elapsed time and state growth.

##### R-05. Session logging is neither cross-process-serialized nor self-validating

- **Description / evidence:** `EventLog` has a Mutex owned by one instance (`crates/scope/src/lib.rs:16-29`), but different CLI processes open independent handles to the same JSONL file. `write_all()` plus sync does not create a documented cross-process transaction. The serializer escapes only backslash, quotation mark and LF, not all JSON control characters (`:50`); a tab or carriage return in `module` yields an invalid JSON string. `totals` searches substrings and silently skips lines lacking numeric keys (`:31-49`), rather than strictly parsing event type/version and surfacing corruption.
- **Root cause:** Hand-rolled JSON serialization and per-object synchronization, without a format validator, lock protocol or append-record recovery format.
- **Impact:** Invalid/dropped accounting, inaccurate savings, hard-to-debug interrupted writes and unreliable audit logs. A small static reproduction of the serializer yielded JSON parse errors for embedded tab/CR characters.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Recommendation:** Use a proven JSON serializer immediately. Add length, sequence, schema version, event ID, parent command ID, monotonic timing and parser validation. For multi-process use, serialize via one writer or transactional SQLite; make incomplete-tail recovery explicit. Use checksums or signed chains only if tamper evidence is an actual requirement.
- **Validation:** Two independent writers under stress, killed appends, malformed text and control characters, duplicate/replayed events, schema migration and intentionally altered records.

##### R-06. Budgets, loop controls and safe test planning are not end-to-end guarantees

- **Description / evidence:** `relay::authorize()` uses `saturating_add` then `>` against a bound (`crates/relay/src/lib.rs:8-9`); if both the budget and used value equal `u64::MAX`, adding a positive requested value saturates at the allowed maximum and returns `Allow`, rather than rejecting nonzero consumption. More importantly, this is a pure function with no atomic reservation or CLI invocation. `guard` can halt locally but is not wired to scheduling, and `verify` is not connected to changed-file events.
- **Root cause:** Safety checks modeled as independent decisions, with no shared resource accounting or scheduler consistency contract.
- **Impact:** Concurrent requests could oversubscribe a limit if callers check separately; false confidence in hard ceilings and loop stops. The integer example is an edge-case invariant failure, not evidence of common overflow in production.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Recommendation:** Use atomic `reserve -> consume -> reconcile/refund` with `requested <= budget - used` after verifying `used <= budget`; enforce runtime CPU/wall/storage as separate OS controls. Give guard explicit scope/window/progress semantics, and make test-selection widening signal unsafe coverage uncertainty rather than claim completeness.
- **Validation:** Two simultaneous reservations at final available budget, integer-boundary/property tests, cancellation/refund, sustained-failure loops and mandatory full-test fallbacks.

#### 4E. Performance and scalability

##### P-01. Atlas builds an O(repository-size) snapshot for every query

- **Description / evidence:** `Snapshot::build` recursively reads all eligible files into memory, stores file content and hash in a vector, then `search` scans every line until a match/limit (`crates/atlas/src/lib.rs:10-52`). The size bound applies to **each file**, not the total tree. A limit on returned hits does not bound index construction time, nor does it bound source-line length in results.
- **Root cause:** A deterministic in-memory snapshot was built before a workload, file-watcher or incremental indexing contract.
- **Impact:** Slow search on monorepos, large RSS, work repeated for adjacent queries, possible process termination on giant trees, unacceptable agent latency.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Industry reference:** ripgrep for ignored-file-aware streaming literal/regex search; Tree-sitter for *selectively* cached incremental syntax trees; LSP/SCIP only where semantic navigation demonstrably improves target workloads.
- **Recommendation:** First replace whole-tree eager memory retention with bounded streaming `ignore` traversal and reusable file hashes. Add per-workspace index caches keyed by content identity, path and config; invalidate from filesystem events plus periodic reconciliation. Make advanced AST/semantic indexing opt-in for queries requiring it.
- **Validation:** Warm/cold query latency p50/p95, CPU/peak RSS/total disk and fidelity on small, medium and large repositories; incremental-change amortized cost.

##### P-02. Blob expansion is O(total blob files), not O(1) indexed lookup

- **Description / evidence:** `BlobStore::get` enumerates each directory and file and compares a derived handle (`crates/store/src/lib.rs:38-47`). Storage cost grows with history/retention and even invalid/temp entries.
- **Root cause:** Compact handle derived from a hash without a handle-to-hash index or canonical lookup path.
- **Impact:** Expansions get slower precisely as the store accumulates data; high filesystem metadata traffic; races with temp-file creation.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; Medium / High / M / High.**
- **Recommendation:** Maintain atomic handle metadata (full hash, byte count, TTL, owner, state) and address full hash directly; use a compact index on disk rather than directory discovery. Include handle namespace and version to avoid future decoding ambiguity.
- **Validation:** Benchmark lookup from 10 to >1 million committed blob records where practical, plus concurrent GC, add and expand correctness. Require near-constant indexed lookup time relative to blob count.

##### P-03. The lens is heuristic, can violate its configured budget, and lacks quality controls

- **Description / evidence:** `token_count` is whitespace splitting (`crates/lens/src/lib.rs:58`), not any particular model tokenizer. Every unique line containing signal substrings is inserted without testing budget (`:18-23`); a test explicitly requires exceeding a budget of one (`:72-78`). `run` hardcodes 700, cannot negotiate an agent context window (`src/main.rs:29`). Broad substrings like `error`/`warning` can retain huge irrelevant blocks or omit useful non-signal context.
- **Root cause:** Fixed recall-first text filter without downstream tokenizer, structural parser, evaluation dataset or uncertainty/fallback policy.
- **Impact:** Context budget exceedances, truncation farther downstream, loss of cause/context and unproven improvement in correct agent actions.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Industry reference:** RTK uses per-command structured reduction for build/test/git outputs; use generic filtering only as a safe fallback.
- **Recommendation:** Split hard-limit enforcement from answer-preservation policy. Add tokenizer profiles, severity-aware and command-aware parsers, bounded diagnostic groups with expansion handles, truncation reason, and a rule that limits may be exceeded only under an explicitly reported `needs_more_budget` condition. Run agent task non-inferiority before enabling aggressive reduction by default.
- **Validation:** Golden retained-evidence corpus with expected decisions, budget property tests, tokenizers for supported agents, quality/false-drop measurements and downstream task-failure regression gates.

##### P-04. Fanout is ordered but not bounded in queued work, output or duration

- **Description / evidence:** `workers` bounds thread count but `mpsc::channel` is unbounded for both inputs and results (`crates/fanout/src/lib.rs:7-24`). Parent queues all tasks, waits for all threads, and only then reads results; a hanging worker blocks completion indefinitely. Worker panics become ordinary text. A huge caller-specified worker count can still attempt excessive threads up to `tasks.len()`.
- **Root cause:** Bounded parallelism was equated with a bounded system.
- **Impact:** Memory and scheduling pressure, no backpressure, no partial-progress reporting, expensive tail-latency coupling and poor failure isolation.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Medium / High / M / High.**
- **Recommendation:** Add global per-profile concurrency limits, bounded channels, a results-draining coordinator, request deadlines/cancellation, typed outcomes and partial results. Adaptive worker sizing should be driven by measured CPU/I/O behavior and fairness rules, not fixed guesses.
- **Validation:** Slow/hung/panicking tasks, 100k small tasks, resource caps, cancellation, deterministic reorder/partial-result semantics and scheduler fairness.


#### 4F. Operations, observability and delivery

##### O-01. Accounting is manually supplied and is not end-to-end evidence of achieved savings

- **Description / evidence:** `conduit run` does not append scope events (`src/main.rs:20-44`). Users can separately type `record SESSION MODULE RAW_TOKENS PRESENTED_TOKENS`; this always sets `measured: false` (`:73-80`). `stats` sums the provided counters and labels their difference `saved_tokens` (`:86-90`), without establishing model context, billed tokens, cache reuse, output quality or source event lineage.
- **Root cause:** Session accounting exists as a demonstration ledger rather than a trace of actual runtime decisions.
- **Impact:** Dashboards would report arbitrary manually entered numbers as operational savings; business decisions could be based on misleading data; failures cannot be traced to raw output or reducer version.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Recommendation:** Every command/tool event should carry unique session/call ID, argv fingerprint (without secrets), policy/version, timestamps, raw and presented blob refs, exact byte lengths, tokenizer ID and token counts, reductions, dropped sections, cost inputs, exit status, safety/fallback reason and expansion activity. Derive all aggregation from these immutable measured events, not manual counters; keep user-entered records separate.
- **Validation:** Replay actual controlled commands and reconcile event counters byte-for-byte to saved content and parser output; reject missing or mixed token accounting bases.

##### O-02. No data lifecycle, recovery tools or operational limits are implemented

- **Description / evidence:** The docs state GC and retention are future work (`docs/wiki/operations.md`). Store has only `open`, `put`, `get` (`crates/store/src/lib.rs:9-49`); scope has only append, totals, read. No state version/migration, compaction, backup verification, integrity sweep, encryption key rotation, quota, disk watermark, repair/quarantine or operational metrics are present.
- **Root cause:** The first architecture milestone prioritized reversibility but not operational ownership of accumulated state.
- **Impact:** Runaway disk consumption, indefinite raw-secret retention, painful upgrades and weak incident recoverability.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Recommendation:** Introduce a single lifecycle service with explicit retention policies, per-user/session/workspace quotas, access provenance, incremental GC, referenced/unreferenced blob reconciliation, storage health checks and `doctor`, `gc --dry-run`, `inspect`, `repair` commands. Define backup/restore as opt-in with documented sensitive-state implications.
- **Validation:** Long-run churn, millions of small records, expired-ref/active-ref edge cases, full disk, disk corruption, backup/restore and safe recovery fixtures; enforce data deletion deadlines.

##### O-03. Observability and incident operations are documentation, not runtime mechanisms

- **Description / evidence:** Only manual `stats`/`replay` surface basic aggregates/raw log strings. There are no named spans, stage-duration histograms, bounded error codes, event correlation, health indicators, retention alarms, progress feedback or audit export. `docs/wiki/operations.md` describes what operators should do but the CLI cannot produce enough consistent facts for those procedures.
- **Root cause:** No trace schema is wired into command, storage, edit and verification operations.
- **Impact:** Difficult to diagnose slow commands, missing handles, policy bypass, incorrect reductions, stale locks or disk leaks; reliability claims remain unobservable.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Medium / High / M / High.**
- **Industry reference:** OpenTelemetry GenAI/tool span conventions offer a common vocabulary, while explicitly warning that inputs/results may be sensitive and should not be captured by default.
- **Recommendation:** Add *local* structured traces and low-cardinality metrics first, with a privacy-preserving correlation ID; support optional OpenTelemetry export only by explicit user choice. Collect latency per stage, pass-through rate, failed expansions, output-drop classification, crash-recovery events, attempted denied actions, index rebuild rate and storage growth. Redact arguments/outputs in default diagnostics.
- **Validation:** Incident drills reconstruct a full tool call and fault from metadata without accessing raw confidential content; telemetry disabled/offline mode remains fully functional.

##### O-04. Reproducibility, dependency assurance and release packaging are incomplete

- **Description / evidence:** The source archive contains no `Cargo.lock` and no LICENSE file. CI (`.github/workflows/checks.yml:23-49`) uses version tags such as `actions/checkout@v4` and `dtolnay/rust-toolchain@stable`, and runs formatting, test, Clippy, Python checks and cargo-deny. No artifact build/sign/release job, SBOM generation, reproducibility check, Rust pin/MSRV matrix, fuzz job or external audit attachment is present. Published outcome of remote CI is unavailable.
- **Root cause:** A source-check workflow exists, but software supply chain and distribution are not yet owned end-to-end.
- **Impact:** Unpinned dependency resolution, action tag mutation risk, unclear release provenance and lack of verifiable install/recovery assets; high operational burden for adopters.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Industry reference:** GitHub Actions security guidance recommends immutable full-length action SHAs; SLSA provenance and GitHub artifact attestations provide verifiable origin.
- **Recommendation:** Commit `Cargo.lock` for the CLI workspace, choose documented Rust version/MSRV, pin external actions to audited immutable SHAs, include actual Apache-2.0 LICENSE text if distributing as Apache-2.0, and add multi-platform release binaries with checksums, SBOM, attestations, signed provenance, reproducibility notes and rollback releases. Establish contribution/security ownership and branch protection.
- **Validation:** Clean-checkout deterministic build from pinned inputs, dependency/license/advisory checks, downloadable artifact signatures and verification on all supported targets.

#### 4G. Product, developer experience and feature leverage

##### X-01. Published benchmark and actual reducer have incompatible definitions of success

- **Description / evidence:** `python3 eval/benchmark.py` on `sample.log` reports `raw_tokens=24`, `presented_tokens=9`, **62.5% corpus reduction**. This exact output was reproduced in the audit. Python uses its own regex reducer (`eval/benchmark.py:8-16`), while `conduit run` invokes `distill` with a 700-word budget (`src/main.rs:29`); the Rust reducer passes raw unchanged whenever its raw whitespace count is <= budget (`crates/lens/src/lib.rs:11-16`). Therefore, for this same six-line, 24-word sample, the production `run` logic would apply **zero reduction**. This is a deterministic source-level inference, not a compiled CLI run.
- **Root cause:** Benchmark logic was forked instead of exercising the shipped engine and its runtime configuration.
- **Impact:** Regression protection is illusory; claimed savings are not comparable; optimizing the Python script can make reports improve while the production reducer does not change.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / M / High.**
- **Recommendation:** Remove duplicated reducer logic: benchmarks should invoke the built CLI/lens as a library or deterministic subprocess, record config/binary hash, tokenizer and inputs, and compare bytes from the same underlying implementation. Add a threshold corpus large enough to exercise reduction; assert preserved errors, provenance and exact expansion.
- **Validation:** Unit fixture parity for Python/Rust paths; a deliberate regression in Rust must fail the benchmark. Output quality and task completion must be tracked separately from text-length reduction.

##### X-02. The CLI lacks safe progressive disclosure and structured automation contracts

- **Description / evidence:** CLI accepts a single search string, one-file edit with `OLD` and `NEW` positional arguments, raw argument-based `store`, and plain-text stdout (`src/main.rs:45-72`). There is no `--help` subcommand beyond generic usage, JSON output format, completion, dry-run edit diff, session list, token profile, privacy choice, expiry inspection or human-readable error taxonomy. Passing secrets as CLI args is risky and the guide warns against it (`docs/wiki/cli-reference.md:38`).
- **Root cause:** Low-level library prototypes were exposed directly without an operator/agent workflow contract.
- **Impact:** Adoption friction, quoting limitations for multiline changes, invisible side effects, accidental disclosure via process listings, fragile automation and difficult support.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Medium / High / M / High.**
- **Recommendation:** Add standard `--help`, `--version`, `--json`, `--dry-run`, `--workspace`, `--profile`, `--session`, stdin/file input, explicit UTF-8/binary mode and nonzero standardized exit reasons. Make edits preview-able with expected file count, diff, canonical paths, hashes and rollback mode. Prioritize CLI/TUI workflow polish before building an unrelated dashboard.
- **Validation:** Usability tasks for first install, search->edit->verify, failure recovery and expansion; automation compatibility tests with spaces/newlines/binary data and secure logging.

##### X-03. Impact-based testing may underselect because exact file membership is not dependency impact

- **Description / evidence:** `verify::impacted` selects tests listing changed paths in their `Test.files` set and widens to all listed tests only when **some changed file is absent from the selected union** (`crates/verify/src/lib.rs:6-12`). It cannot account for tests indirectly affected via imports, runtime configuration, generated code, shared fixtures, ABI/protocol changes, build scripts or a test inventory that is itself stale. `tests=[]` with changed files yields `widened=true` but an empty result.
- **Root cause:** A manually supplied direct-file mapping is treated as a test impact oracle.
- **Impact:** False negatives in reduced test runs; regressions escape if this planner becomes a CI shortcut. A widened flag is not equivalent to actually running a full suite.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; High / High / L / High.**
- **Recommendation:** Build a trustworthy dependency/coverage graph with source revision and freshness; conservatively widen for unknown edges, cross-cutting config and schema changes. Always run a periodic and pre-release full suite, and never mark `widened` as verified without a runner execution receipt.
- **Validation:** Mutation tests, dependency-transitive fixture sets, random held-out changes and comparison of selected vs complete-suite detection rates; require high recall before enabling CI savings.

##### X-04. Ledger and memory lack provenance, lifecycle and semantic safeguards expected by consumers

- **Description / evidence:** `ledger::mask_to` changes an in-memory `State` but does not assemble an actual prompt or reveal whether pinned items leave the budget exceeded (`crates/ledger/src/lib.rs:12-35`). `State::Evicted` is defined but never assigned. The dedup key is only truncated content hash, not workspace/session/authority. `memory::learn/recall` trusts caller-supplied source bytes, and `invalidate_changed` compares **one** input source digest against all facts (`crates/memory/src/lib.rs:9-24`), so facts from different sources can be invalidated together. Both subsystems disappear on process exit.
- **Root cause:** Information identity is modeled as a byte digest without provenance graph, namespace, source revision or consumer-state contract.
- **Impact:** Wrong reuse across contexts, stale or missing knowledge, duplicate/conflicting facts, unenforced prompt budgets and no measurable effect on agents.
- **Category / severity / confidence / effort / ROI:** **High-Leverage Improvement; Medium / High / L / Medium.**
- **Recommendation:** Model a fact as `(namespace, source URI, content hash, source revision, extraction rule, confidence, created_at, expires_at)` with invalidation per source; prohibit arbitrary authority transfer from recalled text. Let ledger produce a verifiable context manifest, with explicit hard-budget error, pin policy, restoration callback and measured downstream task preservation. Keep semantic memory opt-in until actual value is demonstrated.
- **Validation:** Cross-workspace collision tests, multi-source invalidation, concurrent updates, restart/recovery and agent quality comparison against a no-memory baseline.

#### 4H. Governance, decision evidence and roadmap control

##### G-01. Release checks can pass without assessing implementation correctness

- **Description / evidence:** `eval/release_gate.py:5-13` verifies four files exist and that `sample.log` contains one of three signal substrings, then prints a metadata message. It never runs a build, evaluates artifact provenance or checks any numerical objective. `eval/benchmark.py` iterates `*.log` and exits successfully even if the corpus directory is empty: the audit reproduced return code 0 with only CSV header printed.
- **Root cause:** Input-sanity scripts have a release-gate name without fail-closed acceptance criteria.
- **Impact:** Green indicators without runtime confidence; poor acquisition/release decision evidence; silent loss of benchmark coverage.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; High / High / S / High.**
- **Recommendation:** Rename the current script to `validate_release_inputs.py` or strengthen it into an authoritative gate that checks nonempty pinned corpus, binary under test, SHA of fixtures/config, structural and measured metrics, required OS matrices, security checks, artifact existence and policy-approved exceptions. Aggregate proofs only from verified CI job outputs.
- **Validation:** Delete every required artifact/corpus fixture in an isolated test copy, break each check independently, and ensure release fails for each negative control.

##### G-02. Documentation maturity exceeds delivered guarantees and the source distribution is incomplete

- **Description / evidence:** `README.md:7-17` calls Phases 0-6 executable foundations and says cross-platform tests run in GitHub Actions; it explicitly notes Cargo was unavailable in the authors' local environment. Phase reports contain stronger claims about collision-safe blobs, crash-safe transactions and hard budgets than the code supports (`eval/reports/phases-0-6-audit.md`); more careful `docs/wiki/roadmap.md` still lists production gates as pending. The README refers to `LICENSE`, but no such file is in the supplied archive. There are no release version manifest, documented owners, maintainers' security contact, public changelog or CI result artifacts here.
- **Root cause:** Success criteria and implementation maturity were updated in narrative form without automated truth tests or a single controlled status ledger.
- **Impact:** Buyers/users misinterpret milestone status; engineering teams waste time reconciling claims; security and release obligations become ambiguous. Missing archive files must be checked against the upstream repository before concluding that all distributions omit them.
- **Category / severity / confidence / effort / ROI:** **Incremental Improvement; Medium / High / M / High.**
- **Recommendation:** Maintain an evidence-backed capability matrix with states `not started / isolated library / integrated / acceptance-tested / production-validated`; link each claim to implementation, executable contract tests, CI runs and required threat-model controls. Add explicit ownership, release SOP, licensing artifacts and pending-exception policy.
- **Validation:** Cross-check every documented current feature with an automated execution test and verify all user-facing links/files are present in a clean source/archive build.

##### G-03. Strategic economic value, reliability uplift and parity with peers are unmeasured

- **Description / evidence:** The only numeric report is the 24->9 whitespace-word fixture (`eval/reports/phase-0-baseline.md:5-11`). Docs explicitly state live-agent/RTK baselines are pending (`docs/phase-0-design.md:7`). There is no task dataset with answer-quality annotations, provider cache/cost accounting, per-adapter benchmarks, real agent integrations, operator toil numbers, incident evidence or ROI experiment.
- **Root cause:** No reproducible study links low-level text metrics to user tasks and business outcomes.
- **Impact:** Decisions about semantic indexes, model routing, daemons, parallelism and AI-assisted features lack an economic basis. Large infrastructure commitments may cost more than they save.
- **Category / severity / confidence / effort / ROI:** **Transformational Opportunity; High / High / L / Medium.**
- **Recommendation:** Define a paired, blinded trial of unchanged agent workflows versus Conduit-enabled workflows with pinned tasks/models/configs. Measure success/non-inferiority, billed total tokens (including caches), elapsed time, tool calls, retries, expansion demand, user correction rate, memory and privacy events. Perform stage-level ablations and refuse unsupported claims. Select major new modules only when experiments identify their bottleneck.
- **Validation:** Replicated A/B runs, confidence intervals and predefined gates; no external RTK numeric comparison without executing both paths on the identical authorized task set.


### 5. Competitive/parallel engineering reference assessment

The following comparisons focus on **mechanisms that can be adapted**, not claims of equivalent maturity or numerical performance. External project pages may change; all URLs below were retrieved on 2026-10-11. The original Conduit archive is the sole source for claims about Conduit's implementation.

| Relevant peer / authoritative design | What it actually provides | Adaptation with highest ROI | Caveat / non-goal |
|---|---|---|---|
| [RTK - Rust Token Killer](https://github.com/davahiatak1/rtk-rust-token-killer) | Command-aware compact presentation for Git, cargo/npm tests, search and other common developer commands; agent-specific command integration | Ship pluggable structured reducers; establish baseline and separate command-output savings from total token bills | RTK's self-reported savings and overhead are not independent evidence of Conduit savings; run identical workloads first |
| [ripgrep / ignore semantics](https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md) | Ignore-aware, symlink-conscious recursive search that avoids many hidden/generated files by default | Replace homegrown traversal with safe, scoped streaming search plus predictable ignore/override rules | Full semantic graph not supplied by grep; choose based on query type |
| [Tree-sitter](https://github.com/tree-sitter/tree-sitter) | Incremental, robust concrete syntax trees | Optional syntax-aware retrieval, signatures, anchors and edit guards without indexing all repositories by default | Grammar versioning/caches raise complexity; retain source-byte hashes as final authority |
| [SQLite ACID / WAL](https://www.sqlite.org/transactional.html), [WAL guide](https://www.sqlite.org/wal.html) | Explicit atomicity/recovery and concurrent readers/writers for metadata | Versioned session/event/blob-index and transaction-manifest metadata | SQLite transactions do not by themselves atomically change arbitrary source files; WAL is designed for single-host use, not network filesystems |
| [MCP tools/annotations](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2025-06-18/schema.mdx) | Typed tool interfaces, optional annotations indicating read-only/destructive/idempotent properties | Make `find`, `edit`, `expand`, diagnostics and command adapters discoverable with structured outcomes | Tool annotations are hints; **not** an authorization or sandbox security boundary |
| [OpenTelemetry GenAI conventions](https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-spans.md) | Consistent agent/tool spans and usage metadata | Local traces with optional opt-in export, per-stage performance and redacted audits | Default capture of prompts/tool args/outputs can leak sensitive content; do not enable by default |
| [GitHub Actions secure-use](https://docs.github.com/en/actions/reference/security/secure-use) and [artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations) | Immutable action SHA guidance and verifiable artifact provenance | Pin reviewed actions, produce attestations/SBOM and validate reproducible installed binaries | Attestations prove provenance of a build, not correctness of runtime security |
| [SLSA provenance](https://github.com/slsa-framework/slsa/blob/main/spec/build-provenance.md) | Standard representation of source/build inputs and artifacts | Trace source commit, dependency lock, CI builder and binary fingerprints | Adopt proportional controls; avoid heavy platform construction before a real release consumer exists |

#### Lessons from parallels

- **Do not equate output compression with agent outcome improvement.** A missing diagnostic line can cost many more tokens and tool calls than were saved; RTK-like adapters should be judged by non-inferiority, not output length alone.
- **Do not build an inferior generic index before exhausting focused search.** Baseline with ignored-file-aware literal/regex search; add language intelligence only to demonstrably difficult queries.
- **Do not treat a hash as a lock or a transaction.** A digest binds content identity; durability, atomic visibility, concurrency and secure path handling are separate problems.
- **Do not substitute annotations/allowlists for OS privileges.** Typed interfaces clarify risk, but an arbitrary interpreter, build system or version-control command can have expansive side effects.
- **Do not convert a local utility into a permanent service without a workload proving the need.** Idle overhead, boot failure, retained secrets and cross-version migrations can offset efficiency gains.

## 6. Complete peer-comparison and second-order convergence record

The following comparative analysis provides the full peer-by-peer decisions (both original batches), component winner conditions, architecture planes and 18 follow-on findings. References are internal paths within supplied original ZIPs. Where the historical text says “status in diff”, it refers to the first consolidated patch at the time of the initial peer assessment; the cumulative V3 status in Sections 1 and 3 supersedes those earlier status labels. The direct ZIP recount above supersedes older entry-count figures.

### 7. Consolidated source coverage, scope control, and assurance model

This section extends the baseline Conduit audit above with the two peer batches and the final remediation diff. **Interpret the baseline's source-line references as references to the original 51-file Conduit archive.** The modifications described in Section 12 are *proposed patch contents*, not deployed main-branch behavior. This distinction prevents a fixed-in-working-copy defect from being silently counted as a production fix.

#### 7.1 Scope ledger

- Original project: `conduit-main.zip`, 51 files.
- First peer batch retained: `homebrew-tap-main.zip`, `diffsitter-main.zip`, `ast-grep-main.zip`, `ast-bro-main.zip`, `RAG-Enterprise-main.zip`, `ai-jail-master.zip`, `codeseek-main.zip`, `grit-master.zip`, `pg_vectorize-main.zip`, `project-rag-main.zip`, `grep-main.zip`, and `ast-master.zip` (12 peers).
- Additional peer batch retained: `rtk-develop (1).zip`, `icm-main.zip`, `tinyhivemind-main.zip`, `ragu-main.zip`, `Graphite-master.zip`, `memvid-main.zip`, `iwe-master.zip`, `ragit-main.zip`, `hydradb-main.zip`, `tinyrsi-main.zip`, `ai-main.zip`, and `rtk-ldp-master.zip` (12 peers).
- Explicit exclusions: `default.zip` (statistical dataset) and `vox-main.zip` (voice-agent product). Both were excluded by instruction, not silently overlooked.
- Deduplicated: `rtk-develop (1)(1).zip` has the identical SHA-256 digest `494aca088ed22b148c1769d7ec984d538bb8a06795a4a5c3b77245cf62d3a595` as `rtk-develop (1).zip`; treating them as independent corroborating implementations would be wrong.

**Final distinct scope: 25 archives, 9,118 archive entries.** A machine-verifiable, per-entry pathname, byte-count, SHA-256 and available text-line inventory is in `conduit_convergence_source_manifest.json`. Mechanical inventory covers every included archive entry; semantic source inspection was prioritized by actual transferability, failure criticality and implementation path. **It would be false to claim every expression in 9,118 peer files was dynamically tested, fully understood, or proven superior.** This assessment provides a systematic mapping of the peer components and targeted method-level evidence instead.

#### 7.2 How to interpret peer evidence

An external project's README is evidence of *stated scope*, not proof that its implementation is safe. Code inspection is evidence that a mechanism exists, not proof of its production reliability. A passing unit test (when observed) would verify that assertion only for the tested inputs. Peer performance or cost marketing figures have not been adopted as Conduit projections. Source citations below use `archive / internal relative path : approximate source line`; the inventory contains exact source paths and content hashes.

The user's **MIT assumption** is used solely for architectural comparison and prioritization. It does not change actual upstream license statements. The patch reimplements small general engineering patterns using new source changes rather than wholesale copying of third-party packages; redistribute any directly copied upstream code only after independently confirming its actual terms.

#### 7.3 Project/module/folder inventory summary

| Archive | Files | Source/binary mix | Major modules |
|---|---:|---|---|
| `RAG-Enterprise-main.zip` | 122 | .rs:65, .md:21, (none):6, .js:4, .toml:3 | src:73, changelog.d:16, frontend:13, migrations:3, .cargo:1, .env.example:1, .gitattributes:1 |
| `ai-jail-master.zip` | 145 | .md:93, .rs:30, (none):6, .toml:3, .yml:2 | releases:74, src:22, docs:15, tests:12, .github:4, packaging:4, dist:3 |
| `ast-bro-main.zip` | 262 | .rs:150, .md:15, .go:14, .py:10, .mod:8 | src:117, tests:110, wiki:9, cli-python:5, cli-typescript:5, .github:2, Formula:2 |
| `ast-grep-main.zip` | 324 | .rs:158, .json:51, .md:26, .yml:23, .toml:18 | crates:246, schemas:28, npm:19, .github:12, xtask:3, fixtures:2, .cargo:1 |
| `ast-master.zip` | 30 | .rs:10, .pest:5, (none):3, .xml:3, .md:3 | derive:14, .idea:3, examples:3, .github:1, .gitignore:1, Cargo.toml:1, LICENSE-APACHE:1 |
| `codeseek-main.zip` | 149 | .rs:69, .json:16, .md:8, .ts:8, .decl_json:7 | rust-core:127, .github:4, docs:4, src:2, .gitignore:1, CLAUDE.md:1, CONTRIBUTING.md:1 |
| `conduit-main.zip` | 51 | .md:18, .toml:14, .rs:13, .py:2, .yml:1 | crates:24, docs:11, eval:8, .github:1, .gitignore:1, Cargo.toml:1, README.md:1 |
| `diffsitter-main.zip` | 148 | .rs:35, .snap:30, .md:22, .json:9, .toml:9 | tests:34, test_data:22, src:18, .claude:14, plugins:10, .github:7, resources:7 |
| `grep-main.zip` | 45 | .rs:17, .yml:7, .toml:6, .md:5, (none):3 | fuzz:13, .github:8, src:7, util:3, .cargo:1, .gitignore:1, .pre-commit-config.yaml:1 |
| `grit-master.zip` | 100 | .rs:21, .md:18, .sh:16, .ts:8, .json:7 | test-projects:37, docs:13, src:11, scripts:7, examples:6, tests:6, .github:5 |
| `homebrew-tap-main.zip` | 5 | .rb:4, .md:1 | Formula:4, README.md:1 |
| `pg_vectorize-main.zip` | 198 | .rs:69, .sql:45, .md:26, .py:17, (none):13 | extension:71, server:26, core:21, vector-serve:19, docs:18, bench:10, proxy:10 |
| `project-rag-main.zip` | 70 | .rs:54, .md:12, (none):2, .toml:2 | src:46, docs:8, examples:4, tests:2, .gitignore:1, CLAUDE.md:1, CONTRIBUTING.md:1 |
| `Graphite-master.zip` | 1125 | .rs:708, .toml:109, .md:56, .svelte:56, .ts:48 | editor:277, node-graph:240, frontend:130, libraries:112, desktop:106, website:95, tools:52 |
| `ai-main.zip` | 1050 | .rs:543, .yaml:199, .json:108, .md:106, .toml:22 | tests:365, apis:197, examples:107, docs:89, filters:85, xtask:58, .github:57 |
| `hydradb-main.zip` | 232 | .rs:153, .yaml:20, .sh:15, .md:11, .toml:9 | src:84, crates:63, charts:25, scripts:18, examples:13, .github:9, .dockerignore:1 |
| `icm-main.zip` | 318 | .rs:119, .md:35, .py:31, .js:30, .json:20 | crates:192, bench:60, docs:9, .github:8, scripts:8, deploy:6, assets:2 |
| `iwe-master.zip` | 467 | .rs:265, .md:85, .txt:61, .yaml:13, .json:10 | crates:367, docs:70, npm:15, .github:5, .gitattributes:1, .gitignore:1, .release-plz.toml:1 |
| `memvid-main.zip` | 217 | .rs:158, .md:29, (none):9, .yml:5, .toml:2 | src:137, docs:15, docker:12, tests:12, examples:10, .github:8, install:3 |
| `ragit-main.zip` | 502 | .rs:168, .json:91, .py:82, .md:49, .txt:40 | ragithub:205, tests:96, src:64, crates:56, docs:54, RelNotes:13, prompts:9 |
| `ragu-main.zip` | 756 | .rs:332, .lean:158, .md:132, (none):41, .toml:22 | qa:295, crates:292, book:97, .claude:26, .github:18, src:12, scripts:2 |
| `rtk-develop (1).zip` | 617 | .rs:190, .txt:153, .md:116, .toml:65, .sh:20 | src:235, tests:226, .claude:44, hooks:27, scripts:20, docs:19, .github:13 |
| `rtk-ldp-master.zip` | 263 | .astro:62, .csv:56, .md:34, .svg:18, .webp:16 | public:114, src:79, .claude:54, .github:2, docs:2, scripts:2, .gitignore:1 |
| `tinyhivemind-main.zip` | 1340 | .rs:632, .md:563, .json:44, .js:36, .toml:11 | crates:847, examples:315, docs:151, .github:9, .dockerignore:1, .env.example:1, .gitignore:1 |
| `tinyrsi-main.zip` | 582 | .rs:310, .md:123, .json:33, .py:33, .ts:29 | crates:352, docs:95, apps:50, examples:39, loops:12, .github:9, scripts:8 |

The `modules` column is a literal first-level path distribution, **not** a maturity rating. Subfolders, manifest files, tests, generated assets, migrations, and pipelines are retained in the hash inventory. For example, Graphite's 1,125 entries include a large graphics editor/UI and node-graph ecosystem that is not part of Conduit's current product, while `tinyhivemind` includes 632 Rust files and 563 Markdown files: a blind file-count-weighted import strategy would badly overvalue such unrelated content.

### 8. Peer-by-peer differential analysis — initial batch

| Peer | Direct source evidence and concrete transferable mechanism | Adoption decision | What not to import |
|---|---|---|---|
| **RTK Homebrew tap** | `Formula/rtk.rb`, `Formula/icm.rb`, `Formula/grit.rb`: platform release URLs, versions, artifact integrity and install instructions. | **Adopt release automation later (M effort)** after stable binaries and checksum provenance. | Do not create a tap that points at non-existent or unverified artifacts. |
| **diffsitter** | `src/diff.rs:798` (`myers_diff`), `src/parse.rs:158` (language grammar symbols), diff regression snapshots. Provides syntax-aware change presentation and fallback discipline. | **Adapt as optional structural diff adapter (M/L)** for review/edit validation, never as the only truth about changed bytes. | Full grammar bundling and parser lifecycle before demand; AST similarity is not byte or behavior equivalence. |
| **ast-grep** | `crates/outline/tests/python_go_outline_rules.rs:9-100` and language-specific outline tests show multi-language extraction; structured rewrite engine is a richer mechanism than plain text replace. | **High-value opt-in structural search/rewrite (L)** behind typed grammar contract and pristine source hashes. | Do not replace Conduit's safe plaintext edit path with pattern rewrites that lack diff preview, matching cardinality checks or rollback. |
| **ast-bro** | `src/context.rs`, `tests/context_e2e.rs:53,73,109`, `src/squeeze/mod.rs:90`, `src/squeeze/render.rs:70`: symbol context budgets, graceful signature-only fallback and log-squeeze schema. | **Adopt budgeted symbol outline / inspection and reversible formatting (M/L)**. | Never expose reduced logs without a valid raw retrieval path; output budgets are not correctness proofs. |
| **RAG Enterprise / I3K** | `src/extensions/retrieval.rs:15`, `src/backup/service.rs:27,511,593`, `src/rag/chunker.rs`, `src/auth/rbac.rs`: pluggable retrieval, backup verification, document ingestion, role boundaries. | **Inspire optional ingestion and restoration verification (L)** when multi-document use cases appear. | Entire account/server/admin/database UI surface is unnecessary for a trusted local CLI and adds major attack surface. |
| **ai-jail** | `tests/sandbox_escape.rs:268-319,382-413` explicitly probes seccomp and protected paths, while README warns against treating sandboxing as a hard-security guarantee. | **Adapt availability-gated OS containment as a distinct high-risk workstream (XL)** and refuse to claim protection without verifiable platform enforcement. | Portable fake sandboxing, argv allowlists marketed as confinement, or unverified support outside target kernels/OS versions. |
| **CodeSeek** | `rust-core/src/watcher.rs:213`, `rust-core/src/services/embedding_service.rs:114`, `rust-core/src/config.rs:29,84`: incremental index update, pluggable embeddings and reranking parameters. | **Incremental lexical index first (M), optional hybrid search later (L)**, with source-hash invalidation and no required network calls. | Default model downloads, remote embeddings, hidden inference costs, and provider-dependent source exposure. |
| **grit** | `src/db/azure_store.rs:154,187,285,334`: reservation and compare-before-lock patterns around concurrent agent edits; test projects under `test-projects/`. | **Adapt reservation/lease concepts for collaborative edits only after durable version identity exists (L)**. | Age-only stale lock deletion and distributed coordination without a clear single writer or fencing token; no guarantee from git worktrees alone. |
| **pg_vectorize** | `extension/src/search.rs:279`, `extension/tests/integration_tests.rs:54`, `core/src/cache.rs:101`: hybrid keyword/vector retrieval and refresh/index management in PostgreSQL. | **Conceptual benchmark for larger shared installations (L/XL)**, not a default dependency. | Postgres, distributed worker and vector-service complexity for an embedded single-user source-search workload. |
| **Project RAG** | `src/mcp_server.rs:39,79,111`, `tests/simple_integration.rs:67`, `benches/indexing_benchmark.rs:66`: a typed MCP-facing code-index workflow with integration tests. | **Adopt eventual tool-adapter contracts (M)** only after Conduit's CLI semantics are stabilized. | MCP transport as a substitute for access control, source freshness, or safe command execution. |
| **uutils grep (`grep-main`)** | `tests/test_grep.rs:1403-1501` exercises binary detection and recursive behavior; `:1191-1236` covers context-line delimiters and overlapping context. | **Adopt golden correctness matrices for `find`/`peek` (S/M)**; use mature glob/ignore and byte-handling semantics. | Full GNU-grep compatibility layer where a narrow coding-agent search interface is sufficient. |
| **pest-ast (`ast-master`)** | `derive/src/lib.rs:14`, `derive/src/from_pest/mod.rs:33`: typed AST construction from grammar parse trees. | **Use as a design reminder for typed internal syntax IR (M)** if custom grammar surfaces emerge. | Inventing a new grammar and derive-macro framework when Tree-sitter-backed tools meet the need. |

**Supersession rules:** RTK/ast-bro-style reduction is superior to Conduit's simple generic reducer for supported diagnostic formats *if* it preserves errors, exit status, raw retrieval and net savings. Structural AST parsing is superior to substring matching for language-aware symbol/rule queries, **not** for arbitrary text, broken files, generated text, byte-accurate diffs or unknown grammars. Durable storage is superior to ephemeral facts for restart continuity, **not** for every low-value tool-output line. These are workload- and invariant-dependent statements, not universal winner rankings.

### 9. Peer-by-peer differential analysis — additional batch

| Peer | Direct implementation anchor | Transferable capability | Decision, prerequisites, tradeoffs |
|---|---|---|---|
| **RTK v0.49 archive** | `src/core/guard.rs:21` (`never_worse`), `src/core/tee.rs:31,61`, `src/hooks/decision.rs:143`, command adapters under `src/cmds/`, and `tests/grep_faithful_format_test.rs`, `tests/stderr_only_failure_test.rs`, `tests/read_window_bytes_test.rs`. | Recognize command families; prevent compression regressions; preserve failure diagnostics; agent integration/hook consent; raw recall. | **S adopted in patch:** hint-aware net-savings threshold and raw-first expansion. **M/L later:** command-specific adapters with regression fixtures and opt-in agent hooks. Do not assume RTK's advertised savings are independently reproduced here. |
| **ICM** | `crates/icm-core/src/fusion.rs:87` (`rrf_fuse`), `crates/icm-core/src/context_snapshot.rs:115` (deterministic budgeted snapshot), `crates/icm-core/src/temporal.rs:85`, `crates/icm-store/src/` SQL backends. | Weighted reciprocal-rank fusion with stable tie-breaking; project-scoped memory; temporal retrieval; budgeted prompt prefix. | **M/L optional memory layer** after explicit source identities, ACL/retention and tokenizer measurement. RRF should only merge available results; a failed semantic index must not suppress lexical hits. |
| **tinyhivemind** | `crates/tinyhivemind-hives/src/storage/sqlite.rs:22,78` implements schema-versioned immediate-transaction state/append; `crates/tinyhivemind-hives/src/coordinator/scheduler.rs:42,149,363` schedules candidate work. | Version-checked durable coordination, append-only event rows and bounded decision-stage scheduling. | **L later:** use durable state/revision preconditions in fanout. Agent councils/quorum are not a substitute for deterministic verification and should not be deployed automatically for simple tool calls. |
| **ragu** | `README.md` identifies an unaudited Rust proof-carrying cryptography framework; `src/lib.rs` and `crates/` expose specialized recursive proof machinery. | Abstract concept of machine-verifiable assertions and evidence-carrying results. | **Concept only:** compile/test/digest attestations are appropriate; recursive SNARK circuits are disproportionate and expressly unaudited for this project's use case. Do not import cryptography merely for “proof” terminology. |
| **Graphite** | `document/graph-storage/src/history.rs:15,64,99,108` has append-only canonical delta DAG ordering; `document/graph-storage/src/delta.rs:17,196` computes deterministic changes; `editor/src/dispatcher.rs:132` dispatches typed messages. | Versioned change history, deterministic replay ordering, operation/event separation, undo-oriented interaction design. | **L future:** model edit batches as content-hash-based proposals and revisions. Avoid a heavyweight CRDT/editor rewrite when Conduit has no shared live document model or UI. |
| **Memvid v2** | `src/footer.rs:14,58,76` checks hashed commit footers and finds last valid generation; `src/replay/`, `src/search/tantivy/wal.rs`, `src/lockfile.rs:83,94`. | Crash-recoverable generations, validated checkpoints, optional hybrid retrieval and explicit lock lifecycle. | **M/L adapt:** integrity-checked state catalog and recovery manifest; never replace small simple blob storage with a large single-file memory engine without data volume / recovery evidence. |
| **IWE** | `crates/liwe/src/graph/index.rs:8,27,60,71` maintains a reference index and removes stale source edges; `crates/liwe/src/operations/rename.rs`, `crates/iwe/src/`. | Source-linked notes, backlinks, graph invalidation, projections and structured markdown transforms. | **M/L future:** source-linked decision memory and backlinks through an optional knowledge view; do not expand into a full notes editor as a prerequisite to reliable code execution. |
| **Ragit** | `src/index/file.rs:24-57` defines incremental `FileReaderImpl` with buffered tokens for large files; `src/index/`, `src/chunk/`, `crates/fs/src/`. | Staged ingestion, chunk-level provenance, large-file streaming and import/export of versioned knowledge. | **M/L:** reuse bounded ingestion contracts and source-hash manifests; default local search should remain fast when no embedding model or API key exists. |
| **HydraDB** | `src/shard/write_pipeline.rs:23,54`, `src/shard/xlog.rs:43,70`, `crates/telemetry/src/redact.rs:62,77`: coordinated writes, encoded change logs and field-level instrumentation redaction. | Backpressure, scoped data partitioning, redacted telemetry and explicit durability classes. | **S/M adapt telemetry field redaction; XL conditional** tiered/distributed state only at measured capacity limits. A graph database is not Conduit's missing foundational capability. |
| **TinyRSI** | `crates/tinyrsi/src/orchestrator/mod.rs:35-95` runs screen→selection→confirmation; `crates/tinyrsi/src/archive/replay.rs:110,174` validates archive events; approval decisions require a host gate. | Reproducible shadow evaluation, incumbent/candidate comparison, explicit human approval and rollback promotion discipline. | **M/L high leverage:** test optimization candidates in isolated workspaces with paired traces, non-inferiority thresholds and human promotion. Do not self-modify the primary source without deterministic evaluation and authorization. |
| **Praxis AI (`ai-main`)** | `server/src/pipelines.rs:29,72,253,369` resolves and validates filter pipelines; `server/src/readiness.rs:41,71`; `filters/src/token_usage/` and `filters/src/metering/`. | Composable request/response stages, provider boundary validation, cost metering and readiness that checks actual dependencies. | **L gateway design reference** only if Conduit introduces provider integration. The CLI cannot inherit gateway policy guarantees by importing filter names. |
| **RTK landing/docs (`rtk-ldp`)** | `scripts/prepare-docs.mjs`, `scripts/build-search-index.mjs`, `src/content.config.ts`, `package.json` build pipeline. | Generated searchable docs, install guidance, platform matrix and docs CI. | **S/M release UX:** generate CLI docs from the verified command surface, versioned examples, compatibility matrix and an installation journey; do not ship the Astro site or unrelated promotional pages. |

The two RTK archive names refer to the **same source bytes**. RTK-LDP is separately a documentation website, not another implementation of RTK's runtime. `ai-main` is Praxis AI, **not** a generic unspecified AI library; `ragu` is a cryptographic framework, **not** a code-RAG variant. These identity checks prevent category-based false equivalences.

### 10. Cross-peer primitive ranking and small-code “winner” matrix

The goal here is not “merge all the projects.” It is to identify smaller primitives that have **stronger correctness or capability properties under the specific Conduit workload** and either integrate them or preserve a concrete migration contract.

| Primitive / decision | Conduit baseline | Most applicable external mechanism | Final recommendation | Status in diff |
|---|---|---|---|---|
| Reduce command noise | Generic line signals plus trailing context | RTK's command-specific parsers and `never_worse`; ast-bro log squeezing | Preserve high-priority errors, benchmark whole output including recovery hints, then add specialized reducers behind golden tests | **Partially implemented**: conservative reducer and hint-aware guard |
| Store recoverable observations | BLAKE3 blob with unverified read | RTK raw tee/recall; Memvid checksummed generation | Full hash verification, staging, private permissions, no partial-handle lookup, explicit retention policy | **Partially implemented**: validation and atomic publish; GC outstanding |
| Spawn local command | Direct unbounded child output | RTK subprocess and retention tests; ai-jail OS boundary tests | Timeout/capture ceiling and informative overflow, plus platform supervisor/independent sandbox as next project | **Partially implemented**: bounded direct child, no OS isolation |
| Search repository | Eager recursive substring snapshot | uutils grep behavioral fixtures; CodeSeek incremental updates | Ignore-aware bounded lexical baseline first; optional incremental/AST layer only if needed | **Partially implemented**: ignore-aware bounded snapshot + `peek` |
| Navigate symbols | No syntax analysis | ast-bro signature fallback; ast-grep Tree-sitter matching | Small typed `symbols`, `references`, and `impact` interfaces, read-only initially | **Design only** |
| Review semantic changes | Byte/string replace | diffsitter AST diff; Graphite stable revision DAG | Show byte diff and optional AST diff, always retaining original bytes and hash | **Design only** |
| Modify files | Cooperative lock + best-effort staged replacement | Graphite versioned deltas; grit reservations | Crash-aware journal, checksummed manifest, recovery state machine; fenced write identity | **Partially implemented**: canonical target checks and OS-lifetime lock |
| Account for cost | Caller-supplied JSONL fields | Praxis token usage; ICM stable budgeted snapshots | Typed events distinguish measured/caller-supplied/estimated; signed savings and atomic reservations | **Partially implemented**: typed JSONL and signed net metric |
| Memory retrieval | Hash of one caller-supplied source | ICM weighted RRF; IWE backlinks; Ragit streaming | Source-scoped, revisioned, local lexical-first recall; optional dense retrieval | **Design only** |
| Long-running state | Flat files and ephemeral structs | Tinyhivemind immediate SQLite revision checks; Memvid commit footers | Choose SQLite transaction metadata **or** a checksummed append-only manifest, not both by default | **Design only** |
| Parallel agents | Unbounded channel fanout | TinyRSI screened promotion; tinyhivemind durable coordinator; grit reserved edits | Bounded queue, durable task IDs, cancellation, fencing, private shadow evaluations | **Design only** |
| Instrumentation | Manual totals, no production tracer | HydraDB redaction, Praxis readiness, ICM benchmark reporting | Local opt-in structured events with source/secret exclusion and measured overhead | **Design only** |
| Releases/docs | Build/test workflow, incomplete distribution | Homebrew formula; RTK docs generation | Reproducible binaries, pinned actions, signed checksums, install smoke, generated CLI reference | **Docs updated; release pipeline remains** |

#### 10.1 Hidden non-equivalences that matter

**Generic reduction vs command-specific reduction.** A simplistic truncation that makes stdout shorter is not necessarily more informative. Retain structured failure summaries, preserve status, compare net hint-inclusive output size, and test whether an agent still arrives at the same correct diagnosis. RTK includes regression tests for failure-only stderr and faithful grep/diff formatting that Conduit lacks. The patch therefore prioritizes safety/reversibility over maximal reduction.

**Content hashes vs persistent knowledge.** BLAKE3 hashes identify exact bytes; they do not say that a generated statement is true, that a source is trustworthy, or that the user is authorized. ICM's RRF and IWE's backlinks are ranking and navigation primitives, not cryptographic proof of semantic correctness. Long-term memory must carry source, scope, observation time, version, expiry, and conflict resolution. Different memories derived from independent files must not be invalidated by comparing all of them to one global source hash.

**Single-file atomic rename vs batch transactions.** An atomic rename on one filesystem does not create a crash-atomic transaction across files. Graphite's revision DAG and Tinyhivemind's SQLite `IMMEDIATE` transactions solve different problems; Conduit should implement **one well-specified journal or metadata transaction** before promising all-or-nothing edits.

**Structured policies vs isolation.** A typed command policy stops accidental use of a disallowed command only if every execution path passes through it, and even then it cannot prevent all filesystem or network access by the allowed executable. ai-jail's own source/tests are explicitly platform-dependent. Conduit must never call its `vault` function an OS sandbox.

**Semantic index vs lexical fallback.** Optional embeddings can introduce model installs, off-device data transfer, inference latency, vector-index drift and mismatched embedding dimensions. A valid hybrid index is additive: lexical matches must survive provider/vector outages. A non-queryable semantic index must be reported degraded rather than returning an empty, falsely authoritative result.

**More agents vs verified work.** Distributed agent mechanisms can improve task decomposition, but do not prove higher correctness or productivity. Extra workers expand scheduling, cancellation, reproducibility, context and write-conflict complexity. Tinyrsi's private candidate evaluation and explicit promotion gate have clearer near-term value for this project than a full hive/coordinator architecture.


### 11. Second-order convergence: higher-level system planes

The first-order pass finds useful isolated primitives. The second-order pass asks which **new coherent system capabilities** emerge when those primitives are composed, and whether that composition is justified by a user-facing task, measurable performance or a correctness obligation. The following is a **proposed architecture**, not a claim that these planes already exist.

```text
                    ┌─────────────────────────────────────┐
                    │ CLI / agent adapters / UI inspector  │
                    └────────────────┬────────────────────┘
                                     │ typed request, capability, budget
                    ┌────────────────▼────────────────────┐
                    │ Non-bypassable authorization gate     │
                    │ policy, user consent, workspace scope │
                    └────────────────┬────────────────────┘
                                     │ authorized action id
                    ┌────────────────▼────────────────────┐
                    │ Execution & observation plane         │
                    │ process tree | deadline | output cap  │
                    │ raw spool | complete/error terminal   │
                    └──────┬─────────────────────┬─────────┘
                           │                     │
              ┌────────────▼──────────┐ ┌────────▼──────────────┐
              │ Durable evidence       │ │ Presentation adapters │
              │ content hash, ACL,     │ │ command-specific,     │
              │ source, lifecycle      │ │ reversible, budgets   │
              └────────────┬──────────┘ └────────┬──────────────┘
                           │                     │
                    ┌──────▼─────────────────────▼────────┐
                    │ Search / code intelligence plane    │
                    │ lexical -> AST -> optional semantic │
                    │ source/version/capability provenance │
                    └──────┬─────────────────────┬────────┘
                           │                     │
                ┌──────────▼─────────┐ ┌────────▼───────────────┐
                │ Edit / recovery     │ │ Verification / outcome │
                │ revisions, journal │ │ tests, task success,   │
                │ compare-and-swap    │ │ measured token usage   │
                └──────────┬─────────┘ └────────┬───────────────┘
                           │                     │
                     ┌─────▼─────────────────────▼───────┐
                     │ Optional coordination/memory      │
                     │ only after each simpler plane     │
                     │ has independently met its gates   │
                     └───────────────────────────────────┘
```

#### 11.1 Plane A: evidence-preserving execution broker — near-term, high ROI

**Composition:** bounded subprocess capture (Conduit patch), RTK-style contextual output adapters, integrity-checked store, strict event types, optional OS sandbox as separate platform capability. Every invocation gets a stable request ID, command spec, execution principal, workspace root, output-completeness flag, start/end timestamps and raw/delivered digest references. Cancellation/deadline and resource reservations are explicit terminal outcomes, not implicit “some output arrived” success.

**Safety invariants:** (A1) No action represented as authorized bypasses the authorizer. (A2) Redacted/presented output never claims to be full raw content. (A3) A shortened result is emitted only if raw bytes are securely retained and retrievable, or the caller explicitly opts into irreversible reduction. (A4) Timeouts and overflow carry explicit error states. (A5) A direct-child timeout never claims whole-process-tree cleanup without a verified supervisor. (A6) The owner of secret-bearing output owns deletion/retention policy.

**Failure tests:** executable not found; child exit 1; signal exit; stderr-only failure; child times out; stdout exceeds cap; stderr exceeds cap; descendant keeps pipe open; invalid UTF-8; embedded NUL; disk full during raw persist; pre-existing corrupted blob; simultaneous identical blob commits; invalid retrieval handle. **Patch status:** some source-level and unit regression hooks implemented; process tree containment, spool-to-disk, rate controls and secrets lifecycle outstanding.

#### 11.2 Plane B: provenance-native source retrieval — high leverage

**Composition:** ignore-aware streaming lexical scan (uutils grep/Conduit), budgeted symbol outline (ast-bro), grammar-aware structural search (ast-grep), semantic edit/diff preview (diffsitter), revision-aware file hashes (Conduit). A unified `SourceRef` should include canonical workspace identity, repository revision or snapshot generation, relative path, full content hash, optional syntax-span identity, origin file and retrieval limits. A UI or agent can ask `find -> symbols -> peek -> usages -> diff -> edit` without losing identity at step boundaries.

**Specific access pattern:** start with lexical search; only parse the top relevant files; retrieve affected symbol neighborhood within a strict context budget; verify file bytes match the source hash before offering a change; list dependencies and tests; compute a byte diff plus optional AST diff. A parse failure must not silently hide lexical hits. Symlinks, paths changing during open, ignored generated directories, binary files, stale indexes, branch switches, and worktrees each require explicit behavior.

**Capability graduation gates:** establish representative repositories (Rust, TypeScript, Python, Go, mixed/broken projects, Windows paths); measure recall@k for symbols, stale-hit rate after edits, p50/p95 indexing/query time, peak RSS and wrong-file write rate. Only then consider persistent indexes and embedding models.

#### 11.3 Plane C: recovery-safe edit and change-evidence system — foundational before agent auto-edits

**Composition:** per-file BLAKE3 compare-before-write (Conduit), Graphite canonical revision history, grit-style reserved write intent, Tinyhivemind revision-checked durable transaction, Tinyrsi shadow evaluation. Represent an edit proposal with: immutable source hashes; normalized canonical target list; proposed content hashes; diff/intent; validation commands; idempotency key; source agent; human approval state. Prepare replacement files durably, write a checksummed journal/transaction record, commit with a defined ordering, synchronize directories, and recover deterministically before accepting new operations.

**Crucial limit:** a transaction across multiple ordinary filesystem paths cannot be made perfectly atomic merely by repeated renames. Pick one contract: (1) use isolated Git worktree and atomic branch reference for review/promotion where appropriate; (2) use a journal that supports roll-forward/rollback on restart, documenting the interim partial visibility; (3) confine changes to a transactional state store and materialize outputs from a committed manifest. Do not label best-effort rollback as full crash atomicity. For arbitrary external editors, Conduit's advisory lock is not a global synchronization guarantee.

**Fault-injection gates:** crash after journal `prepared`; crash after first rename; crash during metadata fsync; overwritten temporary file; hardlink alias; symlink swap; disk full; concurrent Conduit and non-Conduit writer; interrupted restore. Verify both file bytes and metadata after each restart, and require manual-recovery status rather than reopening an inconsistent workspace as healthy.

#### 11.4 Plane D: trustworthy context economy — high ROI only with outcome evaluation

**Composition:** RTK specialized output reducers, ast-bro signal-aware squeezing, ICM deterministic context snapshot, Conduit signed word savings, Tinyrsi paired shadow evaluations. Context compression and tool-output reduction become *optimization candidates*; they are not assumed safe because strings got shorter. Keep an optional per-command adapter registry with versioned semantics (e.g., cargo test, pytest, git diff/status, rg, compiler diagnostics, package installs) and a general lossless fallback.

**Proposed measurement:** for every paired agent task, compare task completion, diagnosis correctness, retries, total input/output tokens from the **actual provider tokenizer**, end-to-end elapsed time and tool re-executions. Also record the 95th-percentile penalty, false negative diagnostics, and number of expansion invocations. A win is net improvement on a declared Pareto frontier with no unacceptable task quality regression. The six-line sample baseline cannot establish such a win; the patch's longer synthetic fixture only establishes a nonempty regression input and hint-inclusive CLI behavior, not a real-task business case.

#### 11.5 Plane E: optional memory and indexing data layer — medium to long term

**Composition:** Ragit chunked provenance, IWE backlink graph, ICM RRF and project-scoped memories, Memvid checksummed generations, HydraDB redacted observability patterns. Start with a small durable catalog: sources `(path, revision, hash, sensitivity, mtime, retention)`; observations `(source, kind, content hash, created_at, expiry)`; symbolic edges `(source, target, kind, provenance)`; indexes with `generation_id` and explicit degradation. A local SQLite database may be a pragmatic metadata choice, but it must not double-store raw observations that already live in the blob store.

**Decoupling:** lexical retrieval always available; AST retrieval available when grammars are installed; semantic retrieval optional and failure-isolated. On semantic-index failures, return lexical hits with `semantic_degraded=true`; on out-of-date documents, return stale flags, not fabricated confidence. Implement provenance-aware deletion, private data segregation, and no-export defaults. Avoid adopting Postgres, graph DB, embedded vector engine and single-file memory database simultaneously: this would multiply maintenance without evidence of demand.

#### 11.6 Plane F: governed improvement and multi-agent orchestration — transformational, conditional

**Composition:** Tinyrsi typed improvement proposal and paired evaluation, Tinyhivemind scheduler and durable revision, grit collision controls, Graphite event histories. An automated candidate must first pass static policy validation; then run in an isolated shadow worktree; then pass reproducible tests and cross-platform checks; then undergo task-level non-inferiority comparison; then obtain explicit promotion approval. Every stage produces immutable evidence, reason codes and a rollback/reject decision. Never self-approve a source change using only a candidate's own agent-written test.

**Why not now:** absent a proven stable executable, durable write boundary and reliable benchmark, orchestration is a force multiplier for defects. Initiate only when there is demonstrated independent-work demand, measurable saved engineering time, and enough CI throughput to test parallel results reproducibly.

#### 11.7 Plane G: operator/developer experience and release engineering — immediate improvements

There is no current frontend to “redesign.” The correct default UI is a carefully structured terminal workflow: compact help, sensible named presets, discoverable `peek`, readable path/line/hash output, recoverable handles, explainable rejects, machine-readable JSON for automation, a `doctor` command that reports *actual* capability availability, and explicit incomplete/degraded statuses. Later, a lightweight TUI inspector can show recent runs, output expansion, per-session accounting, source context, proposed diffs and recovery status. A sprawling dashboard would be premature.

Build a source-of-truth CLI schema and generate command reference docs; package exact tested binaries for Linux, macOS and Windows; have Homebrew/winget artifacts consume release checksums; pin or attest dependencies and CI actions; require install/uninstall and upgrade tests. `rtk-ldp` is more useful as a reference for docs-generation mechanics than a site to fork wholesale.

### 12. Findings and opportunities created or refined by the convergence pass

The 35 primary audit findings in Sections 1–6 remain the baseline issue inventory. The following are **additional cross-peer improvement candidates and defect refinements**, not 18 extra claims of confirmed exploitable vulnerabilities. Status labels indicate whether the supplied diff addresses the narrow aspect.

#### CP-01 — Ineffective savings metric due to recovery-hint overhead

- **Description / evidence:** The original reduction metric counted reducer words but omitted the handle and explanatory hint that the agent actually sees. RTK's `src/core/guard.rs:21` explicitly compares filtered and raw estimates; Conduit's earlier `render_stdout` could treat a short reduction as beneficial before evaluating the hint. **Root cause:** measuring a transformation in isolation rather than the end-user presentation path.
- **Impact:** Technical: false “saved tokens” and unnecessary writes; operational: bad optimization choices; security: no direct elevation; reliability: memory/time churn; scalability: avoidable blob growth; business: invalid ROI reports.
- **Category / reference:** High-Leverage / RTK never-worse guard + local reversible store. **Severity:** High. **Confidence:** High. **Effort:** S. **ROI:** High.
- **Recommendation / disposition:** **Patched:** estimate hint-inclusive net benefit before persisting; fallback raw if reduction loses; Python harness now invokes actual Rust presentation path (subject to compile verification). **Validation:** fixtures where hint outweighs small savings, large diagnostic fixture, exact expansion hash, real tokenizer cost study.

#### CP-02 — Lossless-claim boundary at bounded output capture

- **Description / evidence:** Capturing at most N bytes while draining a larger child output is bounded retention, *not* full raw recovery. `src/run.rs` now identifies truncated stdout/stderr; `src/main.rs` exits with an incomplete-output warning. **Root cause:** hard resource ceilings and unconditional lossless storage are incompatible unless a bounded/streaming spool is configured.
- **Impact:** Technical: irrecoverably missing suffix; operations: retries; security: possible secret-bearing partial logs; reliability: downstream misinterpretation; scalability: resource exhaustion; business: user trust loss.
- **Category / reference:** High-Leverage / RTK raw-retrieval adapters. **Severity:** Critical if partial output is treated as complete, otherwise Medium. **Confidence:** High. **Effort:** M. **ROI:** High.
- **Recommendation:** **Partially patched:** signal incomplete and fail closed instead of silently claiming losslessness. Implement an optional bounded encrypted/private spool and retention quota or require caller opt-in to explicitly lossy capture. **Validation:** produce output 1 byte below/at/above cap on both streams; assert no reported complete result when bytes were dropped.

#### CP-03 — Direct-child timeouts are not process-tree containment

- **Description / evidence:** `src/run.rs` kills the direct child; it deliberately does not assert descendant containment. AI-jail's platform-specific probes in `tests/sandbox_escape.rs` demonstrate why isolated execution is a different concern. **Root cause:** portable `std::process::Child` is not an OS resource/namespace supervisor.
- **Impact:** Tech/ops: orphan children; security: unauthorized descendant activity if callers mistake wrapper for sandbox; reliability: held-open pipes; scalability: runaway resources; business: production risk.
- **Category / reference:** Transformational safety boundary / ai-jail capability tests. **Severity:** Critical when hostile commands are in scope. **Confidence:** High. **Effort:** L/XL. **ROI:** High for security-sensitive deployments.
- **Recommendation:** Introduce capability-probed Windows Job Objects / Unix process groups+cgroups and verified Linux namespaces/seccomp or macOS containment as separate, non-bypassable operating modes; fail closed if the requested containment is unavailable. **Validation:** nested child/grandchild, escape attempts, process-list snapshots, failure injection on each supported OS. **Diff:** deliberately does not pretend to solve this.

#### CP-04 — Storage correctness, privacy and lifecycle are separate dimensions

- **Description / evidence:** Integrity verification alone does not define retention, encrypted-at-rest guarantees, directory authority, quota, scrub, key management or deletion. Original `crates/store` only saved flat blobs; Memvid's `src/footer.rs:58,76` demonstrates checksum-and-generation recovery, while Conduit's patched `verify_blob` checks recovered bytes.
- **Impact:** Technical: unnoticed corruption without checks; operational: expanding state and backups; security: long-lived sensitive command output; reliability: missing/expired blobs; scalability: storage exhaustion; business: confidentiality and maintenance burden.
- **Category / reference:** High-Leverage / Memvid checksummed manifest & RTK recovery. **Severity:** High. **Confidence:** High on missing lifecycle; Medium for future deployment outcomes. **Effort:** M/L. **ROI:** High.
- **Recommendation:** **Partially patched:** verify reads, reject staging aliases, harden Unix permissions. Next add per-session TTL, quotas, `inspect`, `verify`, GC dry-run, explicit scrub, and a crash-checksummed catalog. Do not re-label BLAKE3 integrity as authentication or encryption. **Validation:** corrupt bytes, concurrent insertion, symlink attacks, low-disk conditions, restart recovery, Windows ACL tests and scoped deletion.

#### CP-05 — Edit rollback cannot provide cross-file atomic visibility

- **Description / evidence:** `crates/edit/src/lib.rs` stages and renames files sequentially and rolls back best-effort. External editors can race between hash check and rename, and power failure can interrupt a batch. Graphite's `document/graph-storage/src/history.rs:99` offers revision ordering, but a CRDT is not an ordinary filesystem transaction.
- **Impact:** Technical/data: partially applied changes; operational: manual repair; security: symlink races; reliability: inconsistent builds; scalability: agent write conflicts; business: loss of user code.
- **Category / reference:** Transformational reliability / Graphite revision DAG, Tinyhivemind SQLite revision checks, grit reservations. **Severity:** Critical. **Confidence:** High. **Effort:** L. **ROI:** High.
- **Recommendation:** **Partially patched:** canonical duplicates, hardlink aliases on Unix, target freshness and process-lifetime advisory lock; explicit docs retract atomicity guarantee. Next define journal state machine and worktree promotion or manifest materialization. **Validation:** power/crash injection at each rename/fsync, symlink swaps, Windows hardlink aliases, external editor races, recovery proof.

#### CP-06 — Search snapshots are bounded but still not scalable indexes

- **Description / evidence:** `crates/atlas` was a whole-directory in-memory read; the patch uses ignore-aware walker, file limits and a global cap. CodeSeek's `rust-core/src/watcher.rs:213` demonstrates event-driven updates; Ragit's `src/index/file.rs:24` defines a chunking reader.
- **Impact:** Technical: stale or incomplete results, parsing constraints; operations: repeated IO; security: accidental secret indexing; reliability: ignored/missing files; scalability: O(repo) per query; business: poor large-repository UX.
- **Category / reference:** High-Leverage / uutils grep, CodeSeek, Ragit. **Severity:** High for large/secret repositories. **Confidence:** High. **Effort:** M/L. **ROI:** High.
- **Recommendation:** **Partially patched:** bounded scan, skip likely secret directories, hash-bearing hits and `peek`; add explicit complete/degraded field, larger-file streaming, watch invalidation, optional persistent generation and comprehensive secret-path defaults. **Validation:** gitignored files, symlinks, broken UTF-8, renames, in-flight writes, monorepo stress, peak RSS and false-negative audit.

#### CP-07 — Source hash is not a memory identity, scope or truth proof

- **Description / evidence:** Original `crates/memory/src/lib.rs:8-23` keys facts by one string and compares source byte hashes globally; `invalidate_changed` does not know which source each fact belongs to. ICM's `crates/icm-core/src/context_snapshot.rs:115`, IWE's `crates/liwe/src/graph/index.rs:27,71` and Ragit's chunk sources model richer provenance.
- **Impact:** Technical: wrong fact invalidation, collision of unrelated keys; operations: expensive rebuild; security: inappropriate cross-workspace recall; reliability: stale context; scalability: no incremental invalidation; business: erroneous agent recommendations.
- **Category / reference:** High-Leverage / ICM+IWE+Ragit. **Severity:** High once memory is integrated. **Confidence:** High. **Effort:** M/L. **ROI:** High if long-lived sessions are demanded.
- **Recommendation:** Use composite `(workspace_id, source_id, source_generation, fact_id)`, sensitivity ACL, trust class and expiry. Embed source spans/lineage; avoid a single global `invalidate_changed` API. **Validation:** unrelated files with identical facts, multi-repo sources, source deletion/rename, competing revisions and scoped authorization. **Diff:** not wired or falsely declared complete.

#### CP-08 — Retrieval must have predictable ranking and degraded behavior

- **Description / evidence:** Lexical source search and optional dense retrieval are not interchangeable. ICM `fusion.rs:87` defines deterministic rank fusion, CodeSeek's embedding provider trait decouples inference, and pg_vectorize exposes a hybrid route at `extension/src/search.rs:279`.
- **Impact:** Technical: poor recall, nondeterministic ranking; operations: model outages; security: source egress if embeddings call networks; reliability: empty results during model failure; scalability: index drift and high compute; business: unreliable agent navigation.
- **Category / reference:** Transformational when measured / ICM RRF + CodeSeek hybrid. **Severity:** Medium pre-integration; High after. **Confidence:** High for the architectural distinction, Medium for ROI. **Effort:** L. **ROI:** Medium, conditional.
- **Recommendation:** Lexical-first fallback, optional AST symbols, last-resort semantic rerank; preserve source ID/hashes and return degradation state. **Validation:** paired query set, recall@k, source citation correctness, index stale tolerance, no-network mode, cold-start latency and privacy instrumentation.

#### CP-09 — Conduit budget arithmetic could allow overflow at maximum limits

- **Description / evidence:** Original `crates/relay/src/lib.rs` used saturating additions to compare used+requested with budget. For `budget == u64::MAX`, `used == u64::MAX` and `requested == 1`, saturation yields `MAX <= MAX` and incorrectly allows a mathematically overflowing request. **Root cause:** saturation silently maps overflow into a valid capacity boundary.
- **Impact:** Technical: incorrect gate output; operations/reliability: excess admitted work; security: missing upper bound in future enforcement; scalability/business: unbounded charge/time under extreme accounting inputs.
- **Category / reference:** Incremental correctness. **Severity:** High if wired into enforcement. **Confidence:** High (pure arithmetic proof). **Effort:** S. **ROI:** High.
- **Recommendation / disposition:** **Patched:** `checked_add` rejects overflow; tests assert full-budget exact limit and overflow denial. `relay` still lacks concurrency-safe reservations and is not called by `run`. **Validation:** property tests for monotonicity and boundary values plus integration when used for scheduling.

#### CP-10 — Loop guard can be bypassed through implementation boundaries

- **Description / evidence:** `crates/guard` is an in-memory sequence fingerprint plus consecutive-failure counter; it is not invoked in CLI and lacks identity, session scope, temporal windows, cost and typed failure categories. The patch hardens arithmetic saturation only. **Root cause:** the guard is a local function, not a lifecycle policy.
- **Impact:** Technical: undetected loops or false stopping; operational: token/CPU waste; security: noisy denial patterns; reliability: suppressed legitimate retries; scalability: state accumulation; business: unpredictable agents.
- **Category / reference:** High-Leverage / TinyRSI reason-coded stage failures. **Severity:** Medium now, High if used as authority. **Confidence:** High. **Effort:** M. **ROI:** Medium/High.
- **Recommendation:** Track normalized `(session, tool, arguments digest, source revision, outcome)`, bounded sliding windows and explicit human override; publish precision/recall on replay traces. **Validation:** repeated no-progress tasks, intentionally identical commands with new sources, transient failures, machine restarts.

#### CP-11 — Fanout is neither resource-bounded nor failure-typed

- **Description / evidence:** `crates/fanout/src/lib.rs` uses unbounded input/output channels and collapses worker panics into the ordinary string `"worker panicked"`. It returns sorted reports but lacks cancellation, result types, leases and deadline propagation. Tinyhivemind's revisioned scheduler and TinyRSI archived outcomes show better control semantics.
- **Impact:** Technical: lost error classification; operations: task hangs and opaque failures; security: resource exhaustion; reliability: worker failure hidden as text; scalability: memory/task growth; business: reduced confidence in parallel automation.
- **Category / reference:** High-Leverage / Tinyhivemind coordinator and TinyRSI outcomes. **Severity:** High if integrated. **Confidence:** High. **Effort:** M/L. **ROI:** Medium until parallel workloads exist.
- **Recommendation:** Introduce `TaskId`, `Result<Report, TaskError>`, queue cap, explicit cancellations and work ceilings, scoped budgets and stable receipt ordering. A multi-agent subsystem should be a later optional plugin, not a mandatory dependency for simple tasks. **Validation:** worker panics, hung worker, 10,000 tasks, cancellation races, queue overload, deterministic ordering and memory ceilings.

#### CP-12 — Test-impact selection lacks transitive evidence

- **Description / evidence:** `crates/verify/src/lib.rs` maps tests to exact file names and widens only when a changed path has no direct mapping. A dependent module's tests may not be selected for a changed source unless coverage/dep edges are provided. Ast-bro call graphs and Project RAG indexing give possible inputs, but no complete graph currently exists.
- **Impact:** Technical: missed regressions; operations: misleading green CI; security: incomplete security regression coverage; reliability: shipped breakage; scalability: expensive full-test fallback; business: release risk.
- **Category / reference:** High-Leverage / ast-bro dependency graph, `project-rag` indexing. **Severity:** High if used to skip tests. **Confidence:** High. **Effort:** L. **ROI:** High for large suites.
- **Recommendation:** Until graph/coverage completeness is demonstrated, always run the full suite in release CI. Where coverage exists, version and verify the graph against build targets, language imports and source changes. **Validation:** mutation testing; intentionally break a transitive dependency and assert dependent tests are selected.

#### CP-13 — String command allowlists are not capability/security policies

- **Description / evidence:** `crates/vault/src/lib.rs:6` splits a command string on whitespace and matches the executable name; many allowed programs can execute arbitrary child commands. The CLI never invokes it. Praxis `server/src/pipelines.rs:253,369` shows richer boundary-oriented validation.
- **Impact:** Technical/security: unauthorized side effects if treated as a permission gate; operations: misleading audit record; reliability: bypass; scalability: drift across adapters; business: unacceptable trust assertion.
- **Category / reference:** Transformational security / Praxis capability validation + independent OS sandbox. **Severity:** Critical for untrusted agents. **Confidence:** High. **Effort:** L/XL. **ROI:** High for security workloads.
- **Recommendation:** Typed non-shell command requests, default-deny capabilities, workspace/cwd identity, deterministic policy versions, explicit user confirmation, and platform isolation. Do not claim a string allowlist is confinement. **Validation:** negative/bypass corpus, alternate executable paths, interpreters, shell escapes, symlinked working directories and OS sandbox probes.

#### CP-14 — Benchmark now exercises production formatting but not production outcomes

- **Description / evidence:** Baseline Python script used different reduction logic from Rust. Patch makes `eval/benchmark.py` invoke `conduit bench`, which routes through `render_stdout`, includes hint cost and checks recovered bytes. The corpus remains two synthetic `.log` fixtures, one previously six lines.
- **Impact:** Technical: future optimizer could regress actual agent accuracy; operations: misleading release gate; security: omitted exploit-relevant diagnostics; reliability: wrong debugging context; scalability: workload-dependent overhead; business: unsupported saving claims.
- **Category / reference:** High-Leverage / RTK regression fixtures and TinyRSI paired evaluations. **Severity:** High. **Confidence:** High. **Effort:** M. **ROI:** High.
- **Recommendation:** **Partially patched:** eliminate Python/Rust algorithm mismatch and reject empty corpus; require real task traces, status/diagnostic goldens, tokenizer measurements, latency/CPU/RSS, confidence intervals, and paired success evaluation before publishing savings. **Validation:** baseline vs optimized A/B replay with task-level non-inferiority threshold, repeated across platforms and projects.

#### CP-15 — Documented maturity exceeded integrated runtime maturity

- **Description / evidence:** Baseline `README.md`, `docs/wiki/architecture.md` claimed policy-before-execution, bounded fanout and all-or-nothing edits, yet `src/main.rs` wired only five library crates. **Root cause:** architectural roadmap prose was framed as runtime guarantees.
- **Impact:** Technical: unsafe integration decisions; operations: false runbooks; security: privilege misconceptions; reliability: unrealistic recovery expectation; scalability: premature refactors; business: flawed acquisition/investment assessment.
- **Category / reference:** High-Leverage governance / versioned capability manifests and live CLI docs. **Severity:** High. **Confidence:** High. **Effort:** S. **ROI:** High.
- **Recommendation / disposition:** **Patched:** README, CLI reference, architecture and threat model now distinguish implemented vs planned. Introduce CI doc/API consistency tests and a machine-readable capability manifest. **Validation:** trace every public documented guarantee to a production call path and failing test.

#### CP-16 — Static asset inventories do not equal verified installations

- **Description / evidence:** Initial Conduit archive has one workflow but no completed binary/release provenance in the ZIP; Homebrew tap offers a packaging reference, RTK-LDP docs build scripts and Praxis readiness checks expose missing integration maturity. `eval/release_gate.py` originally checked a few paths.
- **Impact:** Technical/ops: users cannot reproduce known-good outcomes; security: mutable CI dependency tags; reliability: broken install path; scalability: support burden; business: limited adoption.
- **Category / reference:** High-Leverage / Homebrew tap + RTK docs build. **Severity:** High. **Confidence:** High for missing archive evidence, deployment unknown. **Effort:** M/L. **ROI:** High.
- **Recommendation:** **Partially patched:** broaden static release inputs and add Rust CLI benchmark steps to CI. Then pin runner actions, publish SBOM/attestations/checksums and smoke-test all supported OS installers; never issue a release label from fixture existence alone. **Validation:** fresh install, execution, expansion, platform equivalence and verification of released checksum signature.

#### CP-17 — Excessive infrastructure convergence would reduce project value

- **Description / evidence:** Directly merging Graphite's editor (1,125 entries), Tinyhivemind's hive framework (1,340), Praxis gateway (1,050), Ragu's PCD framework (756), pg_vectorize/PostgreSQL or HydraDB into Conduit's 51-file core would create large, unrelated obligations before core correctness.
- **Impact:** Technical: high coupling; operations: unsupported dependencies; security: much larger attack surface; reliability: failure propagation; scalability: unnecessary distributed coordination; business: development sunk cost.
- **Category / reference:** Strategic design constraint. **Severity:** High opportunity cost. **Confidence:** High. **Effort:** S to enforce; XL to undo. **ROI:** High.
- **Recommendation:** Enforce **consumer-before-platform**: no new plane without a defined caller, failure isolation, owner, acceptance benchmark, and rollback path. Use `feature` gates and separate packages for opt-in semantic/index/coordination. **Validation:** architecture decision record for each dependency; cold-start, memory and security regression measurements.

#### CP-18 — UI/workflow opportunities are real but a full product UI is premature

- **Description / evidence:** Conduit is a terminal CLI, with no frontend application files in the 51-file baseline. Peer UIs (Graphite, RAG Enterprise, IWE, RTK site) support very different user journeys. **Root cause:** tool-origin UX and partial documentation, not an inherently missing React component tree.
- **Impact:** Technical: ambiguous statuses; operations: poor diagnosis; security: users miss incomplete/sandbox warnings; reliability: hard recovery; scalability: support load; business: adoption friction.
- **Category / reference:** Incremental to High-Leverage / RTK CLI docs, IWE task-oriented navigation. **Severity/value:** High strategic value, not a confirmed bug. **Confidence:** High. **Effort:** S/M for CLI and doctor, L for TUI. **ROI:** High for CLI, uncertain for full GUI.
- **Recommendation:** **Partially patched:** named run presets, `peek`, clearer lossless/degraded warnings, stdin store, structured README. Later build `doctor`, `inspect`, `gc --dry-run`, `trace`, `diff --review`, JSON machine output, and optional TUI. **Validation:** novice first-run task completion, recovery time, error comprehension and number of CLI round trips.

## 7. Outstanding findings, prioritized remediation and falsifiable release gates

### P0 — required before claiming a production-grade local runtime

**(P0-1) Native build and regression verification.** The current execution environment lacks Cargo, rustc and rustfmt and could not reach the Rust toolchain host due unavailable DNS. The V3 patch is not compiled. Run the full CI on Ubuntu, macOS and Windows, fix compiler/clippy/rustfmt and platform differences, and publish versioned test results. Require a clean, reproducible dependency resolution and handle the absence of a checked-in Cargo.lock where policy warrants reproducibility. This is a **verification gap**, not a proven compilation failure for V3.

**(P0-2) Real authorization and isolation.** `vault` is a standalone string policy primitive, not part of the `run` path. Define whether Conduit is only a wrapper for trusted commands or a security boundary for agents. If the latter, all execution routes must pass a typed authorization gate and execute under an OS-specific, tested sandbox with process-tree supervision, filesystem/network policy and separate principal. Fail closed when sandboxing is requested but unavailable; no silent fallback to privileged execution.

**(P0-3) Durable, recoverable edit batches.** Choose atomic manifest/worktree promotion or a robust checksummed journal with explicit recovery state. Add kill/restart fault injection after each fsync and rename. Preserve external writers' changes rather than blindly restoring stale bytes. Maintain hash/version fencing and reject unknown recovery states. Until validated, block unattended critical batch writes.

**(P0-4) Confidential output lifecycle.** Enforce private directory/file permissions at creation on each supported OS; add quotas, per-session ownership, TTL/GC with durable reference tracking, safe deletion and auditability. Ensure the sensitive result is never silently copied into logs, model requests, crash reports or unapproved backups. Test both normal and malicious adjacent-user access.

### P1 — high ROI next, once P0 gates are established

**(P1-1) Command-format adapters:** Stage explicit parsers for compiler/build/test/grep/diff output, with RTK-style never-worse guards, golden failure/stderr fixtures, byte-exact expansion, and non-inferiority on task decisions. Favor retaining the first error and exit semantics over maximum compression.

**(P1-2) Provenance-native source navigation:** Integrate optional AST outlines only after verifying exact file hash and grammar version, with lexical fallback. Add `symbols`/`references`/`impact` as typed read-only surfaces. Treat generated/untracked files and worktree branch changes explicitly, and measure recall@k and p95.

**(P1-3) Better edit review:** Show bounded human-readable byte-diffs alongside deterministic structure-aware diffs; require explicit target cardinality and source hashes. Introduce an idempotent proposal ID, source/provenance metadata, an approval policy and a replayable recovery receipt.

**(P1-4) True command-level observability:** Use correlated request/run IDs, measured bytes, output completeness, transport status, direct-child exit status, cancellation, deadlines and recovery handles. Add safe opt-in tracing with HydraDB-inspired sensitive-field suppression. Never claim whitespace words are model tokens.

**(P1-5) Operational release maturity:** Pin/checksum third-party actions, lock dependencies where appropriate, produce SBOM/artifact attestations, verify install and upgrade on target OSs, and ship compact generated docs. A Homebrew tap should consume verified release binaries, not drive the build design.

### P2 — strategic conditional improvements

**(P2-1) Versioned metadata and memory:** Reuse the existing blob store rather than duplicating raw files in a vector service; choose one durable metadata store (SQLite or a committed manifest) with schema evolution, source hash, ACL, timestamps, retention and invalidation. Introduce ICM-style RRF only after proving a need for multi-retriever fusion.

**(P2-2) Incremental index and structural change graph:** Add CodeSeek-like watcher invalidation and Graphite-like revision identity only where repository-size benchmarks justify storage and parser overhead. Keep a `degraded=true` lexical fallback when parsers/embeddings fail.

**(P2-3) Parallel coordination and autonomous improvement:** Adapt Tinyhivemind/TinyRSI revision-checked scheduling and paired shadow evaluation **only** after cancellation, durability and approval are reliable. Compare controlled arms with equal work/cost budgets; do not promote self-modifying code based on its own tests.

### Acceptance matrix

| Gate | Reproducible check | Pass criterion | Current evidence |
| --- | --- | --- | --- |
| Source integrity | Compare all 51 original ZIP members to patch Git HEAD | Byte-identical, no hidden baseline changes | **Passed** |
| Patch portability | `git apply --check` + `git apply` on fresh baseline | Clean, no rejected hunks | **Passed** |
| Round-trip byte identity | SHA-256 of each changed file after clean apply | All 27 paths identical to authoring tree | **Passed** |
| Syntax/meta | TOML parser, Python unittest, source gate, `git diff --check` | No errors | **Passed, limited to these checks** |
| Rust formatting | `cargo fmt --all -- --check` | Zero format diffs | **Not run** |
| Rust compilation/tests | `cargo test --workspace --all-targets` | All crates and CLI integration tests green | **Not run** |
| Rust lint/security | Clippy, cargo-deny, locked dependency audit | No unjustified warnings/advisories | **Not run** |
| Cross-OS behavior | Matrix including Windows/macOS | No unacknowledged ACL/process differences | **Not run** |
| Crash resilience | Child, reader, store, session, edit fault injection | No silent corruption, false completeness, or orphaned processes within contract | **Partially instrumented, not run** |
| Actual savings | Representative paired agent trace experiments | Equal/higher task quality with measured model tokens and resource budgets | **No field evidence** |

### Exact commands from a fresh checkout of the original archive

```bash
# On the *original unmodified Conduit tree*, do NOT apply older v1/v2 patches.
git apply --check conduit_v3_unified.patch
git apply conduit_v3_unified.patch

cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --bin conduit
python3 -m unittest discover -s eval -p 'test_*.py'
python3 eval/benchmark.py --binary target/debug/conduit
python3 eval/release_gate.py

```

Windows executable path is `target/debug/conduit.exe`; use `python` when `python3` is not present. Read `docs/wiki/structured-output.md` before integrating agent consumers. There is no process-level sandbox delivered by this patch.

## 8. Final V3 verdict and assurance statement

**Decisions:** GO for source-level experimentation and further engineering after successful native tests and under trusted-user/local-workspace assumptions; NO-GO for privileged untrusted-agent execution, multi-user secret processing, ACID-style workspace transactions, unattended promotion or advertised 10x token savings. These decisions follow the actual remaining conditions, not the prestige or popularity of the peer projects.

**Evidence:** Original ZIP byte comparisons (51/51), direct re-inventory (25 distinct archives; 9,118 regular members), 34 exact peer function/test anchors, lexical scanning of 5,072 source files, explicit v3 code changes, clean patch apply, 27 changed-file hashes verified, Python tests and TOML/static gates. **Inference:** A locally operated, recoverable execution-and-source mediation tool with optional incremental syntax retrieval has the highest defensible value-to-complexity ratio. **Assumptions:** Target workflow is a trusted single-user local coding agent, no hidden deployed enforcement plane exists outside supplied files, and optional services may be degraded independently. **Unknowns:** Native Rust type correctness, OS behavior, malicious filesystem race performance, real agent outcomes, tokenized cost savings and production operational history.

**Do not claim complete perfection or tested production readiness.** The review made targeted, source-grounded integrations while avoiding false equivalence between source features, executable guarantees, and business outcomes. The next risk-reducing milestone is compile-and-verify V3, then durable edits and real containment; optional AST/semantic/retrieval/coordination planes should be graduated on measured task value, not merged wholesale.
