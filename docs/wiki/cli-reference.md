# CLI reference (implemented commands)

## `run [--preset safe|quick|deep|compact] [--timeout-ms N] [--max-output-bytes N] [--budget N] [--compact] -- <command> [args...]`

Starts a child process with the caller's permissions and environment. Captures stdout and stderr independently up to an explicit byte limit and drains excess bytes. Never treats truncated output as complete. On Unix, timeout signals the child's process group, including descendants that remain in it; escaped descendants may survive. Other platforms terminate only the direct child. Output is buffered, so interactive PTYs and temporal interleaving are not supported. The `vault` policy crate does not authorize this command; `run` is **not a sandbox**.

Only `--compact` or the `compact` preset enables conservative, UTF-8 stdout-only reduction on successful commands. If doing so saves no net words once the recovery hint is included, the original stdout passes through without storing a blob. Otherwise a verified content-addressed blob contains the original stdout and an expansion hint is written to stderr. Neither the capture limit nor an unavailable store allows a false claim of lossless recovery.

Presets: `safe` (120 seconds, 8 MiB per stream, no compaction); `quick` (30 seconds, 4 MiB, no compaction); `deep` (600 seconds, 32 MiB, no compaction); `compact` (120 seconds, 8 MiB, with reduction). Explicit flags override a preset regardless of their relative order.

## `find <query> [limit]` and `peek <relative-path> [start-line] [count]`

`find` scans UTF-8 source file-by-file with ignore rules; it does not materialize a whole-repository snapshot. Search stops after 10,000 text files or 64 MiB scanned, skips files larger than 1 MiB, and shows bounded match-centered excerpts. Traversal sorts entries by filename within each directory, providing repeatable results for the same unchanged tree; it is not a relevance ranking, and results can differ when files change. `hit_limit_reached` and `truncated` are separate flags. Results may also be incomplete because of ignore rules or skipped entries. The query must be nonblank and at most 512 bytes; the optional hit limit is 1–500. `peek` prints 1–200 lines from one bounded text file with its hash for inspect→edit workflows. It refuses traversal or secret-directory paths. Neither performs AST parsing.

## `edit <relative-path> <expected-hash> <old> <new>` and `batch-edit [--check] <manifest.json>`

Hash-anchored source edits. `batch-edit` accepts `search_replace` and `replace_range` operations with per-target BLAKE3 preconditions; `--check` validates all operations without changing source content. A plan is not reserved between the preflight and commit. The shared validator blocks path escapes, protected metadata, stale hashes, duplicate canonical targets, hardlinked files, oversized inputs/outputs, and ambiguous matches. Limits: 1–128 operations, 8 MiB per target, 64 MiB combined input/output, and a 1 MiB manifest. Uses a process-lifetime cooperative lock under the workspace `.conduit` directory and staged file replacement. Multi-file crash atomicity and safety from uncooperative writers are not provided. See [batch edit](batch-edit.md).

## `store <text>|-` and `expand <handle>`

Persists bytes in a local BLAKE3-addressed store or recovers them by handle. `store -` reads up to 64 MiB from stdin, rejects larger inputs rather than saving a truncated prefix, and avoids exposing content in command-line arguments. Retrieval verifies the full digest. Handles are not access-control credentials. There is no automatic retention or deletion.

## `doctor`

Read-only integrity scan of the local blob store. Reports verified objects/bytes, observed staging files, and detected errors (up to 32 examples), failing with a nonzero exit code for integrity problems. The scan does not delete blobs or promise an atomic snapshot while other processes write.

## `record`, `replay`, `stats`, `repair-session`

Writes and reads synchronized JSONL session events. Session identifiers are validated. Records are bounded to 8 KiB; each session log is limited to 64 MiB. `stats` calculates totals from a streaming, validated traversal. An incomplete last record fails closed. `repair-session` appends a missing newline to an otherwise valid event; for malformed tails it writes a private, synced `.torn-*.bak` copy before truncation and reports its path. It refuses malformed earlier lines and does not reconstruct missing events. Quarantined fragments remain on disk until explicitly handled. Accounting is a caller-provided or whitespace-word estimate, **not tokenizer-derived usage**.

## `bench [file] [budget]`

Exercises the CLI's compaction/persistence/expansion path against an input text file with temporary state; prints effective shown-word estimates including hint overhead. Does not benchmark subprocess capture, actual provider tokenization, agent task success, or real-world latency.

## Environment

- `CONDUIT_HOME`: storage and sessions root; default `.conduit`.
- `CONDUIT_WORKSPACE`: root for `find`, `peek`, `edit`, and `batch-edit`; default current directory.
- `CONDUIT_SESSION`: optional session name for accounting of successful compacted observations.
- `CONDUIT_DEBUG_TIMING`: when set, emits child execution wall time.

Use OS-level isolation for untrusted commands, and avoid passing secrets as arguments because local process listings may expose them.
