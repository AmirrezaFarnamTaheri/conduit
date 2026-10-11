diff --git a/.github/workflows/checks.yml b/.github/workflows/checks.yml
index 79500e9..342e79a 100644
--- a/.github/workflows/checks.yml
+++ b/.github/workflows/checks.yml
@@ -26,13 +26,18 @@ jobs:
           components: rustfmt, clippy
       - uses: actions/setup-python@v5
         with:
-          python-version: '3.x'
+          python-version: '3.12'
       - run: cargo fmt --all -- --check
       - run: cargo test --workspace --all-targets
       - run: cargo clippy --workspace --all-targets -- -D warnings
-      - run: python3 eval/benchmark.py
+      - run: cargo build --bin conduit
+      - run: python3 -m unittest discover -s eval -p "test_*.py"
         if: runner.os != 'Windows'
-      - run: python eval/benchmark.py
+      - run: python -m unittest discover -s eval -p "test_*.py"
+        if: runner.os == 'Windows'
+      - run: python3 eval/benchmark.py --binary target/debug/conduit
+        if: runner.os != 'Windows'
+      - run: python eval/benchmark.py --binary target/debug/conduit.exe
         if: runner.os == 'Windows'
       - run: python3 eval/release_gate.py
         if: runner.os != 'Windows'
diff --git a/Cargo.toml b/Cargo.toml
index 1f8e981..c03fb24 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -21,6 +21,8 @@ conduit-verify = { path = "crates/verify" }
 conduit-vault = { path = "crates/vault" }
 conduit-memory = { path = "crates/memory" }
 conduit-fanout = { path = "crates/fanout" }
+serde = { version = "1", features = ["derive"] }
+serde_json = "1"
 
 [profile.release]
 strip = true
diff --git a/README.md b/README.md
index 2495e26..ba3c314 100644
--- a/README.md
+++ b/README.md
@@ -1,89 +1,99 @@
 # Conduit
 
-Conduit is a local, agent-agnostic runtime for reducing coding-agent context and tool overhead without sacrificing recoverability. It keeps raw observations addressable, applies deterministic transformations first, and makes savings claims measurable rather than aspirational.
+Conduit is a **local coding-agent tool-output and source-navigation prototype**. It aims to minimize noisy tool output while preserving recoverable raw observations and content hashes for subsequent inspection and edits.
 
-## Status
+## Implementation status (source snapshot)
 
-The repository contains executable foundations for Phases 0–6:
+The CLI currently implements `run`, `find`, `peek`, `edit`, `batch-edit`, `store`, `expand`, `doctor`, `record`, `replay`, `stats`, `repair-session`, and `bench`. The independent `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, and `fanout` crates are **not wired into the CLI**, and their behavior is **not an execution security boundary**. No daemon, provider gateway, UI, MCP server, remote API, OS sandbox, verified multi-file crash transaction, or data-retention controller is supplied.
 
-- reversible BLAKE3 blob storage and append-only session accounting
-- fail-open command output lens with expansion handles
-- deterministic repository search with content hashes
-- hash-anchored, transactional edits with rollback and path protections
-- synchronized context ledger, hard budgets, loop guard, impact-aware verification, and policy profiles
-- provenance-bound memory and deterministic parallel fanout
-- cross-platform CI, dependency policy, threat model, and release gates
+The CLI launches local commands with the **same privileges and environment as the caller**. It does not enforce the `vault` policy module or isolate the child process. Run only trusted commands within an appropriately isolated operating-system account or managed sandbox. A timeout kills the direct child but cannot guarantee descendant process cleanup. The wrapper is not a PTY and does not preserve stdout/stderr temporal interleaving.
 
-The local container does not include Cargo. Rust compilation, stress/fuzz tests, and cross-platform verification run in GitHub Actions.
+## Build and checks
 
-## Quick start
+Requires a current stable Rust toolchain with Cargo and Python 3.12 for the evaluation harness:
 
 ```sh
-# Run the deterministic corpus benchmark
-python3 eval/benchmark.py
-
-# Run the release-input check
-python3 eval/release_gate.py
-
-# Build and test when Rust is installed
-cargo test --workspace --all-targets
 cargo fmt --all -- --check
+cargo test --workspace --all-targets
 cargo clippy --workspace --all-targets -- -D warnings
+cargo build --bin conduit
+python3 -m unittest discover -s eval -p 'test_*.py'
+python3 eval/benchmark.py --binary target/debug/conduit
+python3 eval/release_gate.py
 ```
 
-## CLI
+The Python release-input check verifies only file presence and fixture sanity. The benchmark executes the **actual Rust CLI output-presentation path** (including recovery-hint overhead and a raw-expansion check) using deterministic fixtures. It counts **whitespace-separated words, not actual model tokens**, and is not evidence of successful agent tasks or production cost savings. The Rust toolchain was unavailable in the environment used to prepare the accompanying audit patch; compilation and cross-platform results must be established independently.
 
-Build the binary with Cargo, then use:
+## Commands
 
 ```sh
+# Default run: bounded output and timeout, but no compression
 conduit run -- cargo test
-conduit find "target symbol" 20
+# Explicitly reduce successful UTF-8 stdout if net positive with a valid blob handle
+conduit run --preset compact -- cargo test
+# Other presets: safe, quick, deep. Flags override preset settings if supplied afterwards.
+conduit run --preset quick --timeout-ms 10000 --max-output-bytes 1048576 -- command arg
+
+# Read-only search/inspection; find scans file-by-file without a full snapshot
+conduit find 'target symbol' 20
+conduit peek path/to/file.rs 10 40
+# Edits require the entire file's BLAKE3 hash obtained from find or peek
 conduit edit path/to/file.rs EXPECTED_HASH OLD NEW
-conduit store "raw text"
-conduit expand cx:o:HANDLE
-conduit record SESSION MODULE RAW_TOKENS PRESENTED_TOKENS
+# Multi-target hash-anchored preflight and best-effort commit from a JSON plan
+conduit batch-edit --check batch.json
+conduit batch-edit batch.json
+
+conduit store 'raw text'
+# Read binary or secret input from stdin instead of leaking it into argv
+producer_of_bytes | conduit store -
+conduit expand cx:o:OUTPUT_HANDLE
+conduit doctor             # verify blob integrity without deleting objects
+conduit record SESSION MODULE RAW_WORDS SHOWN_WORDS
 conduit replay SESSION
 conduit stats SESSION
-conduit bench
+conduit repair-session SESSION   # explicitly discard only an unterminated tail
+conduit bench eval/corpus/sample.log 700
 ```
 
-`run` preserves the child exit code. Raw output is stored before reduction when possible; reduced output includes an expansion handle whenever content was omitted. If storage or reduction fails, raw output is returned. `CONDUIT_HOME` controls session/blob state, and `CONDUIT_WORKSPACE` controls atlas and edit operations.
+`CONDUIT_HOME` selects local blob/session storage (default `.conduit`). `CONDUIT_WORKSPACE` selects the root for `find`, `peek`, `edit`, and `batch-edit`. Both should be protected by normal OS filesystem permissions. On Unix the store/session code attempts restrictive 0700/0600 modes; Windows ACL isolation is **not validated**. Expansion handles are identifiers, not authorization tokens; treat them as sensitive if the blob is sensitive.
+
+`run` retains a bounded amount from stdout and stderr **independently**, drains the rest, returns an explicit incomplete-output error when either limit is exceeded, and returns `124` when the direct child times out. Such incomplete output is **not losslessly recoverable**. When successful stdout is reduced, the original stdout bytes are persisted first and a recovery handle is emitted to stderr. If storage fails or reduction has no net value after counting the recovery hint, stdout passes through unchanged. For non-successful child exits the program leaves stdout uncompressed; the effective exit status is propagated for ordinary exits, with platform limitations for signal termination. Stderr is never reduced.
 
-## Design principles
+`edit` and `batch-edit` refuse stale BLAKE3 hashes, duplicate canonical targets, escape paths, protected metadata paths, hardlinked targets, and edits over 8 MiB per file. A batch accepts 1–128 operations up to a combined 64 MiB of original and replacement text. `--check` performs the same validations but does not modify source files; it still creates the cooperative lock. The commit writes with per-file staging. It **does not** guarantee all-or-nothing multi-file transactions across crashes, concurrent writes from other tools, or power loss. See `docs/wiki/batch-edit.md` for a manifest example.
 
-1. **Fail open:** Conduit must not become the reason an agent task fails.
-2. **Reversible by default:** omitted content remains retrievable until retention removes it.
-3. **Measured, not claimed:** reductions are reported with raw and presented counts.
-4. **Deterministic first:** parsing, hashing, policy, and selection do not require a model.
-5. **Stale data is unsafe:** file facts, search results, and edits carry hashes.
-6. **Least privilege:** paths, commands, and budgets are checked at boundaries.
-7. **Local first:** no provider or telemetry integration is enabled by default.
+Session records are bounded to 8 KiB each and session logs to 64 MiB. `stats` streams and validates records without materializing the history. If a crash interrupts a JSONL append, `record` and `stats` refuse to gloss over the incomplete tail. `repair-session` only discards the unterminated final fragment after verifying all preceding completed events; it cannot reconstruct missing data and must not be mistaken for an audit-log recovery guarantee. `doctor` provides an on-demand, read-only blob integrity scan; it does not perform retention or GC.
 
 ## Repository map
 
-| Path | Purpose |
-| --- | --- |
-| `crates/store` | Content-addressed raw blobs and expansion handles |
-| `crates/scope` | Append-only session events, replay, and accounting |
-| `crates/lens` | Deterministic output reduction |
-| `crates/atlas` | Snapshot search and hash-bearing hits |
-| `crates/edit` | Hash-anchored transaction edits |
-| `crates/ledger` | Context lifecycle and masking |
-| `crates/relay` | Hard token, cost, and wall-time budgets |
-| `crates/guard` | Repetition and failure-loop detection |
-| `crates/verify` | Impacted-test planning |
-| `crates/vault` | Command and workspace policy |
-| `crates/memory` | Provenance-bound facts |
-| `crates/fanout` | Bounded ordered parallel work |
-| `proto` | Wire-schema source of truth |
-| `eval` | Offline benchmark and release checks |
-| `specs` | Threat model and security constraints |
-| `docs` | Architecture, roadmap, and wiki |
-
-## Security
-
-Treat agent input, repository files, child-process output, and provider responses as untrusted. Review `specs/threat-model.md` before adding integrations. Do not commit credentials, raw secrets, or unreviewed policy changes. Report security issues privately to the project maintainers rather than publishing an exploit first.
+| Path | Responsibility | Current CLI integration |
+| --- | --- | --- |
+| `src/main.rs`, `src/run.rs` | CLI and bounded direct-child subprocess adapter | Yes |
+| `crates/store` | Integrity-checked content-addressed blobs | Yes |
+| `crates/scope` | Synchronized and bounded JSONL session events, explicit torn-tail repair | Yes |
+| `crates/lens` | Conservative diagnostic-preserving output reduction | Yes |
+| `crates/atlas` | File-by-file bounded search, optional snapshot API, and `peek` | Yes |
+| `crates/edit` | Shared edit preflight, typed batch manifests, staged edits and cooperative lock | Yes |
+| `crates/ledger` | In-memory context state | No |
+| `crates/relay` | Stateless authorization *calculation*, not enforcement | No |
+| `crates/guard` | In-memory action repetition guard | No |
+| `crates/verify` | Basic impacted-test selection | No |
+| `crates/vault` | String command policy prototype, not an OS sandbox | No |
+| `crates/memory` | In-memory hash-bound facts | No |
+| `crates/fanout` | Ordered worker fanout with unbounded channels | No |
+| `proto` | Design-only wire schema | No |
+| `eval` | Python harness and offline fixtures | CI/gate only |
+| `docs`, `specs` | Documentation and threat model | Documentation only |
+
+## Operating and security limits
+
+- **No untrusted command isolation:** unrestricted caller privileges; no sandbox enforcement or provider egress policy.
+- **No durable content-retention policy:** the local blob store can grow without garbage collection. Sensitive observations may persist even after a command finishes.
+- **No certified multi-file crash recovery:** stage/rename rollback is best-effort and protects only cooperative Conduit instances.
+- **No exact-token accounting:** `record` accepts caller-supplied numbers; `stats` reports sums of estimates or claims, including negative net savings, not verified model usage.
+- **No workload non-inferiority evidence:** no live-agent success/accuracy or production p95 performance comparisons have been supplied.
+
+See `specs/threat-model.md`, `docs/roadmap.md`, and the external due-diligence assessment before promoting this prototype to production use.
 
 ## License
 
-Apache-2.0. See `LICENSE` when distributed as a release artifact.
+Apache-2.0 in this source snapshot. Any external peer source or third-party dependency must have its actual license checked before redistributing copied code; design-pattern comparison alone does not establish a licensing grant.
diff --git a/crates/atlas/Cargo.toml b/crates/atlas/Cargo.toml
index 28bf86f..da3eaae 100644
--- a/crates/atlas/Cargo.toml
+++ b/crates/atlas/Cargo.toml
@@ -6,3 +6,4 @@ license = "Apache-2.0"
 
 [dependencies]
 blake3 = "1.5"
+ignore = "0.4"
diff --git a/crates/atlas/src/lib.rs b/crates/atlas/src/lib.rs
index 79234de..ee310e9 100644
--- a/crates/atlas/src/lib.rs
+++ b/crates/atlas/src/lib.rs
@@ -1,29 +1,88 @@
+//! Ignore-aware, bounded text search snapshots. This is not yet an AST index.
 use blake3::Hasher;
-use std::{fs, io, path::{Path, PathBuf}};
+use ignore::WalkBuilder;
+use std::{fs, io::{self, Read}, path::{Path, PathBuf}};
+
+const MAX_FILE_BYTES: u64 = 1024 * 1024;
+const MAX_INDEX_BYTES: usize = 64 * 1024 * 1024;
+const MAX_FILES: usize = 10_000;
 
 #[derive(Debug, Clone, PartialEq, Eq)]
-pub struct Hit { pub path: PathBuf, pub line: usize, pub text: String, pub hash: String }
+pub struct Hit {
+    pub path: PathBuf,
+    pub line: usize,
+    pub text: String,
+    pub hash: String,
+}
 
 #[derive(Debug, Clone)]
-pub struct Snapshot { files: Vec<(PathBuf, String, String)> }
+pub struct Snapshot {
+    files: Vec<(PathBuf, String, String)>,
+    skipped: usize,
+    truncated: bool,
+}
 
 impl Snapshot {
     pub fn build(root: impl AsRef<Path>) -> io::Result<Self> {
-        let root = root.as_ref().to_path_buf();
+        let root = root.as_ref().canonicalize()?;
         let mut files = Vec::new();
-        collect(&root, &root, &mut files)?;
+        let mut skipped = 0usize;
+        let mut truncated = false;
+        let mut retained_bytes = 0usize;
+        let walker = build_walker(&root);
+
+        for item in walker {
+            let entry = match item {
+                Ok(entry) => entry,
+                Err(_) => { skipped += 1; continue; }
+            };
+            let Some(kind) = entry.file_type() else { continue };
+            if !kind.is_file() { continue; }
+            let path = entry.path();
+            let metadata = match fs::metadata(path) {
+                Ok(metadata) => metadata,
+                Err(_) => { skipped += 1; continue; }
+            };
+            if metadata.len() > MAX_FILE_BYTES { skipped += 1; continue; }
+            if files.len() >= MAX_FILES || retained_bytes.saturating_add(metadata.len() as usize) > MAX_INDEX_BYTES {
+                truncated = true;
+                break;
+            }
+            let bytes = match read_bounded(path, MAX_FILE_BYTES as usize) {
+                Ok(bytes) => bytes,
+                Err(_) => { skipped += 1; continue; }
+            };
+            if bytes.len() > MAX_FILE_BYTES as usize || looks_binary(&bytes) {
+                skipped += 1;
+                continue;
+            }
+            let text = match String::from_utf8(bytes) {
+                Ok(text) => text,
+                Err(_) => { skipped += 1; continue; }
+            };
+            retained_bytes += text.len();
+            let mut hasher = Hasher::new();
+            hasher.update(text.as_bytes());
+            let relative = path.strip_prefix(&root).unwrap_or(path).to_path_buf();
+            files.push((relative, text, hasher.finalize().to_hex().to_string()));
+        }
         files.sort_by(|left, right| left.0.cmp(&right.0));
-        Ok(Self { files })
+        Ok(Self { files, skipped, truncated })
     }
 
     pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
         if query.is_empty() || limit == 0 { return Vec::new(); }
-        let needle = query.to_ascii_lowercase();
+        let needle = query.to_lowercase();
         let mut hits = Vec::new();
         for (path, contents, hash) in &self.files {
             for (index, line) in contents.lines().enumerate() {
-                if line.to_ascii_lowercase().contains(&needle) {
-                    hits.push(Hit { path: path.clone(), line: index + 1, text: line.to_owned(), hash: hash.clone() });
+                if line.to_lowercase().contains(&needle) {
+                    hits.push(Hit {
+                        path: path.clone(),
+                        line: index + 1,
+                        text: line.to_owned(),
+                        hash: hash.clone(),
+                    });
                     if hits.len() == limit { return hits; }
                 }
             }
@@ -32,37 +91,309 @@ impl Snapshot {
     }
 
     pub fn files(&self) -> usize { self.files.len() }
+    pub fn skipped(&self) -> usize { self.skipped }
+    pub fn truncated(&self) -> bool { self.truncated }
+}
+
+/// File-by-file bounded search without materializing the repository contents.
+/// File iteration is filesystem-dependent; results are not a ranking signal.
+/// `limit_reached` means results may remain even when `truncated` is false.
+#[derive(Debug, Clone)]
+pub struct SearchResult {
+    pub hits: Vec<Hit>,
+    pub scanned_files: usize,
+    pub scanned_bytes: usize,
+    pub skipped: usize,
+    pub truncated: bool,
+    pub limit_reached: bool,
+}
+
+pub fn find_streaming(root: impl AsRef<Path>, query: &str, limit: usize) -> io::Result<SearchResult> {
+    if query.trim().is_empty() || query.len() > 512 || !(1..=500).contains(&limit) {
+        return Err(io::Error::new(io::ErrorKind::InvalidInput, "search requires a nonempty query <=512 bytes and 1-500 hits"));
+    }
+    let root = root.as_ref().canonicalize()?;
+    let needle = query.to_lowercase();
+    let mut result = SearchResult {
+        hits: Vec::new(), scanned_files: 0, scanned_bytes: 0,
+        skipped: 0, truncated: false, limit_reached: false,
+    };
+    'entries: for entry in build_walker(&root) {
+        let entry = match entry {
+            Ok(entry) => entry,
+            Err(_) => { result.skipped += 1; continue; }
+        };
+        if !entry.file_type().is_some_and(|kind| kind.is_file()) { continue; }
+        let metadata = match fs::metadata(entry.path()) {
+            Ok(metadata) => metadata,
+            Err(_) => { result.skipped += 1; continue; }
+        };
+        if metadata.len() > MAX_FILE_BYTES { result.skipped += 1; continue; }
+        if result.scanned_files >= MAX_FILES ||
+            result.scanned_bytes.saturating_add(metadata.len() as usize) > MAX_INDEX_BYTES
+        {
+            result.truncated = true;
+            break;
+        }
+        let bytes = match read_bounded(entry.path(), MAX_FILE_BYTES as usize) {
+            Ok(bytes) => bytes,
+            Err(_) => { result.skipped += 1; continue; }
+        };
+        result.scanned_files += 1;
+        result.scanned_bytes += bytes.len();
+        if looks_binary(&bytes) { result.skipped += 1; continue; }
+        let text = match String::from_utf8(bytes) {
+            Ok(text) => text,
+            Err(_) => { result.skipped += 1; continue; }
+        };
+        let hash = {
+            let mut hasher = Hasher::new();
+            hasher.update(text.as_bytes());
+            hasher.finalize().to_hex().to_string()
+        };
+        let relative = entry.path().strip_prefix(&root).unwrap_or(entry.path()).to_path_buf();
+        for (index, line) in text.lines().enumerate() {
+            let lowercase = line.to_lowercase();
+            let Some(match_byte_offset) = lowercase.find(&needle) else { continue };
+            // A hit is a navigation aid, not a second copy of a megabyte line.
+            // Map a Unicode-lowercased match back to source character offsets;
+            // lowercase transformations can expand one source character.
+            let mut lowered_bytes = 0usize;
+            let mut match_chars = 0usize;
+            for character in line.chars() {
+                if lowered_bytes >= match_byte_offset { break; }
+                lowered_bytes += character.to_lowercase().map(char::len_utf8).sum::<usize>();
+                match_chars += 1;
+            }
+            let start = match_chars.saturating_sub(80);
+            let excerpt_body: String = line.chars().skip(start).take(240).collect();
+            let end = start + excerpt_body.chars().count();
+            let excerpt = format!("{}{}{}",
+                if start > 0 { "..." } else { "" },
+                excerpt_body,
+                if line.chars().count() > end { "..." } else { "" },
+            );
+            result.hits.push(Hit {
+                path: relative.clone(), line: index + 1,
+                text: excerpt, hash: hash.clone(),
+            });
+            if result.hits.len() == limit {
+                result.limit_reached = true;
+                break 'entries;
+            }
+        }
+    }
+    Ok(result)
 }
 
-fn collect(root: &Path, directory: &Path, output: &mut Vec<(PathBuf, String, String)>) -> io::Result<()> {
-    for entry in fs::read_dir(directory)? {
-        let entry = entry?;
-        if entry.file_type()?.is_symlink() { continue; }
-        let path = entry.path();
-        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
-        if name == ".git" || name == ".conduit" || name == "target" { continue; }
-        if path.is_dir() { collect(root, &path, output)?; continue; }
-        if !path.is_file() || fs::metadata(&path)?.len() > 2 * 1024 * 1024 { continue; }
-        let bytes = fs::read(&path)?;
-        if bytes.contains(&0) { continue; }
-        let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-utf8 file"))?;
-        let mut hasher = Hasher::new(); hasher.update(text.as_bytes());
-        output.push((path.strip_prefix(root).unwrap_or(&path).to_path_buf(), text, hasher.finalize().to_hex().to_string()));
-    }
-    Ok(())
+fn build_walker(root: &Path) -> ignore::Walk {
+    WalkBuilder::new(root)
+        .follow_links(false)
+        .hidden(false)
+        .git_ignore(true)
+        .git_exclude(true)
+        .git_global(true)
+        .require_git(false)
+        .filter_entry(|entry| {
+            if entry.depth() == 0 { return true; }
+            let name = entry.file_name().to_string_lossy();
+            !matches!(name.as_ref(),
+                ".git" | ".conduit" | ".hg" | ".svn" | ".venv" | "venv" |
+                "node_modules" | "target" | "dist" | "build" | "__pycache__" |
+                ".next" | ".cache" | ".ssh" | ".aws" | ".secrets" | "secrets") &&
+            !name.starts_with(".env") && !name.starts_with(".conduit-edit-")
+        })
+        .build()
+}
+
+fn read_bounded(path: &Path, max_bytes: usize) -> io::Result<Vec<u8>> {
+    let mut input = fs::File::open(path)?.take(max_bytes as u64 + 1);
+    let mut bytes = Vec::new();
+    input.read_to_end(&mut bytes)?;
+    if bytes.len() > max_bytes {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "file grew beyond indexing limit"));
+    }
+    Ok(bytes)
+}
+
+fn looks_binary(bytes: &[u8]) -> bool {
+    bytes.iter().take(8192).any(|byte| *byte == 0 || (*byte < 0x09) || (*byte > 0x0d && *byte < 0x20))
 }
 
 #[cfg(test)]
 mod tests {
     use super::*;
+    use std::sync::atomic::{AtomicU64, Ordering};
+    static NEXT: AtomicU64 = AtomicU64::new(0);
+
+    fn temp_root(label: &str) -> PathBuf {
+        let root = std::env::temp_dir().join(format!(
+            "conduit-atlas-{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
+        ));
+        fs::create_dir_all(&root).unwrap();
+        root
+    }
+
     #[test]
-    fn search_returns_stable_hashed_hits() {
-        let root = std::env::temp_dir().join(format!("conduit-atlas-{}", std::process::id()));
-        fs::create_dir_all(&root).unwrap(); fs::write(root.join("a.rs"), "fn target() {}\n").unwrap();
-        let snapshot = Snapshot::build(&root).unwrap(); let hits = snapshot.search("TARGET", 10);
-        assert_eq!(snapshot.files(), 1); assert_eq!(hits[0].path, PathBuf::from("a.rs")); assert_eq!(hits[0].line, 1); assert_eq!(hits[0].hash.len(), 64);
+    fn returns_stable_hashed_hits() {
+        let root = temp_root("search");
+        fs::write(root.join("a.rs"), "fn target() {}\n").unwrap();
+        let snapshot = Snapshot::build(&root).unwrap();
+        let hits = snapshot.search("TARGET", 10);
+        assert_eq!(snapshot.files(), 1);
+        assert_eq!(hits[0].path, PathBuf::from("a.rs"));
+        assert_eq!(hits[0].line, 1);
+        assert_eq!(hits[0].hash.len(), 64);
+        assert!(snapshot.search("", 1).is_empty());
         let _ = fs::remove_dir_all(root);
     }
+
+    #[test]
+    fn respects_gitignore_and_excludes_secrets_and_binary_files() {
+        let root = temp_root("ignores");
+        fs::write(root.join(".gitignore"), "*.generated\n").unwrap();
+        fs::write(root.join("safe.rs"), "needle\n").unwrap();
+        fs::write(root.join("secret.generated"), "needle\n").unwrap();
+        fs::write(root.join(".env"), "needle\n").unwrap();
+        fs::write(root.join("binary"), b"needle\0more").unwrap();
+        let snapshot = Snapshot::build(&root).unwrap();
+        let hits = snapshot.search("needle", 100);
+        assert_eq!(hits.len(), 1);
+        assert_eq!(hits[0].path, PathBuf::from("safe.rs"));
+        let _ = fs::remove_dir_all(root);
+    }
+
     #[test]
-    fn limit_and_empty_queries_are_deterministic() { let root = std::env::temp_dir().join(format!("conduit-atlas-empty-{}", std::process::id())); fs::create_dir_all(&root).unwrap(); fs::write(root.join("a"), "x\nx\n").unwrap(); let s = Snapshot::build(&root).unwrap(); assert!(s.search("", 1).is_empty()); assert_eq!(s.search("x", 1).len(), 1); let _ = fs::remove_dir_all(root); }
+    fn invalid_utf8_is_skipped_not_a_global_search_failure() {
+        let root = temp_root("nonutf8");
+        fs::write(root.join("invalid.txt"), [0xff, 0xfe]).unwrap();
+        fs::write(root.join("ok.txt"), "needle\n").unwrap();
+        let snapshot = Snapshot::build(&root).unwrap();
+        assert_eq!(snapshot.search("needle", 10).len(), 1);
+        assert!(snapshot.skipped() >= 1);
+        let _ = fs::remove_dir_all(root);
+    }
+}
+
+/// Bounded, hash-anchored read for the search -> inspect -> edit workflow.
+/// Explicit reads are independent of whole-repository snapshot construction.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Peek {
+    pub hash: String,
+    pub total_lines: usize,
+    pub lines: Vec<(usize, String)>,
+    pub truncated: bool,
+}
+
+pub fn peek(root: impl AsRef<Path>, relative: impl AsRef<Path>, start: usize, count: usize) -> io::Result<Peek> {
+    use std::path::Component;
+    if start == 0 || count == 0 || count > 200 {
+        return Err(io::Error::new(io::ErrorKind::InvalidInput, "peek requires a 1-based start and 1-200 lines"));
+    }
+    let relative = relative.as_ref();
+    if relative.as_os_str().is_empty() || relative.is_absolute() || relative.components().any(|part| matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peek path escapes workspace"));
+    }
+    let root = root.as_ref().canonicalize()?;
+    let path = root.join(relative).canonicalize()?;
+    if !path.starts_with(&root) {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peek path escapes workspace via symlink"));
+    }
+    let logical = path.strip_prefix(&root).unwrap_or(&path);
+    for component in relative.components().chain(logical.components()) {
+        if let Component::Normal(name) = component {
+            let name = name.to_string_lossy();
+            if matches!(name.as_ref(), ".git" | ".conduit" | ".ssh" | ".aws" | ".secrets" | "secrets") || name.starts_with(".env") {
+                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peek of sensitive path is disabled"));
+            }
+        }
+    }
+    let metadata = fs::metadata(&path)?;
+    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "peek supports regular files up to 1 MiB"));
+    }
+    let bytes = read_bounded(&path, MAX_FILE_BYTES as usize)?;
+    if looks_binary(&bytes) {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "peek requires a bounded text file"));
+    }
+    let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "peek requires UTF-8"))?;
+    let mut hasher = Hasher::new();
+    hasher.update(text.as_bytes());
+    let all: Vec<&str> = text.lines().collect();
+    let end = start.saturating_add(count).saturating_sub(1).min(all.len());
+    let selected = all.iter().enumerate().skip(start - 1).take(count).map(|(index, line)| (index + 1, (*line).to_owned())).collect();
+    Ok(Peek {
+        hash: hasher.finalize().to_hex().to_string(),
+        total_lines: all.len(),
+        lines: selected,
+        truncated: end < all.len(),
+    })
+}
+
+#[cfg(test)]
+mod peek_tests {
+    use super::*;
+
+    #[test]
+    fn bounded_peek_includes_stable_hash_and_line_numbers() {
+        let root = std::env::temp_dir().join(format!("conduit-peek-{}", std::process::id()));
+        fs::create_dir_all(&root).unwrap();
+        fs::write(root.join("file.rs"), "one\ntwo\nthree\n").unwrap();
+        let result = peek(&root, "file.rs", 2, 1).unwrap();
+        assert_eq!(result.lines, vec![(2, "two".to_owned())]);
+        assert_eq!(result.total_lines, 3);
+        assert!(result.truncated);
+        assert_eq!(result.hash.len(), 64);
+        assert!(peek(&root, ".env", 1, 10).is_err());
+        assert!(peek(&root, "../outside", 1, 10).is_err());
+        let _ = fs::remove_dir_all(root);
+    }
+}
+
+#[cfg(test)]
+mod streaming_tests {
+    use super::*;
+    use std::sync::atomic::{AtomicU64, Ordering};
+    static NEXT: AtomicU64 = AtomicU64::new(0);
+
+    fn root(label: &str) -> PathBuf {
+        let path = std::env::temp_dir().join(format!(
+            "conduit-streaming-{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
+        ));
+        fs::create_dir_all(&path).unwrap();
+        path
+    }
+
+    #[test]
+    fn streaming_find_respects_ignores_and_returns_edit_hash() {
+        let dir = root("ignores");
+        fs::write(dir.join(".gitignore"), "*.generated\n").unwrap();
+        fs::write(dir.join("a.rs"), "first\nTARGET here\n").unwrap();
+        fs::write(dir.join("b.generated"), "TARGET\n").unwrap();
+        fs::write(dir.join(".env"), "TARGET\n").unwrap();
+        let found = find_streaming(&dir, "target", 4).unwrap();
+        assert_eq!(found.hits.len(), 1);
+        assert_eq!(found.hits[0].path, PathBuf::from("a.rs"));
+        assert_eq!(found.hits[0].line, 2);
+        assert_eq!(found.hits[0].hash, peek(&dir, "a.rs", 1, 2).unwrap().hash);
+        assert!(!found.limit_reached);
+        let _ = fs::remove_dir_all(dir);
+    }
+
+    #[test]
+    fn streaming_find_bounds_hit_count_and_excerpt() {
+        let dir = root("bounded");
+        let long = format!("{}needle\nneedle{}\n", "x".repeat(900), "y".repeat(900));
+        fs::write(dir.join("a.txt"), long).unwrap();
+        let found = find_streaming(&dir, "needle", 1).unwrap();
+        assert!(found.limit_reached);
+        assert_eq!(found.hits.len(), 1);
+        assert!(found.hits[0].text.chars().count() <= 246);
+        assert!(found.hits[0].text.contains("needle"));
+        for bad in ["", " ", "x"] {
+            if bad.trim().is_empty() { assert!(find_streaming(&dir, bad, 1).is_err()); }
+        }
+        assert!(find_streaming(&dir, "needle", 0).is_err());
+        let _ = fs::remove_dir_all(dir);
+    }
 }
diff --git a/crates/edit/Cargo.toml b/crates/edit/Cargo.toml
index 8f8cba7..e0e0534 100644
--- a/crates/edit/Cargo.toml
+++ b/crates/edit/Cargo.toml
@@ -6,3 +6,8 @@ license = "Apache-2.0"
 
 [dependencies]
 blake3 = "1.5"
+fs2 = "0.4"
+serde = { version = "1", features = ["derive"] }
+
+[dev-dependencies]
+serde_json = "1"
diff --git a/crates/edit/src/lib.rs b/crates/edit/src/lib.rs
index 68f6044..2e77bb6 100644
--- a/crates/edit/src/lib.rs
+++ b/crates/edit/src/lib.rs
@@ -1,97 +1,401 @@
+//! Hash-anchored best-effort batch edits. Multi-file crash atomicity is NOT
+//! provided; a journaled filesystem transaction remains a separate project.
+//! Preflight validation is shared by previews and commits; a preview does not
+//! reserve files against non-cooperating writers.
 use blake3::Hasher;
-use std::{collections::HashSet, fs, io, path::{Component, Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
+use fs2::FileExt;
+use std::{
+    collections::HashSet,
+    fs::{self, File, OpenOptions, Permissions},
+    io::{self, Read, Write},
+    path::{Component, Path, PathBuf},
+    sync::atomic::{AtomicU64, Ordering},
+};
 
-#[derive(Debug, Clone)]
-pub enum Operation { SearchReplace { path: PathBuf, expected_hash: String, old: String, new: String }, ReplaceRange { path: PathBuf, expected_hash: String, start: usize, end: usize, new: String } }
+static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
+const MAX_EDIT_FILE_BYTES: usize = 8 * 1024 * 1024;
+const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
+const MAX_OPERATIONS: usize = 128;
+
+#[derive(Debug, Clone, serde::Deserialize)]
+#[serde(tag = "type", rename_all = "snake_case")]
+pub enum Operation {
+    SearchReplace { path: PathBuf, expected_hash: String, old: String, new: String },
+    ReplaceRange { path: PathBuf, expected_hash: String, start: usize, end: usize, new: String },
+}
 
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct Applied { pub path: PathBuf, pub hash: String }
 
+struct Staged {
+    relative: PathBuf,
+    path: PathBuf,
+    temp: PathBuf,
+    expected: String,
+    updated: String,
+    previous: Vec<u8>,
+    permissions: Permissions,
+}
+
+struct Planned {
+    relative: PathBuf,
+    path: PathBuf,
+    expected: String,
+    updated: String,
+    previous: Vec<u8>,
+    permissions: Permissions,
+}
+
+/// Validate every target without writing a source file. This still creates a
+/// cooperative lock under `.conduit` and does NOT reserve inputs after return.
+pub fn preview(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
+    let workspace = workspace.as_ref().canonicalize()?;
+    let _lock = lock_workspace(&workspace)?;
+    let planned = validate(&workspace, operations)?;
+    Ok(planned.iter().map(|item| Applied {
+        path: item.relative.clone(), hash: hash(item.updated.as_bytes()),
+    }).collect())
+}
+
 pub fn apply(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
     let workspace = workspace.as_ref().canonicalize()?;
-    let lock_path = workspace.join(".conduit-edit.lock");
-    let _lock = Lock::acquire(&lock_path)?;
-    let mut staged = Vec::new();
+    let _lock = lock_workspace(&workspace)?;
+    let planned = validate(&workspace, operations)?;
+    let mut staged = Vec::with_capacity(planned.len());
+    let result = (|| {
+        for item in planned {
+            let temp = create_staged(&item.path, item.updated.as_bytes(), &item.permissions)?;
+            staged.push(Staged {
+                relative: item.relative, path: item.path, temp,
+                expected: item.expected, updated: item.updated,
+                previous: item.previous, permissions: item.permissions,
+            });
+        }
+        // Cooperative validation narrows races, but external writers can
+        // modify files between this check and the OS rename operation.
+        for item in &staged { verify_expected(&item.path, &item.expected)?; }
+        let mut committed = 0usize;
+        for item in &staged {
+            let result = verify_expected(&item.path, &item.expected)
+                .and_then(|_| fs::rename(&item.temp, &item.path));
+            if result.is_ok() { committed += 1; }
+            let result = result.and_then(|_| sync_parent(&item.path));
+            if let Err(error) = result {
+                let mut rollback_error = None;
+                for prior in staged[..committed].iter().rev() {
+                    if let Err(failed) = restore(prior) { rollback_error.get_or_insert(failed); }
+                }
+                return Err(rollback_error.unwrap_or(error));
+            }
+        }
+        Ok(staged.iter().map(|item| Applied {
+            path: item.relative.clone(), hash: hash(item.updated.as_bytes()),
+        }).collect())
+    })();
+    for item in &staged { let _ = fs::remove_file(&item.temp); }
+    result
+}
+
+fn lock_workspace(workspace: &Path) -> io::Result<Lock> {
+    let state_dir = workspace.join(".conduit");
+    fs::create_dir_all(&state_dir)?;
+    if !fs::symlink_metadata(&state_dir)?.file_type().is_dir() {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "workspace state directory must not be a symlink"));
+    }
+    Lock::acquire(&state_dir.join("edit.lock"))
+}
+
+fn validate(workspace: &Path, operations: &[Operation]) -> io::Result<Vec<Planned>> {
+    if operations.is_empty() || operations.len() > MAX_OPERATIONS {
+        return Err(invalid("edit batch must contain 1-128 operations"));
+    }
+    let mut planned = Vec::with_capacity(operations.len());
     let mut paths = HashSet::new();
+    #[cfg(unix)]
+    let mut inodes = HashSet::new();
+    let mut total_bytes = 0usize;
     for operation in operations {
-        let prepared = match prepare(&workspace, operation) {
-            Ok(value) => value,
-            Err(error) => { cleanup(&staged); return Err(error); }
-        };
-        let (relative, expected, updated) = prepared;
-        if !paths.insert(relative.clone()) { cleanup(&staged); return Err(io::Error::new(io::ErrorKind::InvalidInput, "duplicate path in transaction")); }
-        let path = match checked_path(&workspace, &relative) { Ok(value) => value, Err(error) => { cleanup(&staged); return Err(error); } };
-        let metadata = match fs::metadata(&path) { Ok(value) => value, Err(error) => { cleanup(&staged); return Err(error); } };
-        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
-        let temp = path.with_extension(format!("conduit-tmp-{}-{}", std::process::id(), nonce));
-        if let Err(error) = fs::write(&temp, updated.as_bytes()).and_then(|_| fs::set_permissions(&temp, metadata.permissions())) {
-            let _ = fs::remove_file(&temp); cleanup(&staged); return Err(error);
+        let (relative, expected, updated) = prepare(workspace, operation)?;
+        let path = checked_path(workspace, &relative)?;
+        if !paths.insert(path.clone()) {
+            return Err(invalid("two operations resolve to the same canonical file"));
+        }
+        let metadata = fs::metadata(&path)?;
+        if !metadata.is_file() || metadata.len() > MAX_EDIT_FILE_BYTES as u64 {
+            return Err(invalid("edit target must be a regular file <=8 MiB"));
+        }
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::MetadataExt;
+            if metadata.nlink() > 1 {
+                return Err(invalid("edit would silently break a hardlinked file identity"));
+            }
+            if !inodes.insert((metadata.dev(), metadata.ino())) {
+                return Err(invalid("two operations reference the same hardlinked inode"));
+            }
+        }
+        let previous = read_bounded(&path)?;
+        if hash(&previous) != expected {
+            return Err(io::Error::new(io::ErrorKind::WouldBlock, "file changed during edit preparation"));
+        }
+        if updated.len() > MAX_EDIT_FILE_BYTES {
+            return Err(invalid("updated file would exceed 8 MiB"));
         }
-        staged.push((relative, path, temp, expected, updated));
+        total_bytes = total_bytes.checked_add(previous.len()).and_then(|value| value.checked_add(updated.len()))
+            .ok_or_else(|| invalid("edit batch size overflow"))?;
+        if total_bytes > MAX_BATCH_BYTES {
+            return Err(invalid("edit batch input/output exceeds 64 MiB"));
+        }
+        planned.push(Planned {
+            relative, path, expected, updated, previous,
+            permissions: metadata.permissions(),
+        });
     }
-    let mut committed: Vec<(PathBuf, Vec<u8>)> = Vec::new();
-    for (_, path, _, expected, _) in &staged {
-        let current = match fs::read(path) { Ok(value) => value, Err(error) => { cleanup(&staged); return Err(error); } };
-        if hash(&current) != *expected { cleanup(&staged); return Err(io::Error::new(io::ErrorKind::WouldBlock, "file changed during transaction")); }
+    Ok(planned)
+}
+
+fn verify_expected(path: &Path, expected: &str) -> io::Result<()> {
+    if hash(&read_bounded(path)?) != expected {
+        return Err(io::Error::new(io::ErrorKind::WouldBlock, "file changed during edit transaction"));
     }
-    for (_, path, temp, _, _) in &staged {
-        let old = match fs::read(path) { Ok(value) => value, Err(error) => { cleanup(&staged); rollback(&committed); return Err(error); } };
-        if let Err(error) = fs::rename(temp, path) { cleanup(&staged); rollback(&committed); return Err(error); }
-        committed.push((path.clone(), old));
+    Ok(())
+}
+
+fn create_staged(path: &Path, bytes: &[u8], permissions: &Permissions) -> io::Result<PathBuf> {
+    let parent = path.parent().ok_or_else(|| invalid("target has no parent"))?;
+    for _ in 0..16 {
+        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
+        let temp = parent.join(format!(".conduit-edit-{}-{serial}.tmp", std::process::id()));
+        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
+            Ok(file) => file,
+            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
+            Err(error) => return Err(error),
+        };
+        let result = (|| {
+            file.write_all(bytes)?;
+            file.set_permissions(permissions.clone())?;
+            file.sync_all()?;
+            sync_parent(&temp)
+        })();
+        if let Err(error) = result {
+            let _ = fs::remove_file(&temp);
+            return Err(error);
+        }
+        return Ok(temp);
     }
-    Ok(staged.into_iter().map(|(relative, _, _, _, updated)| Applied { path: relative, hash: hash(updated.as_bytes()) }).collect())
+    Err(invalid("unable to create unique edit staging file"))
+}
+
+fn restore(item: &Staged) -> io::Result<()> {
+    let temp = create_staged(&item.path, &item.previous, &item.permissions)?;
+    let result = fs::rename(&temp, &item.path).and_then(|_| sync_parent(&item.path));
+    if result.is_err() { let _ = fs::remove_file(temp); }
+    result
 }
 
 fn prepare(workspace: &Path, operation: &Operation) -> io::Result<(PathBuf, String, String)> {
-    let (relative, expected, updated) = match operation {
+    match operation {
         Operation::SearchReplace { path, expected_hash, old, new } => {
-            let relative = safe_relative(path)?; let target = checked_path(workspace, &relative)?; let bytes = fs::read(target)?; let actual = hash(&bytes);
-            if actual != *expected_hash { return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed")); }
-            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not utf-8"))?;
-            let count = text.matches(old).count(); if count != 1 { return Err(io::Error::new(io::ErrorKind::InvalidInput, "replacement must match exactly once")); }
-            (relative, actual, text.replacen(old, new, 1))
+            if old.is_empty() { return Err(invalid("search text must not be empty")); }
+            let relative = safe_relative(path)?;
+            let bytes = read_bounded(&checked_path(workspace, &relative)?)?;
+            let actual = hash(&bytes);
+            if actual != *expected_hash {
+                return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed"));
+            }
+            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not UTF-8"))?;
+            if text.matches(old).count() != 1 {
+                return Err(invalid("replacement must match exactly once"));
+            }
+            Ok((relative, actual, text.replacen(old, new, 1)))
         }
         Operation::ReplaceRange { path, expected_hash, start, end, new } => {
-            let relative = safe_relative(path)?; let target = checked_path(workspace, &relative)?; let bytes = fs::read(target)?; let actual = hash(&bytes);
-            if actual != *expected_hash { return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed")); }
-            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not utf-8"))?;
-            if *start > *end || *end > text.len() || !text.is_char_boundary(*start) || !text.is_char_boundary(*end) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid UTF-8 range")); }
-            (relative, actual, format!("{}{}{}", &text[..*start], new, &text[*end..]))
+            let relative = safe_relative(path)?;
+            let bytes = read_bounded(&checked_path(workspace, &relative)?)?;
+            let actual = hash(&bytes);
+            if actual != *expected_hash {
+                return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed"));
+            }
+            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not UTF-8"))?;
+            if start > end || *end > text.len() || !text.is_char_boundary(*start) || !text.is_char_boundary(*end) {
+                return Err(invalid("invalid UTF-8 range"));
+            }
+            Ok((relative, actual, format!("{}{}{}", &text[..*start], new, &text[*end..])))
         }
-    };
-    Ok((relative, expected, updated))
+    }
 }
 
 fn safe_relative(path: &Path) -> io::Result<PathBuf> {
-    if path.is_absolute() || path.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace")); }
+    if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace"));
+    }
     Ok(path.to_path_buf())
 }
+
 fn checked_path(workspace: &Path, relative: &Path) -> io::Result<PathBuf> {
     let target = workspace.join(relative).canonicalize()?;
-    if !target.starts_with(workspace) { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace through symlink")); }
+    if !target.starts_with(workspace) {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace through symlink"));
+    }
+    let inner = target.strip_prefix(workspace).unwrap_or(&target);
+    if inner.components().any(|component| matches!(component, Component::Normal(name) if name == ".git" || name == ".conduit")) {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "editing repository metadata is disabled"));
+    }
     Ok(target)
 }
 
+fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
+    let mut bytes = Vec::new();
+    File::open(path)?.take(MAX_EDIT_FILE_BYTES as u64 + 1).read_to_end(&mut bytes)?;
+    if bytes.len() > MAX_EDIT_FILE_BYTES {
+        return Err(invalid("edit target exceeds 8 MiB"));
+    }
+    Ok(bytes)
+}
+
+fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }
 fn hash(bytes: &[u8]) -> String { let mut hasher = Hasher::new(); hasher.update(bytes); hasher.finalize().to_hex().to_string() }
-fn cleanup(staged: &[(PathBuf, PathBuf, PathBuf, String, String)]) { for (_, _, temp, _, _) in staged { let _ = fs::remove_file(temp); } }
-fn rollback(committed: &[(PathBuf, Vec<u8>)]) {
-    for (path, old) in committed {
-        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
-        let temp = path.with_extension(format!("conduit-rollback-{}-{}", std::process::id(), nonce));
-        if fs::write(&temp, old).is_ok() { let _ = fs::rename(&temp, path); }
-        let _ = fs::remove_file(&temp);
+
+fn sync_parent(path: &Path) -> io::Result<()> {
+    #[cfg(unix)]
+    {
+        File::open(path.parent().ok_or_else(|| invalid("no parent directory"))?)?.sync_all()?;
     }
+    #[cfg(not(unix))]
+    let _ = path;
+    Ok(())
 }
 
-struct Lock { path: PathBuf }
-impl Lock { fn acquire(path: &Path) -> io::Result<Self> { fs::create_dir_all(path.parent().expect("lock path has parent"))?; fs::OpenOptions::new().write(true).create_new(path).open(path)?; Ok(Self { path: path.to_path_buf() }) } }
-impl Drop for Lock { fn drop(&mut self) { let _ = fs::remove_file(&self.path); } }
+// The lock is held by the OS and released on process exit. An ordinary
+// lockfile survives crashes without permanently preventing later edits.
+// This coordinates Conduit instances, not external processes editing files.
+struct Lock { file: File }
+impl Lock {
+    fn acquire(path: &Path) -> io::Result<Self> {
+        if let Ok(metadata) = fs::symlink_metadata(path) {
+            if !metadata.file_type().is_file() {
+                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "edit lock path is not a regular file"));
+            }
+        }
+        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
+        file.lock_exclusive()?;
+        Ok(Self { file })
+    }
+}
+impl Drop for Lock {
+    fn drop(&mut self) { let _ = FileExt::unlock(&self.file); }
+}
 
 #[cfg(test)]
 mod tests {
     use super::*;
+    fn temp_root(label: &str) -> PathBuf {
+        static NEXT: AtomicU64 = AtomicU64::new(0);
+        let root = std::env::temp_dir().join(format!("conduit-edit-{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
+        fs::create_dir_all(&root).unwrap();
+        root
+    }
+    fn replace(path: &str, expected: &str, old: &str, new: &str) -> Operation {
+        Operation::SearchReplace { path: path.into(), expected_hash: expected.into(), old: old.into(), new: new.into() }
+    }
+    #[test]
+    fn preexisting_unlocked_lockfile_does_not_block_future_edits() {
+        let root = temp_root("orphan-lock");
+        let path = root.join(".conduit-edit.lock");
+        fs::write(&path, b"left by crashed process").unwrap();
+        let first = Lock::acquire(&path).unwrap();
+        drop(first);
+        let second = Lock::acquire(&path).unwrap();
+        drop(second);
+        let _ = fs::remove_dir_all(root);
+    }
+    #[test]
+    fn edits_one_file_and_rejects_stale_hash() {
+        let root = temp_root("single");
+        let path = root.join("a.txt");
+        fs::write(&path, "before").unwrap();
+        let expected = hash(b"before");
+        assert_eq!(apply(&root, &[replace("a.txt", &expected, "before", "after")]).unwrap().len(), 1);
+        assert_eq!(fs::read_to_string(&path).unwrap(), "after");
+        assert!(apply(&root, &[replace("a.txt", &expected, "after", "bad")]).is_err());
+        let _ = fs::remove_dir_all(root);
+    }
     #[test]
-    fn applies_unique_edit_and_rejects_stale_hash() { let root = std::env::temp_dir().join(format!("conduit-edit-{}", std::process::id())); fs::create_dir_all(&root).unwrap(); let path = root.join("a.txt"); fs::write(&path, "before").unwrap(); let expected = hash(b"before"); let applied = apply(&root, &[Operation::SearchReplace { path: PathBuf::from("a.txt"), expected_hash: expected.clone(), old: "before".into(), new: "after".into() }]).unwrap(); assert_eq!(applied[0].path, PathBuf::from("a.txt")); assert_eq!(fs::read_to_string(&path).unwrap(), "after"); assert!(apply(&root, &[Operation::SearchReplace { path: PathBuf::from("a.txt"), expected_hash: expected, old: "after".into(), new: "bad".into() }]).is_err()); let _ = fs::remove_dir_all(root); }
+    fn rejects_duplicate_target_and_empty_search() {
+        let root = temp_root("duplicate");
+        fs::write(root.join("a.txt"), "before").unwrap();
+        let expected = hash(b"before");
+        assert!(apply(&root, &[replace("a.txt", &expected, "before", "one"), replace("a.txt", &expected, "before", "two")]).is_err());
+        assert!(apply(&root, &[replace("a.txt", &expected, "", "two")]).is_err());
+        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "before");
+        let _ = fs::remove_dir_all(root);
+    }
+    #[cfg(unix)]
     #[test]
-    fn rejects_ambiguous_and_escape_edits() { let root = std::env::temp_dir().join(format!("conduit-edit-reject-{}", std::process::id())); fs::create_dir_all(&root).unwrap(); let path = root.join("a"); fs::write(&path, "x x").unwrap(); let expected = hash(b"x x"); assert!(apply(&root, &[Operation::SearchReplace { path: PathBuf::from("a"), expected_hash: expected, old: "x".into(), new: "y".into() }]).is_err()); assert!(safe_relative(Path::new("../a")).is_err()); let _ = fs::remove_dir_all(root); }
+    fn rejects_symlink_alias_to_same_target() {
+        use std::os::unix::fs::symlink;
+        let root = temp_root("alias");
+        fs::write(root.join("real.rs"), "before").unwrap();
+        symlink(root.join("real.rs"), root.join("alias.rs")).unwrap();
+        let expected = hash(b"before");
+        let result = apply(&root, &[replace("real.rs", &expected, "before", "one"), replace("alias.rs", &expected, "before", "two")]);
+        assert!(result.is_err());
+        assert_eq!(fs::read_to_string(root.join("real.rs")).unwrap(), "before");
+        let _ = fs::remove_dir_all(root);
+    }
+    #[test]
+    fn prevents_path_escape() {
+        let root = temp_root("escape");
+        assert!(safe_relative(Path::new("../a")).is_err());
+        fs::create_dir(root.join(".git")).unwrap();
+        fs::write(root.join(".git/config"), "before").unwrap();
+        assert!(apply(&root, &[replace(".git/config", &hash(b"before"), "before", "after")]).is_err());
+        let _ = fs::remove_dir_all(root);
+    }
+}
+
+#[cfg(test)]
+mod preview_tests {
+    use super::*;
+
+    #[test]
+    fn preview_hash_matches_commit_without_modifying_source() {
+        let root = std::env::temp_dir().join(format!("conduit-preview-{}", std::process::id()));
+        fs::create_dir_all(&root).unwrap();
+        fs::write(root.join("file.txt"), "one two").unwrap();
+        let ops = [Operation::SearchReplace {
+            path: "file.txt".into(), expected_hash: hash(b"one two"),
+            old: "two".into(), new: "three".into(),
+        }];
+        let preview = preview(&root, &ops).unwrap();
+        assert_eq!(fs::read_to_string(root.join("file.txt")).unwrap(), "one two");
+        assert_eq!(preview[0].hash, hash(b"one three"));
+        let applied = apply(&root, &ops).unwrap();
+        assert_eq!(preview, applied);
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn rejects_oversized_input_and_empty_batch_before_editing() {
+        let root = std::env::temp_dir().join(format!("conduit-large-edit-{}", std::process::id()));
+        fs::create_dir_all(&root).unwrap();
+        assert!(preview(&root, &[]).is_err());
+        fs::write(root.join("huge.txt"), vec![b'x'; MAX_EDIT_FILE_BYTES + 1]).unwrap();
+        assert!(apply(&root, &[Operation::SearchReplace {
+            path: "huge.txt".into(), expected_hash: "0".repeat(64),
+            old: "x".into(), new: "y".into(),
+        }]).is_err());
+        assert_eq!(fs::metadata(root.join("huge.txt")).unwrap().len(), (MAX_EDIT_FILE_BYTES + 1) as u64);
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn accepts_typed_batch_manifest_variants() {
+        let manifest = r#"[
+          {"type":"search_replace","path":"a.rs","expected_hash":"abc","old":"foo","new":"bar"},
+          {"type":"replace_range","path":"b.rs","expected_hash":"def","start":0,"end":2,"new":"x"}
+        ]"#;
+        let operations: Vec<Operation> = serde_json::from_str(manifest).unwrap();
+        assert_eq!(operations.len(), 2);
+    }
 }
diff --git a/crates/guard/src/lib.rs b/crates/guard/src/lib.rs
index 9673710..e0acc10 100644
--- a/crates/guard/src/lib.rs
+++ b/crates/guard/src/lib.rs
@@ -8,9 +8,9 @@ pub struct Guard { fingerprints: VecDeque<String>, max_repeats: usize, consecuti
 impl Guard {
     pub fn new(max_repeats: usize, failure_limit: usize) -> Self { Self { fingerprints: VecDeque::new(), max_repeats, consecutive_failures: 0, failure_limit } }
     pub fn observe(&mut self, action: &str, succeeded: bool) -> Decision {
-        if succeeded { self.consecutive_failures = 0; } else { self.consecutive_failures += 1; }
+        if succeeded { self.consecutive_failures = 0; } else { self.consecutive_failures = self.consecutive_failures.saturating_add(1); }
         let mut hasher = Hasher::new(); hasher.update(action.as_bytes()); let fingerprint = hasher.finalize().to_hex().to_string();
-        self.fingerprints.push_back(fingerprint.clone()); if self.fingerprints.len() > self.max_repeats + 1 { self.fingerprints.pop_front(); }
+        self.fingerprints.push_back(fingerprint.clone()); if self.fingerprints.len() > self.max_repeats.saturating_add(1) { self.fingerprints.pop_front(); }
         let repeats = self.fingerprints.iter().filter(|item| item.as_str() == fingerprint).count();
         if repeats > self.max_repeats || self.consecutive_failures >= self.failure_limit { Decision::Halt } else { Decision::Continue }
     }
diff --git a/crates/lens/src/lib.rs b/crates/lens/src/lib.rs
index a42f756..fd72d9a 100644
--- a/crates/lens/src/lib.rs
+++ b/crates/lens/src/lib.rs
@@ -1,9 +1,13 @@
-use std::collections::BTreeMap;
-
+//! Conservative, reversible-presentation log reduction.
+//!
+//! `distill` never stores raw data. Callers MUST preserve recoverable raw bytes
+//! before displaying a reduced result. An exceeded budget is possible when
+//! retaining diagnostics is safer than omitting them.
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct Distilled {
     pub presented: String,
     pub dropped: Vec<String>,
+    /// Whitespace-delimited word estimates, NOT provider token measurements.
     pub raw_tokens: usize,
     pub presented_tokens: usize,
 }
@@ -11,66 +15,92 @@ pub struct Distilled {
 pub fn distill(raw: &str, budget: usize) -> Distilled {
     let raw_tokens = token_count(raw);
     let lines: Vec<&str> = raw.lines().collect();
-    if raw_tokens <= budget || lines.is_empty() {
-        return Distilled { presented: raw.to_owned(), dropped: Vec::new(), raw_tokens, presented_tokens: raw_tokens };
+    if budget == 0 || raw_tokens <= budget || lines.is_empty() {
+        return Distilled {
+            presented: raw.to_owned(),
+            dropped: Vec::new(),
+            raw_tokens,
+            presented_tokens: raw_tokens,
+        };
     }
 
-    let mut selected = Vec::new();
-    let mut seen = BTreeMap::new();
-    for line in &lines {
-        let trimmed = line.trim();
-        if is_signal(trimmed) && seen.insert(trimmed.to_owned(), ()).is_none() { selected.push(*line); }
+    // Diagnostics win over a soft presentation budget. In particular, never
+    // suppress the first error just because a later summary is shorter.
+    let mut keep = vec![false; lines.len()];
+    let mut retained_words = 0usize;
+    for (index, line) in lines.iter().enumerate() {
+        if is_signal(line.trim()) {
+            keep[index] = true;
+            retained_words += token_count(line);
+        }
     }
 
-    let tail_start = lines.len().saturating_sub(20);
-    for line in &lines[tail_start..] {
-        if seen.contains_key(line) { continue; }
-        let candidate = line.to_string();
-        let with_candidate = join_lines(&selected, Some(line));
-        if token_count(&with_candidate) <= budget {
-            seen.insert(candidate, ());
-            selected.push(*line);
+    // Add recent surrounding context within the remaining budget. Selection
+    // happens newest-first, but output is emitted in original line order.
+    for index in (lines.len().saturating_sub(20)..lines.len()).rev() {
+        if keep[index] {
+            continue;
+        }
+        let words = token_count(lines[index]);
+        if retained_words.saturating_add(words) <= budget {
+            keep[index] = true;
+            retained_words += words;
         }
     }
 
-    let presented = join_lines(&selected, None);
-    let retained = selected.len();
-    let dropped = if retained < lines.len() { vec![format!("{} lines", lines.len() - retained)] } else { Vec::new() };
-    Distilled { presented: presented.clone(), dropped, raw_tokens, presented_tokens: token_count(&presented) }
-}
-
-fn join_lines(lines: &[&str], extra: Option<&str>) -> String {
-    let mut result = lines.join("\n");
-    if let Some(line) = extra {
-        if !result.is_empty() { result.push('\n'); }
-        result.push_str(line);
+    let selected: Vec<&str> = lines
+        .iter()
+        .zip(keep.iter())
+        .filter_map(|(line, included)| included.then_some(*line))
+        .collect();
+    let removed = lines.len() - selected.len();
+    if removed == 0 {
+        return Distilled {
+            presented: raw.to_owned(),
+            dropped: Vec::new(),
+            raw_tokens,
+            presented_tokens: raw_tokens,
+        };
+    }
+    let presented = format!("{}\n", selected.join("\n"));
+    Distilled {
+        presented_tokens: token_count(&presented),
+        presented,
+        dropped: vec![format!("{removed} lines")],
+        raw_tokens,
     }
-    if !result.is_empty() { result.push('\n'); }
-    result
 }
 
 fn is_signal(line: &str) -> bool {
     let lower = line.to_ascii_lowercase();
     ["error", "failed", "failure", "panic", "fatal", "traceback", "exception", "assertion", "warning", "passed", "exit code", "exit="]
-        .iter().any(|needle| lower.contains(needle)) || line.contains(":/") || line.contains(".rs:") || line.contains(".py:") || line.contains(".ts:")
+        .iter()
+        .any(|needle| lower.contains(needle))
+        || line.contains(".rs:")
+        || line.contains(".py:")
+        || line.contains(".ts:")
 }
 
-fn token_count(text: &str) -> usize { text.split_whitespace().count() }
+fn token_count(text: &str) -> usize {
+    text.split_whitespace().count()
+}
 
 #[cfg(test)]
 mod tests {
     use super::*;
+
     #[test]
-    fn preserves_failure_location_and_exit_state() {
-        let raw = "noise one\nnoise two\nerror src/lib.rs:42 failed\nnoise three\nexit=1\n";
+    fn preserves_diagnostics_and_exit_state_chronologically() {
+        let raw = "noise one\nerror src/lib.rs:42 failed\nnoise two\nexit=1\n";
         let result = distill(raw, 5);
         assert!(result.presented.contains("src/lib.rs:42"));
         assert!(result.presented.contains("exit=1"));
-        assert!(result.presented_tokens <= 5);
+        assert!(result.presented.find("src/lib.rs:42") < result.presented.find("exit=1"));
         assert!(!result.dropped.is_empty());
     }
+
     #[test]
-    fn never_drops_a_signal_when_budget_is_too_small() {
+    fn preserves_critical_signal_even_over_soft_budget() {
         let raw = "error this diagnostic has a long explanation src/main.rs:9\nordinary noise\n";
         let result = distill(raw, 1);
         assert!(result.presented.contains("src/main.rs:9"));
@@ -81,14 +111,24 @@ mod tests {
     fn does_not_report_a_drop_when_all_lines_are_retained() {
         let raw = "ordinary\nerror src/main.rs:1\n";
         let result = distill(raw, 3);
+        assert_eq!(result.presented, raw);
         assert!(result.dropped.is_empty());
     }
 
     #[test]
     fn passthrough_is_byte_exact_under_budget() {
-        let raw = "ok\n";
+        let raw = "ok\r\n";
         assert_eq!(distill(raw, 10).presented, raw);
-        assert!(distill(raw, 10).dropped.is_empty());
     }
 
+    #[test]
+    fn does_not_reorder_signal_lines_with_tail_context() {
+        let raw = "error:first a b\nnoise a b\nwarning:second c d\nnoise c d\nexit=1\n";
+        let shown = distill(raw, 7).presented;
+        let ordered: Vec<_> = shown.lines().collect();
+        let a = ordered.iter().position(|line| line.contains("error:first")).unwrap();
+        let b = ordered.iter().position(|line| line.contains("warning:second")).unwrap();
+        let c = ordered.iter().position(|line| line.contains("exit=1")).unwrap();
+        assert!(a < b && b < c);
+    }
 }
diff --git a/crates/relay/src/lib.rs b/crates/relay/src/lib.rs
index 177b69f..d246e17 100644
--- a/crates/relay/src/lib.rs
+++ b/crates/relay/src/lib.rs
@@ -1,13 +1,59 @@
+//! Pure accounting feasibility primitive. This is NOT a concurrent reservation
+//! service and is not currently used to authorize `conduit run`.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub struct Budget { pub tokens: u64, pub dollars_micros: u64, pub wall_ms: u64 }
+pub struct Budget {
+    pub tokens: u64,
+    pub dollars_micros: u64,
+    pub wall_ms: u64,
+}
+
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub struct Usage { pub tokens: u64, pub dollars_micros: u64, pub wall_ms: u64 }
+pub struct Usage {
+    pub tokens: u64,
+    pub dollars_micros: u64,
+    pub wall_ms: u64,
+}
+
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub enum Decision { Allow, Reject }
+pub enum Decision {
+    Allow,
+    Reject,
+}
+
+fn within_budget(used: u64, request: u64, maximum: u64) -> bool {
+    // Saturating addition would incorrectly permit overflow if maximum=MAX.
+    used.checked_add(request).is_some_and(|total| total <= maximum)
+}
 
 pub fn authorize(budget: Budget, used: Usage, requested: Usage) -> Decision {
-    if used.tokens.saturating_add(requested.tokens) > budget.tokens || used.dollars_micros.saturating_add(requested.dollars_micros) > budget.dollars_micros || used.wall_ms.saturating_add(requested.wall_ms) > budget.wall_ms { Decision::Reject } else { Decision::Allow }
+    if within_budget(used.tokens, requested.tokens, budget.tokens)
+        && within_budget(used.dollars_micros, requested.dollars_micros, budget.dollars_micros)
+        && within_budget(used.wall_ms, requested.wall_ms, budget.wall_ms)
+    {
+        Decision::Allow
+    } else {
+        Decision::Reject
+    }
 }
 
 #[cfg(test)]
-mod tests { use super::*; #[test] fn hard_limits_reject_overruns() { let budget = Budget { tokens: 10, dollars_micros: 10, wall_ms: 10 }; let used = Usage { tokens: 9, dollars_micros: 9, wall_ms: 9 }; assert_eq!(authorize(budget, used, Usage { tokens: 1, dollars_micros: 1, wall_ms: 1 }), Decision::Allow); assert_eq!(authorize(budget, used, Usage { tokens: 2, dollars_micros: 1, wall_ms: 1 }), Decision::Reject); } }
+mod tests {
+    use super::*;
+
+    #[test]
+    fn allows_the_exact_limit_and_rejects_excess() {
+        let budget = Budget { tokens: 10, dollars_micros: 10, wall_ms: 10 };
+        let used = Usage { tokens: 9, dollars_micros: 9, wall_ms: 9 };
+        assert_eq!(authorize(budget, used, Usage { tokens: 1, dollars_micros: 1, wall_ms: 1 }), Decision::Allow);
+        assert_eq!(authorize(budget, used, Usage { tokens: 2, dollars_micros: 1, wall_ms: 1 }), Decision::Reject);
+    }
+
+    #[test]
+    fn overflow_never_authorizes_a_request() {
+        let budget = Budget { tokens: u64::MAX, dollars_micros: u64::MAX, wall_ms: u64::MAX };
+        let zero = Usage { tokens: 0, dollars_micros: 0, wall_ms: 0 };
+        let at_max = Usage { tokens: u64::MAX, dollars_micros: 0, wall_ms: 0 };
+        assert_eq!(authorize(budget, at_max, Usage { tokens: 1, ..zero }), Decision::Reject);
+        assert_eq!(authorize(budget, zero, Usage { tokens: u64::MAX, ..zero }), Decision::Allow);
+    }
+}
diff --git a/crates/scope/Cargo.toml b/crates/scope/Cargo.toml
index b019a4d..7ecf83e 100644
--- a/crates/scope/Cargo.toml
+++ b/crates/scope/Cargo.toml
@@ -5,3 +5,6 @@ edition = "2021"
 license = "Apache-2.0"
 
 [dependencies]
+fs2 = "0.4"
+serde = { version = "1", features = ["derive"] }
+serde_json = "1"
diff --git a/crates/scope/src/lib.rs b/crates/scope/src/lib.rs
index e036759..63d3979 100644
--- a/crates/scope/src/lib.rs
+++ b/crates/scope/src/lib.rs
@@ -1,62 +1,309 @@
-use std::{fmt, fs::{self, OpenOptions}, io::{self, BufRead, BufReader, Write}, path::{Path, PathBuf}, sync::Mutex};
+//! Validated, bounded append-only session events. Readers reject malformed or
+//! torn records. A damaged final fragment requires explicit recovery.
+use fs2::FileExt;
+use serde::{Deserialize, Serialize};
+use std::{
+    fmt,
+    fs::{self, File, OpenOptions},
+    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
+    path::{Path, PathBuf},
+};
 
-#[derive(Debug, Clone)]
-pub struct Savings { pub module: String, pub raw_tokens: u32, pub presented_tokens: u32, pub measured: bool }
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Savings {
+    pub module: String,
+    pub raw_tokens: u32,
+    pub presented_tokens: u32,
+    pub measured: bool,
+}
 
-pub enum Event { Savings(Savings) }
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(tag = "type", rename_all = "snake_case")]
+pub enum Event {
+    Savings(Savings),
+}
 
 impl fmt::Display for Event {
     fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
         match self {
-            Self::Savings(s) => write!(output, "savings module={} raw_tokens={} presented_tokens={} measured={}", s.module, s.raw_tokens, s.presented_tokens, s.measured),
+            Self::Savings(s) => write!(
+                output,
+                "savings module={} raw_tokens={} presented_tokens={} measured={}",
+                s.module, s.raw_tokens, s.presented_tokens, s.measured
+            ),
         }
     }
 }
 
-pub struct EventLog { path: PathBuf, lock: Mutex<()> }
+const MAX_LOG_BYTES: u64 = 64 * 1024 * 1024;
+const MAX_RECORD_BYTES: usize = 8 * 1024;
+const MAX_MODULE_BYTES: usize = 512;
+
+pub struct EventLog {
+    path: PathBuf,
+}
 
 impl EventLog {
     pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
         let path = path.as_ref().to_path_buf();
-        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
-        OpenOptions::new().create(true).append(true).open(&path)?;
-        Ok(Self { path, lock: Mutex::new(()) })
+        let parent = path.parent().ok_or_else(|| io::Error::other("log path has no parent"))?;
+        fs::create_dir_all(parent)?;
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::PermissionsExt;
+            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
+        }
+        if let Ok(metadata) = fs::symlink_metadata(&path) {
+            if !metadata.file_type().is_file() {
+                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "session path is not a file"));
+            }
+        }
+        let file = Self::open_file(&path)?;
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::PermissionsExt;
+            file.set_permissions(fs::Permissions::from_mode(0o600))?;
+        }
+        Ok(Self { path })
+    }
+
+    fn open_file(path: &Path) -> io::Result<File> {
+        OpenOptions::new().read(true).append(true).create(true).open(path)
     }
+
     pub fn append(&self, event: Event) -> io::Result<()> {
-        let _guard = self.lock.lock().map_err(|_| io::Error::other("event log lock poisoned"))?;
-        let line = match event { Event::Savings(s) => format!("{{\"type\":\"savings\",\"module\":\"{}\",\"raw_tokens\":{},\"presented_tokens\":{},\"measured\":{}}}\n", escape(&s.module), s.raw_tokens, s.presented_tokens, s.measured) };
-        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
-        file.write_all(line.as_bytes())?; file.sync_data()
+        let Event::Savings(savings) = &event;
+        if savings.module.is_empty() || savings.module.len() > MAX_MODULE_BYTES {
+            return Err(io::Error::new(io::ErrorKind::InvalidInput, "event module must contain 1-512 bytes"));
+        }
+        let line = serde_json::to_vec(&event).map_err(io::Error::other)?;
+        if line.len() + 1 > MAX_RECORD_BYTES {
+            return Err(io::Error::new(io::ErrorKind::InvalidInput, "session event exceeds 8 KiB"));
+        }
+        let mut file = Self::open_file(&self.path)?;
+        FileExt::lock_exclusive(&file)?;
+        let result = (|| {
+            let length = file.metadata()?.len();
+            if length.saturating_add(line.len() as u64 + 1) > MAX_LOG_BYTES {
+                return Err(io::Error::other("session log reached 64 MiB limit"));
+            }
+            // Do not append onto a torn record: that would permanently poison
+            // valid JSONL and silently undercount previously recorded events.
+            if length != 0 {
+                file.seek(SeekFrom::End(-1))?;
+                let mut tail = [0u8];
+                file.read_exact(&mut tail)?;
+                if tail[0] != b'\n' {
+                    return Err(io::Error::new(io::ErrorKind::InvalidData, "session log has a torn final record; run repair-session"));
+                }
+            }
+            file.write_all(&line)?;
+            file.write_all(b"\n")?;
+            file.sync_data()
+        })();
+        let unlocked = FileExt::unlock(&file);
+        result.and(unlocked)
     }
+
+    fn each_event(&self, mut observe: impl FnMut(Event) -> io::Result<()>) -> io::Result<()> {
+        let file = Self::open_file(&self.path)?;
+        FileExt::lock_shared(&file)?;
+        let result = (|| {
+            if file.metadata()?.len() > MAX_LOG_BYTES {
+                return Err(io::Error::new(io::ErrorKind::InvalidData, "session log exceeds 64 MiB limit"));
+            }
+            let mut reader = BufReader::new(&file);
+            let mut record = Vec::new();
+            let mut number = 0usize;
+            loop {
+                record.clear();
+                let length = reader.read_until(b'\n', &mut record)?;
+                if length == 0 { break; }
+                number += 1;
+                if length > MAX_RECORD_BYTES || record.last() != Some(&b'\n') {
+                    return Err(io::Error::new(io::ErrorKind::InvalidData,
+                        format!("oversized or incomplete session record at line {number}")));
+                }
+                let event = serde_json::from_slice::<Event>(&record[..length - 1]).map_err(|error| {
+                    io::Error::new(io::ErrorKind::InvalidData,
+                        format!("invalid event at line {number}: {error}"))
+                })?;
+                observe(event)?;
+            }
+            Ok(())
+        })();
+        let unlocked = FileExt::unlock(&file);
+        result.and(unlocked)
+    }
+
+    pub fn events(&self) -> io::Result<Vec<Event>> {
+        let mut events = Vec::new();
+        self.each_event(|event| { events.push(event); Ok(()) })?;
+        Ok(events)
+    }
+
+    /// Stream event totals without allocating a full history vector.
     pub fn totals(&self) -> io::Result<(u64, u64)> {
-        let mut raw = 0;
-        let mut presented = 0;
-        for line in self.read()? {
-            let Some(raw_value) = json_number(&line, "raw_tokens") else { continue };
-            let Some(presented_value) = json_number(&line, "presented_tokens") else { continue };
-            raw += raw_value;
-            presented += presented_value;
-        }
+        let (mut raw, mut presented) = (0u64, 0u64);
+        self.each_event(|event| {
+            let Event::Savings(savings) = event;
+            raw = raw.checked_add(u64::from(savings.raw_tokens)).ok_or_else(|| io::Error::other("raw total overflow"))?;
+            presented = presented.checked_add(u64::from(savings.presented_tokens)).ok_or_else(|| io::Error::other("presented total overflow"))?;
+            Ok(())
+        })?;
         Ok((raw, presented))
     }
-    pub fn read(&self) -> io::Result<Vec<String>> { BufReader::new(fs::File::open(&self.path)?).lines().collect() }
-}
 
-fn json_number(line: &str, key: &str) -> Option<u64> {
-    let marker = format!("\"{key}\":");
-    let start = line.find(&marker)? + marker.len();
-    line[start..].split(|character: char| !character.is_ascii_digit()).next()?.parse().ok()
+    pub fn read(&self) -> io::Result<Vec<String>> {
+        let mut lines = Vec::new();
+        self.each_event(|event| {
+            lines.push(serde_json::to_string(&event).map_err(io::Error::other)?);
+            Ok(())
+        })?;
+        Ok(lines)
+    }
+
+    /// Explicitly discard only a final unterminated record after validating
+    /// that all preceding complete records remain valid. Never repairs a
+    /// completed malformed line or infers missing accounting data.
+    pub fn repair_partial_tail(&self) -> io::Result<bool> {
+        let mut file = OpenOptions::new().read(true).write(true).open(&self.path)?;
+        FileExt::lock_exclusive(&file)?;
+        let result = (|| {
+            let length = file.metadata()?.len();
+            if length > MAX_LOG_BYTES {
+                return Err(io::Error::new(io::ErrorKind::InvalidData, "session log exceeds 64 MiB limit"));
+            }
+            file.seek(SeekFrom::Start(0))?;
+            let mut bytes = Vec::new();
+            file.read_to_end(&mut bytes)?;
+            if bytes.is_empty() || bytes.last() == Some(&b'\n') { return Ok(false); }
+            let prefix_end = bytes.iter().rposition(|byte| *byte == b'\n')
+                .map_or(0, |index| index + 1);
+            if prefix_end != 0 {
+                // Exclude only the final terminator, not blank middle records.
+                for (index, line) in bytes[..prefix_end - 1].split(|byte| *byte == b'\n').enumerate() {
+                    if line.len() + 1 > MAX_RECORD_BYTES || serde_json::from_slice::<Event>(line).is_err() {
+                        return Err(io::Error::new(io::ErrorKind::InvalidData,
+                            format!("refusing to discard tail because preceding record {} is invalid", index + 1)));
+                    }
+                }
+            }
+            file.set_len(prefix_end as u64)?;
+            file.sync_all()?;
+            Ok(true)
+        })();
+        let unlocked = FileExt::unlock(&file);
+        unlocked?;
+        result
+    }
+
 }
-fn escape(value: &str) -> String { value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n") }
 
 #[cfg(test)]
 mod tests {
     use super::*;
+    use std::sync::atomic::{AtomicU64, Ordering};
+    static NEXT: AtomicU64 = AtomicU64::new(0);
+
+    fn path(label: &str) -> PathBuf {
+        std::env::temp_dir().join(format!(
+            "conduit-scope-{label}-{}-{}.jsonl",
+            std::process::id(),
+            NEXT.fetch_add(1, Ordering::Relaxed)
+        ))
+    }
+
+    #[test]
+    fn sums_typed_events() {
+        let path = path("totals");
+        let log = EventLog::open(&path).unwrap();
+        log.append(Event::Savings(Savings { module: "lens".into(), raw_tokens: 100, presented_tokens: 40, measured: true })).unwrap();
+        log.append(Event::Savings(Savings { module: "atlas".into(), raw_tokens: 20, presented_tokens: 20, measured: false })).unwrap();
+        assert_eq!(log.totals().unwrap(), (120, 60));
+        assert_eq!(log.events().unwrap().len(), 2);
+        let _ = fs::remove_file(path);
+    }
+
+    #[test]
+    fn preserves_modules_with_json_metacharacters() {
+        let path = path("escape");
+        let log = EventLog::open(&path).unwrap();
+        let event = Event::Savings(Savings { module: "a\"b\n\\\tc".into(), raw_tokens: 1, presented_tokens: 1, measured: false });
+        log.append(event.clone()).unwrap();
+        assert_eq!(log.events().unwrap(), vec![event]);
+        let _ = fs::remove_file(path);
+    }
+
+    #[test]
+    fn rejects_malformed_or_partial_events_instead_of_silent_undercount() {
+        let path = path("malformed");
+        let log = EventLog::open(&path).unwrap();
+        fs::write(&path, b"{\"type\":\"savings\",\"raw_tokens\":4}\n").unwrap();
+        assert_eq!(log.totals().unwrap_err().kind(), io::ErrorKind::InvalidData);
+        let _ = fs::remove_file(path);
+    }
+
+    #[test]
+    fn independent_instances_serialize_concurrent_appends() {
+        let path = path("concurrent");
+        EventLog::open(&path).unwrap();
+        let writers: Vec<_> = (0..4).map(|_| {
+            let path = path.clone();
+            std::thread::spawn(move || {
+                let log = EventLog::open(path).unwrap();
+                for _ in 0..25 {
+                    log.append(Event::Savings(Savings { module: "test".into(), raw_tokens: 2, presented_tokens: 1, measured: false })).unwrap();
+                }
+            })
+        }).collect();
+        for writer in writers { writer.join().unwrap(); }
+        assert_eq!(EventLog::open(&path).unwrap().totals().unwrap(), (200, 100));
+        let _ = fs::remove_file(path);
+    }
+}
+
+#[cfg(test)]
+mod recovery_tests {
+    use super::*;
+
     #[test]
-    fn appends_and_replays_events() { let path = std::env::temp_dir().join(format!("conduit-scope-{}.jsonl", std::process::id())); let log = EventLog::open(&path).unwrap(); log.append(Event::Savings(Savings { module: "lens".into(), raw_tokens: 100, presented_tokens: 40, measured: false })).unwrap(); let lines = log.read().unwrap(); assert_eq!(lines.len(), 1); assert!(lines[0].contains("\"raw_tokens\":100")); let _ = fs::remove_file(path); }
+    fn torn_tail_blocks_appends_and_requires_explicit_repair() {
+        let path = std::env::temp_dir().join(format!("conduit-torn-session-{}.jsonl", std::process::id()));
+        let log = EventLog::open(&path).unwrap();
+        let event = Event::Savings(Savings { module: "source".into(), raw_tokens: 2, presented_tokens: 1, measured: false });
+        log.append(event.clone()).unwrap();
+        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
+        file.write_all(b"{\"type\":\"sav").unwrap();
+        assert_eq!(log.totals().unwrap_err().kind(), io::ErrorKind::InvalidData);
+        assert_eq!(log.append(event.clone()).unwrap_err().kind(), io::ErrorKind::InvalidData);
+        assert!(log.repair_partial_tail().unwrap());
+        assert!(!log.repair_partial_tail().unwrap());
+        log.append(event).unwrap();
+        assert_eq!(log.totals().unwrap(), (4, 2));
+        let _ = fs::remove_file(path);
+    }
 
- { let path = std::env::temp_dir().join(format!("conduit-scope-totals-{}.jsonl", std::process::id())); let log = EventLog::open(&path).unwrap(); log.append(Event::Savings(Savings { module: "lens".into(), raw_tokens: 100, presented_tokens: 40, measured: true })).unwrap(); log.append(Event::Savings(Savings { module: "atlas".into(), raw_tokens: 20, presented_tokens: 20, measured: false })).unwrap(); assert_eq!(log.totals().unwrap(), (120, 60)); let _ = fs::remove_file(path); }
+    #[test]
+    fn invalid_complete_record_is_not_silently_repaired() {
+        let path = std::env::temp_dir().join(format!("conduit-corrupt-session-{}.jsonl", std::process::id()));
+        let log = EventLog::open(&path).unwrap();
+        fs::write(&path, b"not-json\ntruncated").unwrap();
+        assert!(log.repair_partial_tail().is_err());
+        assert!(log.totals().is_err());
+        let _ = fs::remove_file(path);
+    }
 
     #[test]
-    fn escapes_module_names_without_breaking_record_shape() { let path = std::env::temp_dir().join(format!("conduit-scope-escape-{}.jsonl", std::process::id())); let log = EventLog::open(&path).unwrap(); log.append(Event::Savings(Savings { module: "a\"b\nc".into(), raw_tokens: 1, presented_tokens: 1, measured: false })).unwrap(); assert_eq!(log.totals().unwrap(), (1, 1)); let _ = fs::remove_file(path); }
-}
\ No newline at end of file
+    fn rejects_large_or_empty_event_module() {
+        let path = std::env::temp_dir().join(format!("conduit-module-session-{}.jsonl", std::process::id()));
+        let log = EventLog::open(&path).unwrap();
+        for module in ["".to_owned(), "x".repeat(MAX_MODULE_BYTES + 1)] {
+            assert!(log.append(Event::Savings(Savings {
+                module, raw_tokens: 1, presented_tokens: 1, measured: false,
+            })).is_err());
+        }
+        assert_eq!(log.totals().unwrap(), (0, 0));
+        let _ = fs::remove_file(path);
+    }
+}
diff --git a/crates/store/src/lib.rs b/crates/store/src/lib.rs
index a6326c0..5d04514 100644
--- a/crates/store/src/lib.rs
+++ b/crates/store/src/lib.rs
@@ -1,58 +1,238 @@
 use blake3::Hasher;
-use std::{fs, io, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
+use std::{
+    fs::{self, File, OpenOptions},
+    io::{self, Write},
+    path::{Path, PathBuf},
+    sync::atomic::{AtomicU64, Ordering},
+    time::{SystemTime, UNIX_EPOCH},
+};
+
+const MAX_BLOB_BYTES: usize = 64 * 1024 * 1024;
+static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
 
 #[derive(Debug, Clone, PartialEq, Eq)]
-pub struct BlobRef { pub handle: String, pub hash: String, pub bytes: u64 }
+pub struct BlobRef {
+    pub handle: String,
+    pub hash: String,
+    pub bytes: u64,
+}
+
+pub struct BlobStore {
+    root: PathBuf,
+}
+
+#[derive(Debug, Default)]
+pub struct Audit {
+    pub checked_blobs: u64,
+    pub verified_bytes: u64,
+    pub staging_files: u64,
+    pub problem_count: u64,
+    /// The first 32 problems. This list is intentionally bounded.
+    pub problems: Vec<String>,
+}
+
+impl Audit {
+    fn problem(&mut self, description: String) {
+        self.problem_count = self.problem_count.saturating_add(1);
+        if self.problems.len() < 32 { self.problems.push(description); }
+    }
+}
 
-pub struct BlobStore { root: PathBuf }
 
 impl BlobStore {
     pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
         let root = root.as_ref().to_path_buf();
-        fs::create_dir_all(&root)?;
+        private_directory(&root)?;
         Ok(Self { root })
     }
 
     pub fn put(&self, data: &[u8]) -> io::Result<BlobRef> {
-        let mut hasher = Hasher::new();
-        hasher.update(data);
-        let hash = hasher.finalize().to_hex().to_string();
-        let path = self.root.join(&hash[0..2]).join(&hash);
-        if !path.exists() {
-            fs::create_dir_all(path.parent().expect("hash path has parent"))?;
-            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
-            let temp = path.with_extension(format!("tmp-{}-{}", std::process::id(), nonce));
-            fs::write(&temp, data)?;
-            if let Err(error) = fs::rename(&temp, &path) {
-                let _ = fs::remove_file(&temp);
-                if !path.exists() { return Err(error); }
+        if data.len() > MAX_BLOB_BYTES {
+            return Err(invalid("blob exceeds the 64 MiB store limit"));
+        }
+        let hash = digest(data);
+        let directory = self.root.join(&hash[..2]);
+        private_directory(&directory)?;
+        let path = directory.join(&hash);
+        if path.exists() {
+            // An existing digest name is not proof of intact content.
+            verify_blob(&path, &hash)?;
+        } else {
+            self.publish(&directory, &path, data)?;
+            verify_blob(&path, &hash)?;
+        }
+        Ok(BlobRef {
+            handle: handle_for_hash(&hash),
+            hash,
+            bytes: data.len() as u64,
+        })
+    }
+
+    fn publish(&self, directory: &Path, final_path: &Path, data: &[u8]) -> io::Result<()> {
+        let nonce = SystemTime::now()
+            .duration_since(UNIX_EPOCH)
+            .unwrap_or_default()
+            .as_nanos();
+        let mut opened = None;
+        for _ in 0..8 {
+            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
+            let temp = directory.join(format!(
+                ".conduit-{}-{nonce}-{serial}.tmp",
+                std::process::id()
+            ));
+            match OpenOptions::new().write(true).create_new(true).open(&temp) {
+                Ok(file) => {
+                    opened = Some((temp, file));
+                    break;
+                }
+                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
+                Err(error) => return Err(error),
             }
         }
-        Ok(BlobRef { handle: handle_for_hash(&hash), hash, bytes: data.len() as u64 })
+        let (temp, mut file) = opened.ok_or_else(|| invalid("unable to create unique blob staging file"))?;
+        let result = (|| {
+            private_file(&file)?;
+            file.write_all(data)?;
+            file.sync_all()?;
+            drop(file);
+            // Unlike rename, hard_link refuses to replace a concurrently committed blob.
+            match fs::hard_link(&temp, final_path) {
+                Ok(()) => {}
+                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
+                Err(error) => return Err(error),
+            }
+            sync_directory(directory)?;
+            Ok(())
+        })();
+        let _ = fs::remove_file(&temp);
+        result
     }
 
-    pub fn get(&self, handle: &str) -> io::Result<Vec<u8>> {
-        let hash = handle.strip_prefix("cx:o:").ok_or_else(|| invalid("invalid output handle"))?;
-        if hash.len() != 20 || !hash.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) { return Err(invalid("invalid output handle")); }
-        let mut matches = Vec::new();
-        for prefix in fs::read_dir(&self.root)? {
-            let prefix = prefix?.path();
-            if prefix.is_dir() {
-                for item in fs::read_dir(prefix)? {
-                    let item = item?.path();
-                    if item.file_name().and_then(|name| name.to_str()).map(|name| handle_for_hash(name) == handle).unwrap_or(false) { matches.push(item); }
+    /// Explicit on-demand integrity scan. Does not delete objects, follow
+    /// symlinks, or promise a stable snapshot during concurrent writers.
+    /// Runs in O(total stored bytes) time with bounded per-file memory.
+    pub fn audit(&self) -> io::Result<Audit> {
+        let mut report = Audit::default();
+        for item in fs::read_dir(&self.root)? {
+            let item = item?;
+            let name = item.file_name();
+            let Some(shard) = name.to_str() else {
+                report.problem("non-UTF8 shard directory".into());
+                continue;
+            };
+            if shard.len() != 2 || !is_hex(shard) || !item.file_type()?.is_dir() {
+                report.problem(format!("unexpected object in store root: {shard}"));
+                continue;
+            }
+            for entry in fs::read_dir(item.path())? {
+                let entry = entry?;
+                let name = entry.file_name();
+                let Some(name) = name.to_str() else {
+                    report.problem(format!("non-UTF8 blob name in {shard}"));
+                    continue;
+                };
+                if name.starts_with(".conduit-") && name.ends_with(".tmp") {
+                    report.staging_files = report.staging_files.saturating_add(1);
+                    continue;
+                }
+                if !is_full_hash(name) || !name.starts_with(shard) || !entry.file_type()?.is_file() {
+                    report.problem(format!("unexpected blob entry {shard}/{name}"));
+                    continue;
+                }
+                match verify_blob(&entry.path(), name) {
+                    Ok(bytes) => {
+                        report.checked_blobs = report.checked_blobs.saturating_add(1);
+                        report.verified_bytes = report.verified_bytes.saturating_add(bytes.len() as u64);
+                    }
+                    Err(error) => report.problem(format!("unverifiable blob {shard}/{name}: {error}")),
                 }
             }
         }
-        match matches.as_slice() { [path] => fs::read(path), [] => Err(io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown")), _ => Err(invalid("ambiguous handle")) }
+        Ok(report)
     }
+
+    pub fn get(&self, handle: &str) -> io::Result<Vec<u8>> {
+        let encoded = handle
+            .strip_prefix("cx:o:")
+            .ok_or_else(|| invalid("invalid output handle"))?;
+        let shard = shard_from_handle(encoded)?;
+        let directory = self.root.join(shard);
+        match fs::symlink_metadata(&directory) {
+            Ok(metadata) if metadata.file_type().is_dir() => {}
+            Ok(_) => return Err(invalid("blob shard is not a directory")),
+            Err(error) if error.kind() == io::ErrorKind::NotFound => {
+                return Err(io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown"));
+            }
+            Err(error) => return Err(error),
+        }
+        let mut match_path = None;
+        for entry in fs::read_dir(&directory)? {
+            let entry = entry?;
+            if !entry.file_type()?.is_file() {
+                continue;
+            }
+            let name = entry.file_name();
+            let Some(name) = name.to_str() else { continue };
+            // Strictly exclude staging files and arbitrarily named path aliases.
+            if !is_full_hash(name) || handle_for_hash(name) != handle {
+                continue;
+            }
+            if match_path.replace((entry.path(), name.to_owned())).is_some() {
+                return Err(invalid("ambiguous output handle"));
+            }
+        }
+        let (path, hash) = match_path.ok_or_else(|| {
+            io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown")
+        })?;
+        verify_blob(&path, &hash)
+    }
+}
+
+fn digest(data: &[u8]) -> String {
+    let mut hasher = Hasher::new();
+    hasher.update(data);
+    hasher.finalize().to_hex().to_string()
+}
+
+fn verify_blob(path: &Path, expected_hash: &str) -> io::Result<Vec<u8>> {
+    let metadata = fs::symlink_metadata(path)?;
+    if !metadata.file_type().is_file() || metadata.len() > MAX_BLOB_BYTES as u64 {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid stored blob type or size"));
+    }
+    let bytes = fs::read(path)?;
+    if digest(&bytes) != expected_hash {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "stored blob failed digest verification"));
+    }
+    Ok(bytes)
+}
+
+fn is_hex(name: &str) -> bool {
+    name.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
+}
+
+fn is_full_hash(name: &str) -> bool { name.len() == 64 && is_hex(name) }
+
+fn shard_from_handle(encoded: &str) -> io::Result<String> {
+    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
+    if encoded.len() != 20 || !encoded.bytes().all(|c| ALPHABET.contains(&c)) {
+        return Err(invalid("invalid output handle"));
+    }
+    let bytes = encoded.as_bytes();
+    let first = ALPHABET.iter().position(|c| *c == bytes[0]).expect("validated alphabet") as u8;
+    let second = ALPHABET.iter().position(|c| *c == bytes[1]).expect("validated alphabet") as u8;
+    let first_hash_byte = (first << 3) | (second >> 2);
+    Ok(format!("{first_hash_byte:02x}"))
+}
+
+fn invalid(message: &str) -> io::Error {
+    io::Error::new(io::ErrorKind::InvalidInput, message)
 }
 
-fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }
 fn handle_for_hash(hash: &str) -> String {
     let mut bytes = [0u8; 12];
     for (index, chunk) in hash.as_bytes().chunks(2).take(12).enumerate() {
-        bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("00"), 16).unwrap_or(0);
+        bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("00"), 16)
+            .unwrap_or(0);
     }
     format!("cx:o:{}", base32_prefix(&bytes))
 }
@@ -71,21 +251,153 @@ fn base32_prefix(bytes: &[u8]) -> String {
             buffer &= (1u32 << bits).saturating_sub(1);
         }
     }
-    if bits > 0 { out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char); }
+    if bits > 0 {
+        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
+    }
     out
 }
 
+fn private_directory(path: &Path) -> io::Result<()> {
+    fs::create_dir_all(path)?;
+    if !fs::symlink_metadata(path)?.file_type().is_dir() {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "blob directory is a symlink or non-directory"));
+    }
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
+    }
+    Ok(())
+}
+
+fn private_file(file: &File) -> io::Result<()> {
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        file.set_permissions(fs::Permissions::from_mode(0o600))?;
+    }
+    #[cfg(not(unix))]
+    let _ = file;
+    Ok(())
+}
+
+fn sync_directory(path: &Path) -> io::Result<()> {
+    #[cfg(unix)]
+    {
+        File::open(path)?.sync_all()?;
+    }
+    #[cfg(not(unix))]
+    let _ = path;
+    Ok(())
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
+
+    fn temp_root(name: &str) -> PathBuf {
+        std::env::temp_dir().join(format!(
+            "conduit-store-{name}-{}-{}",
+            std::process::id(),
+            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
+        ))
+    }
+
     #[test]
     fn stores_deduplicated_content_and_expands_handle() {
-        let dir = std::env::temp_dir().join(format!("conduit-store-{}", std::process::id()));
+        let dir = temp_root("dedup");
         let store = BlobStore::open(&dir).unwrap();
-        let first = store.put(b"same").unwrap(); let second = store.put(b"same").unwrap();
-        assert_eq!(first, second); assert_eq!(store.get(&first.handle).unwrap(), b"same");
+        let first = store.put(b"same").unwrap();
+        let second = store.put(b"same").unwrap();
+        assert_eq!(first, second);
+        assert_eq!(store.get(&first.handle).unwrap(), b"same");
         let _ = fs::remove_dir_all(dir);
     }
+
     #[test]
-    fn rejects_non_output_handles() { let dir = std::env::temp_dir().join("conduit-store-invalid"); let store = BlobStore::open(&dir).unwrap(); assert!(store.get("nope").is_err()); let _ = fs::remove_dir_all(dir); }
+    fn excludes_staging_files_from_handle_lookup() {
+        let dir = temp_root("staging");
+        let store = BlobStore::open(&dir).unwrap();
+        let blob = store.put(b"content").unwrap();
+        let stage = dir.join(&blob.hash[..2]).join(format!("{}.tmp-race", blob.hash));
+        fs::write(stage, b"partial").unwrap();
+        assert_eq!(store.get(&blob.handle).unwrap(), b"content");
+        let _ = fs::remove_dir_all(dir);
+    }
+
+    #[test]
+    fn rejects_corrupted_existing_blob_and_expansion() {
+        let dir = temp_root("corrupt");
+        let store = BlobStore::open(&dir).unwrap();
+        let blob = store.put(b"original").unwrap();
+        fs::write(dir.join(&blob.hash[..2]).join(&blob.hash), b"tampered").unwrap();
+        assert_eq!(store.get(&blob.handle).unwrap_err().kind(), io::ErrorKind::InvalidData);
+        assert!(store.put(b"original").is_err());
+        let _ = fs::remove_dir_all(dir);
+    }
+
+    #[test]
+    fn rejects_non_output_handles() {
+        let dir = temp_root("invalid");
+        let store = BlobStore::open(&dir).unwrap();
+        for bad in ["nope", "cx:o:../../xx", "cx:o:aaaaaaaaaaaaaaa00000"] {
+            assert!(store.get(bad).is_err());
+        }
+        let _ = fs::remove_dir_all(dir);
+    }
+
+    #[test]
+    fn concurrently_commits_identical_content() {
+        let dir = temp_root("concurrent");
+        let store = std::sync::Arc::new(BlobStore::open(&dir).unwrap());
+        let threads: Vec<_> = (0..8)
+            .map(|_| {
+                let store = store.clone();
+                std::thread::spawn(move || store.put(b"shared content").unwrap())
+            })
+            .collect();
+        let blobs: Vec<_> = threads.into_iter().map(|thread| thread.join().unwrap()).collect();
+        assert!(blobs.iter().all(|blob| blob == &blobs[0]));
+        assert_eq!(store.get(&blobs[0].handle).unwrap(), b"shared content");
+        let _ = fs::remove_dir_all(dir);
+    }
+}
+
+#[cfg(test)]
+mod audit_tests {
+    use super::*;
+    use std::sync::atomic::{AtomicU64, Ordering};
+    static NEXT: AtomicU64 = AtomicU64::new(0);
+
+    #[test]
+    fn audit_detects_corruption_without_deleting_or_rewriting_objects() {
+        let root = std::env::temp_dir().join(format!("conduit-blob-audit-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
+        let store = BlobStore::open(&root).unwrap();
+        let valid = store.put(b"good blob").unwrap();
+        let invalid = store.put(b"different blob").unwrap();
+        let before = store.audit().unwrap();
+        assert_eq!(before.checked_blobs, 2);
+        assert_eq!(before.problem_count, 0);
+        assert_eq!(before.verified_bytes, 23); // 9 + 14
+        fs::write(root.join(&invalid.hash[..2]).join(&invalid.hash), b"changed").unwrap();
+        let after = store.audit().unwrap();
+        assert_eq!(after.checked_blobs, 1);
+        assert_eq!(after.problem_count, 1);
+        assert!(store.get(&valid.handle).is_ok());
+        assert!(store.get(&invalid.handle).is_err());
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn audit_counts_staging_without_following_fake_entries() {
+        let root = std::env::temp_dir().join(format!("conduit-blob-stage-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
+        let store = BlobStore::open(&root).unwrap();
+        let valid = store.put(b"x").unwrap();
+        fs::write(root.join(&valid.hash[..2]).join(".conduit-123-456.tmp"), b"partial").unwrap();
+        let audit = store.audit().unwrap();
+        assert_eq!(audit.checked_blobs, 1);
+        assert_eq!(audit.staging_files, 1);
+        assert_eq!(audit.problem_count, 0);
+        let _ = fs::remove_dir_all(root);
+    }
 }
diff --git a/docs/wiki/architecture.md b/docs/wiki/architecture.md
index 0f2d0c9..d409a50 100644
--- a/docs/wiki/architecture.md
+++ b/docs/wiki/architecture.md
@@ -1,42 +1,29 @@
-# Architecture
+# Architecture — implemented behavior versus proposed planes
 
-Conduit is organized as deterministic local libraries behind a thin CLI. The intended production daemon can later expose the same contracts over IPC without changing the core safety rules.
+Conduit currently is a local CLI and independent Rust library crates. The following is the **implemented pipeline** for `run --compact`:
 
-## Data flow
+1. Accept caller-provided executable path and arguments (no policy enforcement; process inherits caller privileges).
+2. Run the direct child with a duration limit; drain stdout and stderr with independent capture limits.
+3. If capture overflows, report incomplete output and exit with an error (no lossless expansion available).
+4. If the child exits successfully with valid text stdout, attempt a diagnostic-preserving reduction.
+5. Estimate hint-inclusive reduction, then persist original stdout to an integrity-checked BLAKE3 store before displaying a shorter body.
+6. Emit the expansion hint on stderr. If persisting fails, pass stdout through unchanged.
+7. If `CONDUIT_SESSION` is set, record a JSONL word-estimate event (not actual model tokens).
 
-1. A command is authorized by policy.
-2. The child runs with its exit status preserved.
-3. Raw bytes are captured and stored by content hash.
-4. The lens classifies and reduces output while retaining decision-relevant signals.
-5. The presented result includes a handle if anything was omitted.
-6. Scope records accounting and replay events.
-7. Atlas and ledger provide hash-aware retrieval and context state.
-8. Edit and verify operate only on fresh, bounded inputs.
+`find` now scans bounded files without retaining a repository-wide snapshot; the library still exposes a snapshot API. `peek` is a bounded hash-anchored read. Neither implements syntax trees or a persistent index. `edit` and `batch-edit` share preflight validation and perform per-file staged renames with cooperative locking, but not crash-atomic transactions across files. `doctor` verifies storage integrity on demand; `repair-session` safely truncates only a torn final JSONL record after validating the preceding records.
 
-## Module responsibilities
+The following crates exist but are **not integrated with CLI dispatch**: `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, `fanout`. The protobuf schema is not generated or consumed by the CLI. No OS sandbox, PTY, provider gateway, daemon or distributed state plane is available.
 
-- **Store:** immutable blobs, deduplication, expansion handles.
-- **Scope:** append-only event records and typed totals.
-- **Lens:** output presentation; it must never alter child behavior.
-- **Atlas:** coherent read snapshot with file hashes.
-- **Edit:** stale-read rejection and all-or-nothing staged writes.
-- **Ledger:** live/stubbed/evicted context lifecycle and pinned items.
-- **Relay:** token, money, and wall-clock authorization.
-- **Guard:** repeated action and consecutive failure detection.
-- **Verify:** impacted-test selection with widening when coverage is incomplete.
-- **Vault:** command, path, and workspace authorization.
-- **Memory:** facts bound to source hashes.
-- **Fanout:** bounded parallel execution with stable result ordering.
+## Target architecture (proposal only)
 
-## Invariants
+```text
+agent adapters / typed tool requests
+    -> authorization and capability reservation [not implemented]
+    -> supervised execution with bounded raw spool [partially implemented]
+    -> verified content store and output adapters [partially implemented]
+    -> correlated accounting and deterministic retrieval [partially implemented]
+    -> version-checked edit and recovery journal [not implemented]
+    -> evaluation/promotion gates and optional memory [not implemented]
+```
 
-- A failed optimizer returns raw behavior where possible.
-- A handle never substitutes for the underlying retained content without an expansion path.
-- A hash mismatch prevents an edit or stale fact from being used.
-- A budget check fails closed before a bounded operation proceeds.
-- Parallel execution may change completion order, never returned order.
-- Policy decisions occur before side effects.
-
-## Deliberate boundaries
-
-The current implementation does not pretend to be a full LSP/SCIP index, provider gateway, OS sandbox, or PTY multiplexer. Those require separate contracts, platform tests, and threat-model review; the current crates are deterministic foundations rather than fake implementations of those systems.
+Key requirements before any production trust claim are independent operating-system containment for hostile commands, typed non-bypassable authorization, explicit resource exhaustion behavior, transactional recovery, privacy/retention controls, and live-agent non-inferiority experiments.
diff --git a/docs/wiki/batch-edit.md b/docs/wiki/batch-edit.md
new file mode 100644
index 0000000..a5ac9dc
--- /dev/null
+++ b/docs/wiki/batch-edit.md
@@ -0,0 +1,62 @@
+# Hash-anchored batch edits
+
+Conduit accepts a JSON manifest containing a bounded list of source edits. The
+manifest itself is data, not shell code. No program or tool is executed by this
+operation. Relative paths resolve under `CONDUIT_WORKSPACE` (default `.`).
+
+## Manifest
+
+```json
+{
+  "operations": [
+    {
+      "type": "search_replace",
+      "path": "src/main.rs",
+      "expected_hash": "<entire 64-character BLAKE3 hash from conduit peek>",
+      "old": "old code fragment",
+      "new": "replacement code fragment"
+    },
+    {
+      "type": "replace_range",
+      "path": "src/lib.rs",
+      "expected_hash": "<hash from conduit peek>",
+      "start": 0,
+      "end": 5,
+      "new": "hello"
+    }
+  ]
+}
+```
+
+`start` and `end` are **0-based UTF-8 byte offsets** in the original source,
+with an exclusive end. `search_replace` requires exactly one occurrence of the
+`old` string. There can be only one edit per canonical target in a batch;
+combine adjacent changes into one updated range when necessary. Manifest
+placeholders are not valid hashes and must be replaced before execution.
+
+```sh
+export CONDUIT_WORKSPACE="$PWD"
+conduit peek src/main.rs 1 40
+conduit batch-edit --check /tmp/reviewed-change.json
+conduit batch-edit /tmp/reviewed-change.json
+```
+
+## Guarantees and limits
+
+- `--check` and a real commit share target/path/hash/size validation.
+- Preflight does **not** reserve a target against modifications by other tools.
+- Batch size: 1–128 operations; each source/replacement <=8 MiB; total input
+  plus replacement payload <=64 MiB; JSON manifest <=1 MiB.
+- Rejects paths outside the workspace and edits targeting `.git` or `.conduit`.
+- Rejects duplicate canonical targets and hardlinked files on Unix, because
+  the rename implementation would otherwise silently separate aliases.
+- Staging uses same-directory files and a process-lifetime cooperative OS lock;
+  per-file source hashes are rechecked before replacement.
+- Multi-file all-or-nothing power-loss/crash recovery is **not** implemented.
+  Even successful dry-run checks do not change that limitation.
+- Concurrent uncooperative external writers, symbolic-link races and filesystem
+  semantics remain potential threats. This tool is not a filesystem sandbox.
+
+Do not use the batch API for untrusted source writes or multi-file changes
+requiring transactional crash recovery. For such operations use a versioned
+source-control workflow and a supervisory transaction/recovery mechanism.
diff --git a/docs/wiki/cli-reference.md b/docs/wiki/cli-reference.md
index 807fbdf..61503f9 100644
--- a/docs/wiki/cli-reference.md
+++ b/docs/wiki/cli-reference.md
@@ -1,38 +1,42 @@
-# CLI reference
+# CLI reference (implemented commands)
 
-## `run -- <command> [args...]`
+## `run [--preset safe|quick|deep|compact] [--timeout-ms N] [--max-output-bytes N] [--budget N] [--compact] -- <command> [args...]`
 
-Runs a child process, captures output, stores raw bytes when possible, and presents reduced output. The child exit code is returned unchanged. Interactive programs and exact stdout/stderr temporal interleaving are not yet fully supported by the current `Command::output` boundary.
+Starts a direct child process with the caller's permissions and environment. Captures stdout and stderr independently up to an explicit byte limit and drains excess bytes. Never treats truncated output as complete. Timeout terminates the direct child only, and descendants may outlive it. Output is buffered, so interactive PTYs and temporal interleaving are not supported. The `vault` policy crate does not authorize this command; `run` is **not a sandbox**.
 
-## `find <query> [limit]`
+Only `--compact` or the `compact` preset enables conservative, UTF-8 stdout-only reduction on successful commands. If doing so saves no net words once the recovery hint is included, the original stdout passes through without storing a blob. Otherwise a verified content-addressed blob contains the original stdout and an expansion hint is written to stderr. Neither the capture limit nor an unavailable store allows a false claim of lossless recovery.
 
-Searches a deterministic workspace snapshot. Each hit contains a relative path, one-based line number, source line, and content hash. Empty queries return no results. `CONDUIT_WORKSPACE` selects the root.
+Presets: `safe` (120 seconds, 8 MiB per stream, no compaction); `quick` (30 seconds, 4 MiB, no compaction); `deep` (600 seconds, 32 MiB, no compaction); `compact` (120 seconds, 8 MiB, with reduction). Explicit flags override a preset regardless of their relative order.
 
-## `edit <path> <expected-hash> <old> <new>`
+## `find <query> [limit]` and `peek <relative-path> [start-line] [count]`
 
-Performs a unique search/replace only if the complete file hash matches. Parent traversal, absolute paths, symlink escapes, ambiguous matches, and duplicate transaction paths are rejected. Edits use a workspace lock and staged replacement.
+`find` scans UTF-8 source file-by-file with ignore rules; it does not materialize a whole-repository snapshot. Search stops after 10,000 text files or 64 MiB scanned, skips files larger than 1 MiB, and shows bounded match-centered excerpts. Filesystem enumeration order is not a relevance ranking and can differ between platforms. `hit_limit_reached` and `truncated` are separate flags. Results may also be incomplete because of ignore rules or skipped entries. The query must be nonblank and at most 512 bytes; the optional hit limit is 1–500. `peek` prints 1–200 lines from one bounded text file with its hash for inspect→edit workflows. It refuses traversal or secret-directory paths. Neither performs AST parsing.
 
-## `store <text>` and `expand <handle>`
+## `edit <relative-path> <expected-hash> <old> <new>` and `batch-edit [--check] <manifest.json>`
 
-Stores text and prints an output handle, or retrieves a previously stored blob. Expired or unknown handles fail clearly; output handles are not guessed.
+Hash-anchored source edits. `batch-edit` accepts `search_replace` and `replace_range` operations with per-target BLAKE3 preconditions; `--check` validates all operations without changing source content. A plan is not reserved between the preflight and commit. The shared validator blocks path escapes, protected metadata, stale hashes, duplicate canonical targets, hardlinked files, oversized inputs/outputs, and ambiguous matches. Limits: 1–128 operations, 8 MiB per target, 64 MiB combined input/output, and a 1 MiB manifest. Uses a process-lifetime cooperative lock under the workspace `.conduit` directory and staged file replacement. Multi-file crash atomicity and safety from uncooperative writers are not provided. See [batch edit](batch-edit.md).
 
-## `record`, `replay`, `stats`
+## `store <text>|-` and `expand <handle>`
 
-Record a session accounting event, print its append-only events, or compute typed token totals:
+Persists bytes in a local BLAKE3-addressed store or recovers them by handle. `store -` reads up to 64 MiB from stdin, without exposing content in command-line arguments. Retrieval verifies the full digest. Handles are not access-control credentials. There is no automatic retention or deletion.
 
-```sh
-conduit record demo lens 1000 400
-conduit stats demo
-conduit replay demo
-```
+## `doctor`
 
-## `bench`
+Read-only integrity scan of the local blob store. Reports verified objects/bytes, observed staging files, and detected errors (up to 32 examples), failing with a nonzero exit code for integrity problems. The scan does not delete blobs or promise an atomic snapshot while other processes write.
 
-Runs the small local corpus smoke check. The authoritative offline benchmark is `python3 eval/benchmark.py`.
+## `record`, `replay`, `stats`, `repair-session`
+
+Writes and reads synchronized JSONL session events. Session identifiers are validated. Records are bounded to 8 KiB; each session log is limited to 64 MiB. `stats` calculates totals from a streaming, validated traversal. An incomplete last record fails closed. `repair-session` explicitly removes only an unterminated final record, after validating the preceding records. It does not reconstruct missing events or repair malformed complete lines. Accounting is a caller-provided or whitespace-word estimate, **not tokenizer-derived usage**.
+
+## `bench [file] [budget]`
+
+Exercises the CLI's compaction/persistence/expansion path against an input text file with temporary state; prints effective shown-word estimates including hint overhead. Does not benchmark subprocess capture, actual provider tokenization, agent task success, or real-world latency.
 
 ## Environment
 
-- `CONDUIT_HOME`: blob and session root.
-- `CONDUIT_WORKSPACE`: atlas/edit root.
+- `CONDUIT_HOME`: storage and sessions root; default `.conduit`.
+- `CONDUIT_WORKSPACE`: root for `find`, `peek`, `edit`, and `batch-edit`; default current directory.
+- `CONDUIT_SESSION`: optional session name for accounting of successful compacted observations.
+- `CONDUIT_DEBUG_TIMING`: when set, emits child execution wall time.
 
-Never pass secrets as command-line arguments; process listings may expose them.
+Use OS-level isolation for untrusted commands, and avoid passing secrets as arguments because local process listings may expose them.
diff --git a/docs/wiki/operations.md b/docs/wiki/operations.md
index d731863..008060b 100644
--- a/docs/wiki/operations.md
+++ b/docs/wiki/operations.md
@@ -6,11 +6,13 @@ Blob storage is content-addressed and deduplicated. Session logs are append-only
 
 ## Diagnostics
 
+Run `conduit doctor` to verify stored blob contents and identify unexpected shard entries. `doctor` reads object contents, so restrict access to authorized operators. It is read-only and neither deletes staging files nor races-freezes concurrently published objects. Run `conduit stats SESSION` to validate session accounting without building an in-memory list of all events. `repair-session SESSION` is explicitly destructive to a torn final record only; preserve a backup of a damaged log if forensic reconstruction matters.
+
 When behavior is unexpected, collect the command, profile, exit code, presented output, and handle—not secrets or the full raw blob by default. Confirm the workspace and `CONDUIT_HOME` values first. Re-run with the raw handle only when necessary.
 
 ## Recovery
 
-A missing or expired handle requires rerunning the command. A stale edit must be regenerated from a fresh atlas hit. A held edit lock should be investigated before removal; deleting it while another process is active can allow concurrent writes.
+A missing or expired handle requires rerunning the command. A stale edit must be regenerated from a fresh atlas hit. The edit lock is held by the OS for the process lifetime and released automatically on process exit; do not delete the file while edits are active because that could create separate lock inodes. For multi-file batches, an interrupted commit may leave a partially applied set: `batch-edit --check` does not constitute crash recovery. Preserve original files/backups and reconcile hashes before retrying. A torn session tail requires explicit `repair-session` after evidence preservation.
 
 ## Release checklist
 
diff --git a/eval/benchmark.py b/eval/benchmark.py
index 4ab7e62..2d78da5 100644
--- a/eval/benchmark.py
+++ b/eval/benchmark.py
@@ -1,30 +1,75 @@
 #!/usr/bin/env python3
-"""Deterministic Phase 0 corpus benchmark; external agents are intentionally optional."""
+"""Exercise the Rust reducer on a real corpus, not a Python imitation.
+
+This is a word-count regression gate. It does not measure model token usage,
+agent task success, or end-to-end latency. Those require paired trace studies.
+"""
+import argparse
+import csv
+import os
 from pathlib import Path
-import re
+import subprocess
+import sys
+
+CORPUS = Path(__file__).resolve().parent / "corpus"
 
-ROOT = Path(__file__).parent
-CORPUS = ROOT / "corpus"
-ERROR = re.compile(r"error|failed|panic|fatal|traceback|exception|FAILED", re.I)
 
-def tokens(text: str) -> int:
-    return len(re.findall(r"\S+", text))
+def parse_measurement(stdout: str) -> tuple[int, int, bool]:
+    fields = {}
+    for item in stdout.strip().split():
+        if "=" in item:
+            key, value = item.split("=", 1)
+            fields[key] = value
+    if fields.get("mode") != "rust_cli_effective_word_estimate":
+        raise ValueError("unexpected benchmark mode or incompatible Conduit binary")
+    raw = int(fields["raw_words"])
+    shown = int(fields["presented_words"])
+    if raw <= 0 or shown < 0 or shown > raw:
+        raise ValueError("invalid or empty raw/presented word counts")
+    if fields.get("compact") not in ("true", "false"):
+        raise ValueError("missing or invalid reduction flag")
+    if fields.get("raw_recoverable") != "true":
+        raise ValueError("raw bytes were not verified as recoverable")
+    if fields["compact"] == "false" and shown != raw:
+        raise ValueError("no compaction happened but savings were reported")
+    return raw, shown, fields["compact"] == "true"
 
-def reduce_output(text: str) -> str:
-    lines = text.splitlines()
-    retained = [line for line in lines if ERROR.search(line) or re.search(r"\b\d+ (passed|failed)\b|exit=", line, re.I)]
-    return "\n".join(dict.fromkeys(retained)) + ("\n" if retained else "")
 
-def main() -> None:
+def main() -> int:
+    parser = argparse.ArgumentParser(description=__doc__)
+    filename = "conduit.exe" if os.name == "nt" else "conduit"
+    parser.add_argument("--binary", type=Path, default=Path("target/debug") / filename)
+    parser.add_argument("--budget", type=int, default=700)
+    args = parser.parse_args()
+    if args.budget <= 0 or args.budget > 100_000:
+        parser.error("budget must be between 1 and 100000")
+    binary = args.binary.resolve()
+    if not binary.is_file():
+        parser.error(f"the compiled Conduit executable was not found: {binary}. Run cargo build --bin conduit")
+    paths = sorted(CORPUS.glob("*.log"))
+    if not paths:
+        parser.error("benchmark corpus is empty; no evidence was measured")
     rows = []
-    for path in sorted(CORPUS.glob("*.log")):
-        raw = path.read_text()
-        shown = reduce_output(raw)
-        raw_tokens, shown_tokens = tokens(raw), tokens(shown)
-        rows.append((path.name, raw_tokens, shown_tokens, round(1 - shown_tokens / raw_tokens, 4) if raw_tokens else 0))
-    print("corpus,raw_tokens,presented_tokens,reduction")
-    for row in rows:
-        print(",".join(map(str, row)))
+    for path in paths:
+        if not path.read_bytes().strip():
+            parser.error(f"empty benchmark file: {path}")
+        try:
+            result = subprocess.run([str(binary), "bench", str(path), str(args.budget)],
+                                    check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
+                                    text=True, timeout=30)
+            raw, shown, compact = parse_measurement(result.stdout)
+        except (OSError, ValueError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
+            parser.error(f"benchmark failed for {path.name}: {error}")
+        rows.append((path.name, raw, shown, raw - shown, compact, round(1 - shown / raw, 4)))
+    writer = csv.writer(sys.stdout)
+    writer.writerow(("corpus", "raw_words", "effective_shown_words", "saved_word_estimate", "compact", "reduction"))
+    writer.writerows(rows)
+    total_raw = sum(row[1] for row in rows)
+    total_shown = sum(row[2] for row in rows)
+    writer.writerow(("TOTAL", total_raw, total_shown, total_raw - total_shown, "mixed", round(1 - total_shown / total_raw, 4)))
+    print("evidence=rust_cli_presentation_path_including_recovery_hint_not_model_tokens_or_agent_outcomes", file=sys.stderr)
+    return 0
+
 
 if __name__ == "__main__":
-    main()
+    raise SystemExit(main())
diff --git a/eval/corpus/long-build.log b/eval/corpus/long-build.log
new file mode 100644
index 0000000..a88f139
--- /dev/null
+++ b/eval/corpus/long-build.log
@@ -0,0 +1,183 @@
+building crate_0000 source package assembled to intermediate target object
+building crate_0001 source package assembled to intermediate target object
+building crate_0002 source package assembled to intermediate target object
+building crate_0003 source package assembled to intermediate target object
+building crate_0004 source package assembled to intermediate target object
+building crate_0005 source package assembled to intermediate target object
+building crate_0006 source package assembled to intermediate target object
+building crate_0007 source package assembled to intermediate target object
+building crate_0008 source package assembled to intermediate target object
+building crate_0009 source package assembled to intermediate target object
+building crate_0010 source package assembled to intermediate target object
+building crate_0011 source package assembled to intermediate target object
+building crate_0012 source package assembled to intermediate target object
+building crate_0013 source package assembled to intermediate target object
+building crate_0014 source package assembled to intermediate target object
+building crate_0015 source package assembled to intermediate target object
+building crate_0016 source package assembled to intermediate target object
+building crate_0017 source package assembled to intermediate target object
+building crate_0018 source package assembled to intermediate target object
+building crate_0019 source package assembled to intermediate target object
+building crate_0020 source package assembled to intermediate target object
+building crate_0021 source package assembled to intermediate target object
+building crate_0022 source package assembled to intermediate target object
+building crate_0023 source package assembled to intermediate target object
+building crate_0024 source package assembled to intermediate target object
+building crate_0025 source package assembled to intermediate target object
+building crate_0026 source package assembled to intermediate target object
+building crate_0027 source package assembled to intermediate target object
+building crate_0028 source package assembled to intermediate target object
+building crate_0029 source package assembled to intermediate target object
+building crate_0030 source package assembled to intermediate target object
+building crate_0031 source package assembled to intermediate target object
+building crate_0032 source package assembled to intermediate target object
+building crate_0033 source package assembled to intermediate target object
+building crate_0034 source package assembled to intermediate target object
+building crate_0035 source package assembled to intermediate target object
+building crate_0036 source package assembled to intermediate target object
+building crate_0037 source package assembled to intermediate target object
+building crate_0038 source package assembled to intermediate target object
+building crate_0039 source package assembled to intermediate target object
+building crate_0040 source package assembled to intermediate target object
+building crate_0041 source package assembled to intermediate target object
+building crate_0042 source package assembled to intermediate target object
+building crate_0043 source package assembled to intermediate target object
+building crate_0044 source package assembled to intermediate target object
+building crate_0045 source package assembled to intermediate target object
+building crate_0046 source package assembled to intermediate target object
+building crate_0047 source package assembled to intermediate target object
+building crate_0048 source package assembled to intermediate target object
+building crate_0049 source package assembled to intermediate target object
+building crate_0050 source package assembled to intermediate target object
+building crate_0051 source package assembled to intermediate target object
+building crate_0052 source package assembled to intermediate target object
+building crate_0053 source package assembled to intermediate target object
+building crate_0054 source package assembled to intermediate target object
+building crate_0055 source package assembled to intermediate target object
+building crate_0056 source package assembled to intermediate target object
+building crate_0057 source package assembled to intermediate target object
+building crate_0058 source package assembled to intermediate target object
+building crate_0059 source package assembled to intermediate target object
+building crate_0060 source package assembled to intermediate target object
+building crate_0061 source package assembled to intermediate target object
+building crate_0062 source package assembled to intermediate target object
+building crate_0063 source package assembled to intermediate target object
+building crate_0064 source package assembled to intermediate target object
+building crate_0065 source package assembled to intermediate target object
+building crate_0066 source package assembled to intermediate target object
+building crate_0067 source package assembled to intermediate target object
+building crate_0068 source package assembled to intermediate target object
+building crate_0069 source package assembled to intermediate target object
+building crate_0070 source package assembled to intermediate target object
+building crate_0071 source package assembled to intermediate target object
+building crate_0072 source package assembled to intermediate target object
+building crate_0073 source package assembled to intermediate target object
+building crate_0074 source package assembled to intermediate target object
+building crate_0075 source package assembled to intermediate target object
+building crate_0076 source package assembled to intermediate target object
+building crate_0077 source package assembled to intermediate target object
+building crate_0078 source package assembled to intermediate target object
+building crate_0079 source package assembled to intermediate target object
+building crate_0080 source package assembled to intermediate target object
+building crate_0081 source package assembled to intermediate target object
+building crate_0082 source package assembled to intermediate target object
+building crate_0083 source package assembled to intermediate target object
+building crate_0084 source package assembled to intermediate target object
+building crate_0085 source package assembled to intermediate target object
+building crate_0086 source package assembled to intermediate target object
+building crate_0087 source package assembled to intermediate target object
+building crate_0088 source package assembled to intermediate target object
+building crate_0089 source package assembled to intermediate target object
+building crate_0090 source package assembled to intermediate target object
+building crate_0091 source package assembled to intermediate target object
+building crate_0092 source package assembled to intermediate target object
+building crate_0093 source package assembled to intermediate target object
+building crate_0094 source package assembled to intermediate target object
+building crate_0095 source package assembled to intermediate target object
+building crate_0096 source package assembled to intermediate target object
+building crate_0097 source package assembled to intermediate target object
+building crate_0098 source package assembled to intermediate target object
+building crate_0099 source package assembled to intermediate target object
+building crate_0100 source package assembled to intermediate target object
+building crate_0101 source package assembled to intermediate target object
+building crate_0102 source package assembled to intermediate target object
+building crate_0103 source package assembled to intermediate target object
+building crate_0104 source package assembled to intermediate target object
+building crate_0105 source package assembled to intermediate target object
+building crate_0106 source package assembled to intermediate target object
+building crate_0107 source package assembled to intermediate target object
+building crate_0108 source package assembled to intermediate target object
+building crate_0109 source package assembled to intermediate target object
+building crate_0110 source package assembled to intermediate target object
+building crate_0111 source package assembled to intermediate target object
+building crate_0112 source package assembled to intermediate target object
+building crate_0113 source package assembled to intermediate target object
+building crate_0114 source package assembled to intermediate target object
+building crate_0115 source package assembled to intermediate target object
+building crate_0116 source package assembled to intermediate target object
+building crate_0117 source package assembled to intermediate target object
+building crate_0118 source package assembled to intermediate target object
+building crate_0119 source package assembled to intermediate target object
+building crate_0120 source package assembled to intermediate target object
+building crate_0121 source package assembled to intermediate target object
+building crate_0122 source package assembled to intermediate target object
+building crate_0123 source package assembled to intermediate target object
+building crate_0124 source package assembled to intermediate target object
+building crate_0125 source package assembled to intermediate target object
+building crate_0126 source package assembled to intermediate target object
+building crate_0127 source package assembled to intermediate target object
+building crate_0128 source package assembled to intermediate target object
+building crate_0129 source package assembled to intermediate target object
+building crate_0130 source package assembled to intermediate target object
+building crate_0131 source package assembled to intermediate target object
+building crate_0132 source package assembled to intermediate target object
+building crate_0133 source package assembled to intermediate target object
+building crate_0134 source package assembled to intermediate target object
+building crate_0135 source package assembled to intermediate target object
+building crate_0136 source package assembled to intermediate target object
+building crate_0137 source package assembled to intermediate target object
+building crate_0138 source package assembled to intermediate target object
+building crate_0139 source package assembled to intermediate target object
+building crate_0140 source package assembled to intermediate target object
+building crate_0141 source package assembled to intermediate target object
+building crate_0142 source package assembled to intermediate target object
+building crate_0143 source package assembled to intermediate target object
+building crate_0144 source package assembled to intermediate target object
+building crate_0145 source package assembled to intermediate target object
+building crate_0146 source package assembled to intermediate target object
+building crate_0147 source package assembled to intermediate target object
+building crate_0148 source package assembled to intermediate target object
+building crate_0149 source package assembled to intermediate target object
+building crate_0150 source package assembled to intermediate target object
+building crate_0151 source package assembled to intermediate target object
+building crate_0152 source package assembled to intermediate target object
+building crate_0153 source package assembled to intermediate target object
+building crate_0154 source package assembled to intermediate target object
+building crate_0155 source package assembled to intermediate target object
+building crate_0156 source package assembled to intermediate target object
+building crate_0157 source package assembled to intermediate target object
+building crate_0158 source package assembled to intermediate target object
+building crate_0159 source package assembled to intermediate target object
+building crate_0160 source package assembled to intermediate target object
+building crate_0161 source package assembled to intermediate target object
+building crate_0162 source package assembled to intermediate target object
+building crate_0163 source package assembled to intermediate target object
+building crate_0164 source package assembled to intermediate target object
+building crate_0165 source package assembled to intermediate target object
+building crate_0166 source package assembled to intermediate target object
+building crate_0167 source package assembled to intermediate target object
+building crate_0168 source package assembled to intermediate target object
+building crate_0169 source package assembled to intermediate target object
+building crate_0170 source package assembled to intermediate target object
+building crate_0171 source package assembled to intermediate target object
+building crate_0172 source package assembled to intermediate target object
+building crate_0173 source package assembled to intermediate target object
+building crate_0174 source package assembled to intermediate target object
+building crate_0175 source package assembled to intermediate target object
+building crate_0176 source package assembled to intermediate target object
+building crate_0177 source package assembled to intermediate target object
+building crate_0178 source package assembled to intermediate target object
+building crate_0179 source package assembled to intermediate target object
+warning: crate feature is deprecated at src/module.rs:42
+test result: passed; 187 passed; 0 failed
+exit=0
diff --git a/eval/release_gate.py b/eval/release_gate.py
index 30a4a76..738be57 100644
--- a/eval/release_gate.py
+++ b/eval/release_gate.py
@@ -1,13 +1,24 @@
 #!/usr/bin/env python3
+"""Static release inputs sanity check. This does not replace build/security tests."""
 from pathlib import Path
 import re
+import sys
 
 root = Path(__file__).resolve().parents[1]
-required = [root / "Cargo.toml", root / "proto/conduit.proto", root / "deny.toml", root / "specs/threat-model.md"]
-missing = [str(path.relative_to(root)) for path in required if not path.exists()]
+required = [root / "Cargo.toml", root / "proto/conduit.proto", root / "deny.toml", root / "specs/threat-model.md",
+            root / "src/main.rs", root / "src/run.rs", root / "tests/cli.rs", root / "docs/wiki/batch-edit.md", root / "eval/benchmark.py", root / ".github/workflows/checks.yml"]
+crate_names = ("store", "scope", "lens", "atlas", "edit", "ledger", "relay", "guard", "verify", "vault", "memory", "fanout")
+for crate in crate_names:
+    required.extend((root / f"crates/{crate}/Cargo.toml", root / f"crates/{crate}/src/lib.rs"))
+missing = [str(path.relative_to(root)) for path in required if not path.is_file()]
 if missing:
-    raise SystemExit("missing release inputs: " + ", ".join(missing))
-text = (root / "eval/corpus/sample.log").read_text()
-if not re.search(r"error|passed|exit=", text, re.I):
-    raise SystemExit("corpus lacks required signal coverage")
-print("release metadata present; Rust gates must run in CI")
+    raise SystemExit("missing required project inputs: " + ", ".join(missing))
+files = sorted((root / "eval/corpus").glob("*.log"))
+if not files:
+    raise SystemExit("missing benchmark corpus: no .log fixtures")
+for path in files:
+    contents = path.read_text(encoding="utf-8")
+    if not contents.strip() or not re.search(r"error|failed|panic|passed|exit=", contents, re.I):
+        raise SystemExit(f"invalid or signal-free benchmark corpus: {path.name}")
+print(f"static_release_inputs=present fixtures={len(files)} crates={len(crate_names)}")
+print("not_verified=rust_compilation,subprocess_isolation,live_agent_performance", file=sys.stderr)
diff --git a/eval/test_benchmark.py b/eval/test_benchmark.py
new file mode 100644
index 0000000..d3271a1
--- /dev/null
+++ b/eval/test_benchmark.py
@@ -0,0 +1,25 @@
+"""Real parser invariants, independent of any simulated executable."""
+import unittest
+
+from benchmark import parse_measurement
+
+
+class BenchmarkValidation(unittest.TestCase):
+    def test_parses_rust_measurement(self):
+        self.assertEqual(parse_measurement("raw_words=24 presented_words=9 raw_lines=6 presented_lines=2 compact=true raw_recoverable=true mode=rust_cli_effective_word_estimate"), (24, 9, True))
+
+    def test_rejects_impossible_savings_and_wrong_implementation(self):
+        for line in (
+            "raw_words=0 presented_words=0 compact=false raw_recoverable=true mode=rust_cli_effective_word_estimate",
+            "raw_words=5 presented_words=10 compact=true raw_recoverable=true mode=rust_cli_effective_word_estimate",
+            "raw_words=10 presented_words=2 compact=true raw_recoverable=true mode=python_proxy",
+            "raw_words=10 presented_words=2 compact=unknown raw_recoverable=true mode=rust_cli_effective_word_estimate",
+            "raw_words=10 presented_words=2 compact=true raw_recoverable=false mode=rust_cli_effective_word_estimate",
+            "raw_words=10 presented_words=2 compact=false raw_recoverable=true mode=rust_cli_effective_word_estimate",
+        ):
+            with self.subTest(line=line), self.assertRaises(ValueError):
+                parse_measurement(line)
+
+
+if __name__ == "__main__":
+    unittest.main()
diff --git a/specs/threat-model.md b/specs/threat-model.md
index a1a1a69..0242253 100644
--- a/specs/threat-model.md
+++ b/specs/threat-model.md
@@ -11,8 +11,8 @@ Agent input and child-process output are untrusted. The workspace is untrusted.
 ## Required controls
 
 - Content-addressed storage and hash-anchored edits prevent stale reads from becoming writes.
-- Policy checks precede command execution and path checks precede file access.
-- Output reduction is reversible and fail-open.
+- Intended control: policy checks should precede command execution and path checks should precede file access. **Present reality:** `run` is not policy-gated and inherits caller privileges; it is not an isolation boundary.
+- Compact stdout reduction is reversible only when a blob was successfully persisted and verified. Output beyond capture limits is explicitly incomplete and cannot be expanded.
 - Shared mutable state has one synchronization boundary and deterministic snapshots.
 - Network access is absent unless explicitly configured by a later provider adapter.
 - Release artifacts require dependency audit, formatting, lint, tests, benchmark execution, and reproducible build metadata.
diff --git a/src/main.rs b/src/main.rs
index dd42a91..0ec0d71 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,12 +1,21 @@
+mod run;
+
+use conduit_atlas::{find_streaming, peek};
+use conduit_edit::{apply, preview, Operation};
+use conduit_lens::distill;
 use conduit_scope::{Event, EventLog, Savings};
 use conduit_store::BlobStore;
-use conduit_lens::distill;
-use conduit_atlas::Snapshot;
-use conduit_edit::{apply, Operation};
-use std::{env, fs, path::PathBuf, process};
+use run::{Capture, RunOptions, RunResult};
+use std::{
+    env, fs,
+    io::{self, Read, Write},
+    path::PathBuf,
+    process,
+    time::{Duration, SystemTime, UNIX_EPOCH},
+};
 
 fn usage() -> ! {
-    eprintln!("usage: conduit <run|find|edit|store|expand|record|replay|stats|bench> ...");
+    eprintln!("usage: conduit run [--preset safe|quick|deep|compact] [--timeout-ms N] [--max-output-bytes N] [--budget N] [--compact] -- PROGRAM [ARGS...]\n       conduit <find|peek|edit|batch-edit|store|expand|doctor|record|replay|stats|repair-session|bench> ...");
     process::exit(2);
 }
 
@@ -14,88 +23,353 @@ fn root() -> PathBuf {
     env::var_os("CONDUIT_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".conduit"))
 }
 
+fn session_log(session: &str) -> io::Result<EventLog> {
+    if session.is_empty() || session.len() > 64 || !session.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
+        return Err(io::Error::new(io::ErrorKind::InvalidInput, "session IDs must contain 1-64 ASCII letters, digits, _ or -"));
+    }
+    EventLog::open(root().join("sessions").join(format!("{session}.jsonl")))
+}
+
+#[derive(Debug, Clone, Copy)]
+struct RunConfig {
+    timeout_ms: u64,
+    max_output_bytes: usize,
+    budget: usize,
+    compact: bool,
+}
+
+fn preset(name: &str) -> Option<RunConfig> {
+    match name {
+        "safe" => Some(RunConfig { timeout_ms: 120_000, max_output_bytes: 8 * 1024 * 1024, budget: 700, compact: false }),
+        "quick" => Some(RunConfig { timeout_ms: 30_000, max_output_bytes: 4 * 1024 * 1024, budget: 500, compact: false }),
+        "deep" => Some(RunConfig { timeout_ms: 600_000, max_output_bytes: 32 * 1024 * 1024, budget: 1_500, compact: false }),
+        "compact" => Some(RunConfig { timeout_ms: 120_000, max_output_bytes: 8 * 1024 * 1024, budget: 700, compact: true }),
+        _ => None,
+    }
+}
+
+fn parse_run(mut args: impl Iterator<Item = String>) -> (RunConfig, String, Vec<String>) {
+    let mut chosen_preset = "safe".to_owned();
+    let (mut timeout, mut max_output, mut budget) = (None, None, None);
+    let mut compact = false;
+    let mut program = None;
+    while let Some(argument) = args.next() {
+        match argument.as_str() {
+            "--" => { program = args.next(); break; }
+            "--preset" => chosen_preset = args.next().unwrap_or_else(|| usage()),
+            "--timeout-ms" => {
+                timeout = Some(args.next().and_then(|arg| arg.parse().ok()).unwrap_or_else(|| usage()));
+            }
+            "--max-output-bytes" => {
+                max_output = Some(args.next().and_then(|arg| arg.parse().ok()).unwrap_or_else(|| usage()));
+            }
+            "--budget" => {
+                budget = Some(args.next().and_then(|arg| arg.parse().ok()).unwrap_or_else(|| usage()));
+            }
+            "--compact" => compact = true,
+            flag if flag.starts_with("--") => usage(),
+            command => { program = Some(command.to_owned()); break; }
+        }
+    }
+    let mut config = preset(&chosen_preset).unwrap_or_else(|| usage());
+    // Explicit flags win regardless of whether --preset comes before or after.
+    if let Some(value) = timeout { config.timeout_ms = value; }
+    if let Some(value) = max_output { config.max_output_bytes = value; }
+    if let Some(value) = budget { config.budget = value; }
+    if compact { config.compact = true; }
+    if config.timeout_ms == 0 || config.timeout_ms > 3_600_000 || config.max_output_bytes == 0 || config.max_output_bytes > 64 * 1024 * 1024 || config.budget == 0 || config.budget > 100_000 {
+        usage();
+    }
+    (config, program.unwrap_or_else(|| usage()), args.collect())
+}
+
+#[derive(Debug, serde::Deserialize)]
+#[serde(deny_unknown_fields)]
+struct BatchManifest {
+    operations: Vec<Operation>,
+}
+
+fn read_manifest(path: &std::path::Path) -> io::Result<BatchManifest> {
+    let mut bytes = Vec::new();
+    fs::File::open(path)?.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
+    if bytes.len() > 1024 * 1024 {
+        return Err(io::Error::new(io::ErrorKind::InvalidInput, "batch manifest exceeds 1 MiB"));
+    }
+    serde_json::from_slice(&bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
+}
+
+fn no_extra_args(args: &mut impl Iterator<Item = String>) {
+    if args.next().is_some() { usage(); }
+}
+
+fn run_command(args: impl Iterator<Item = String>) -> ! {
+    let (config, program, program_args) = parse_run(args);
+    let result = run::run(&program, &program_args, RunOptions {
+        timeout: Duration::from_millis(config.timeout_ms),
+        max_stream_bytes: config.max_output_bytes,
+    }).unwrap_or_else(|error| fail(error));
+    let (printed_stdout, saved) = render_stdout(&result, config, &root());
+    io::stdout().write_all(&printed_stdout).unwrap_or_else(|error| fail(error));
+    io::stderr().write_all(&result.stderr.bytes).unwrap_or_else(|error| fail(error));
+    if let Some((handle, raw_words, shown_words)) = saved {
+        eprintln!("[conduit raw_stdout={} raw_words={} effective_shown_words={} expand='conduit expand {}']", handle, raw_words, shown_words, handle);
+        if let Ok(session) = env::var("CONDUIT_SESSION") {
+            if let Ok(log) = session_log(&session) {
+                // Whitespace counts are not tokenizer-accurate usage metrics.
+                let event = Event::Savings(Savings {
+                    module: "lens-words".into(),
+                    raw_tokens: raw_words.min(u32::MAX as usize) as u32,
+                    presented_tokens: shown_words.min(u32::MAX as usize) as u32,
+                    measured: false,
+                });
+                if let Err(error) = log.append(event) { eprintln!("conduit: unable to record session event: {error}"); }
+            }
+        }
+    }
+    let output_overflow = result.stdout.truncated || result.stderr.truncated;
+    if output_overflow {
+        eprintln!("conduit: output capture limit exceeded (stdout={} stderr={} bytes); output is INCOMPLETE and no lossless expansion is available", result.stdout.total_bytes, result.stderr.total_bytes);
+    }
+    if result.timed_out {
+        eprintln!("conduit: direct child timed out after {} ms; descendant processes may survive", config.timeout_ms);
+    }
+    if env::var_os("CONDUIT_DEBUG_TIMING").is_some() {
+        eprintln!("conduit: child_elapsed_ms={}", result.elapsed.as_millis());
+    }
+    if result.timed_out { process::exit(124); }
+    if output_overflow { process::exit(75); }
+    process::exit(result.exit_code);
+}
+
+fn render_stdout(result: &RunResult, config: RunConfig, home: &std::path::Path) -> (Vec<u8>, Option<(String, usize, usize)>) {
+    if !config.compact || result.exit_code != 0 || result.timed_out || result.stdout.truncated || result.stderr.truncated {
+        return (result.stdout.bytes.clone(), None);
+    }
+    let Ok(text) = std::str::from_utf8(&result.stdout.bytes) else {
+        return (result.stdout.bytes.clone(), None);
+    };
+    if text.contains('\0') {
+        return (result.stdout.bytes.clone(), None);
+    }
+    let filtered = distill(text, config.budget);
+    if filtered.dropped.is_empty() || filtered.presented.len() >= text.len() {
+        return (result.stdout.bytes.clone(), None);
+    }
+    // Compute the actual hint overhead before writing a blob. A small
+    // reduction that only moves words into its recovery hint is a regression.
+    let simulated_handle = "cx:o:aaaaaaaaaaaaaaaaaaaa";
+    let hint = format!("[conduit raw_stdout={} raw_words={} shown_words={} expand='conduit expand {}']", simulated_handle, filtered.raw_tokens, filtered.presented_tokens, simulated_handle);
+    let presented_words = filtered.presented_tokens + hint.split_whitespace().count();
+    if presented_words >= filtered.raw_tokens || filtered.presented.len() + hint.len() >= text.len() {
+        return (result.stdout.bytes.clone(), None);
+    }
+    let Ok(store) = BlobStore::open(home.join("blobs")) else {
+        return (result.stdout.bytes.clone(), None);
+    };
+    let Ok(blob) = store.put(&result.stdout.bytes) else {
+        return (result.stdout.bytes.clone(), None);
+    };
+    (filtered.presented.into_bytes(), Some((blob.handle, filtered.raw_tokens, presented_words)))
+}
+
 fn main() {
     let mut args = env::args().skip(1);
     match args.next().as_deref() {
-        Some("run") => {
-            let mut command = args.next().unwrap_or_else(|| usage());
-            if command == "--" { command = args.next().unwrap_or_else(|| usage()); }
-            let command_args: Vec<String> = args.collect();
-            let output = std::process::Command::new(&command).args(&command_args).output().unwrap_or_else(|e| fail(e));
-            let mut raw = output.stdout;
-            raw.extend_from_slice(&output.stderr);
-            let raw_text = String::from_utf8_lossy(&raw);
-            let blob = BlobStore::open(root().join("blobs")).ok().and_then(|store| store.put(&raw).ok());
-            let result = std::panic::catch_unwind(|| distill(&raw_text, 700));
-            match result {
-                Ok(filtered) => {
-                    if filtered.dropped.is_empty() {
-                        print!("{}", filtered.presented);
-                    } else if let Some(blob) = blob {
-                        print!("{}", filtered.presented);
-                        println!("[conduit h={} shown={}/{}tok exit={} dropped='{}' expand='conduit expand {}']", blob.handle, filtered.presented_tokens, filtered.raw_tokens, output.status.code().unwrap_or(1), filtered.dropped.join(", "), blob.handle);
-                    } else {
-                        print!("{}", raw_text);
-                    }
-                }
-                Err(_) => print!("{}", raw_text),
-            }
-            std::process::exit(output.status.code().unwrap_or(1));
-        }
+        Some("run") => run_command(args),
         Some("find") => {
             let query = args.next().unwrap_or_else(|| usage());
-            let limit = args.next().and_then(|value| value.parse().ok()).unwrap_or(20);
+            let limit = args.next().map(|value| value.parse::<usize>().unwrap_or_else(|_| usage())).unwrap_or(20);
+            if !(1..=500).contains(&limit) { usage(); }
+            no_extra_args(&mut args);
             let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
-            let snapshot = Snapshot::build(workspace).unwrap_or_else(|e| fail(e));
-            for hit in snapshot.search(&query, limit) { println!("{}:{}:{} [{}]", hit.path.display(), hit.line, hit.text, hit.hash); }
+            let result = find_streaming(workspace, &query, limit).unwrap_or_else(|error| fail(error));
+            for hit in &result.hits {
+                let preview: String = hit.text.chars().take(240).collect();
+                let cropped = hit.text.chars().count() > 240;
+                println!("{}:{}:{}{} [{}]", hit.path.display(), hit.line, preview, if cropped { " [line clipped; use peek]" } else { "" }, hit.hash);
+            }
+            if result.skipped != 0 || result.truncated || result.limit_reached {
+                eprintln!("conduit: search scanned_files={} scanned_bytes={} skipped={} truncated={} hit_limit_reached={}",
+                    result.scanned_files, result.scanned_bytes, result.skipped, result.truncated, result.limit_reached);
+            }
+        }
+        Some("peek") => {
+            let relative = PathBuf::from(args.next().unwrap_or_else(|| usage()));
+            let start = args.next().map(|value| value.parse::<usize>().unwrap_or_else(|_| usage())).unwrap_or(1);
+            let count = args.next().map(|value| value.parse::<usize>().unwrap_or_else(|_| usage())).unwrap_or(80);
+            no_extra_args(&mut args);
+            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
+            let excerpt = peek(workspace, &relative, start, count).unwrap_or_else(|error| fail(error));
+            println!("file={} hash={} lines={} truncated={}", relative.display(), excerpt.hash, excerpt.total_lines, excerpt.truncated);
+            for (line, text) in excerpt.lines { println!("{line}: {text}"); }
         }
         Some("edit") => {
             let path = PathBuf::from(args.next().unwrap_or_else(|| usage()));
             let expected_hash = args.next().unwrap_or_else(|| usage());
             let old = args.next().unwrap_or_else(|| usage());
             let new = args.next().unwrap_or_else(|| usage());
+            no_extra_args(&mut args);
             let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
-            let applied = apply(workspace, &[Operation::SearchReplace { path, expected_hash, old, new }]).unwrap_or_else(|e| fail(e));
+            let applied = apply(workspace, &[Operation::SearchReplace { path, expected_hash, old, new }]).unwrap_or_else(|error| fail(error));
             for item in applied { println!("{} {}", item.path.display(), item.hash); }
         }
+        Some("batch-edit") => {
+            let first = args.next().unwrap_or_else(|| usage());
+            let (check, manifest_path) = if first == "--check" {
+                (true, PathBuf::from(args.next().unwrap_or_else(|| usage())))
+            } else {
+                (false, PathBuf::from(first))
+            };
+            no_extra_args(&mut args);
+            let manifest = read_manifest(&manifest_path).unwrap_or_else(|error| fail(error));
+            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
+            let results = if check { preview(workspace, &manifest.operations) } else { apply(workspace, &manifest.operations) }
+                .unwrap_or_else(|error| fail(error));
+            for item in results {
+                println!("{}={} hash={}", if check { "validated" } else { "applied" }, item.path.display(), item.hash);
+            }
+        }
         Some("store") => {
-            let text = args.next().unwrap_or_else(|| usage());
-            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|e| fail(e));
-            let blob = store.put(text.as_bytes()).unwrap_or_else(|e| fail(e));
+            let argument = args.next().unwrap_or_else(|| usage());
+            no_extra_args(&mut args);
+            let bytes = if argument == "-" {
+                let mut bytes = Vec::new();
+                io::stdin().lock().take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes)
+                    .unwrap_or_else(|error| fail(error));
+                bytes
+            } else {
+                argument.into_bytes()
+            };
+            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
+            let blob = store.put(&bytes).unwrap_or_else(|error| fail(error));
             println!("{}", blob.handle);
         }
         Some("expand") => {
             let handle = args.next().unwrap_or_else(|| usage());
-            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|e| fail(e));
-            let bytes = store.get(&handle).unwrap_or_else(|e| fail(e));
-            print!("{}", String::from_utf8_lossy(&bytes));
+            no_extra_args(&mut args);
+            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
+            let bytes = store.get(&handle).unwrap_or_else(|error| fail(error));
+            io::stdout().write_all(&bytes).unwrap_or_else(|error| fail(error));
+        }
+        Some("doctor") => {
+            no_extra_args(&mut args);
+            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
+            let report = store.audit().unwrap_or_else(|error| fail(error));
+            println!("verified_blobs={} verified_bytes={} staged_blobs={} integrity_problems={}",
+                report.checked_blobs, report.verified_bytes, report.staging_files, report.problem_count);
+            for problem in report.problems { eprintln!("conduit: {problem}"); }
+            if report.problem_count > 0 { process::exit(1); }
         }
         Some("record") => {
             let session = args.next().unwrap_or_else(|| usage());
             let module = args.next().unwrap_or_else(|| usage());
             let raw: u32 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
             let presented: u32 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
-            let log = EventLog::open(root().join("sessions").join(format!("{session}.jsonl"))).unwrap_or_else(|e| fail(e));
-            log.append(Event::Savings(Savings { module, raw_tokens: raw, presented_tokens: presented, measured: false })).unwrap_or_else(|e| fail(e));
+            no_extra_args(&mut args);
+            let log = session_log(&session).unwrap_or_else(|error| fail(error));
+            log.append(Event::Savings(Savings { module, raw_tokens: raw, presented_tokens: presented, measured: false })).unwrap_or_else(|error| fail(error));
         }
         Some("replay") => {
             let session = args.next().unwrap_or_else(|| usage());
-            let log = EventLog::open(root().join("sessions").join(format!("{session}.jsonl"))).unwrap_or_else(|e| fail(e));
-            for event in log.read().unwrap_or_else(|e| fail(e)) { println!("{}", event); }
+            no_extra_args(&mut args);
+            let log = session_log(&session).unwrap_or_else(|error| fail(error));
+            for event in log.events().unwrap_or_else(|error| fail(error)) { println!("{event}"); }
+        }
+        Some("repair-session") => {
+            let session = args.next().unwrap_or_else(|| usage());
+            no_extra_args(&mut args);
+            let log = session_log(&session).unwrap_or_else(|error| fail(error));
+            println!("partial_tail_removed={}", log.repair_partial_tail().unwrap_or_else(|error| fail(error)));
         }
         Some("stats") => {
             let session = args.next().unwrap_or_else(|| usage());
-            let log = EventLog::open(root().join("sessions").join(format!("{session}.jsonl"))).unwrap_or_else(|e| fail(e));
-            let (raw, shown) = log.totals().unwrap_or_else(|e| fail(e));
-            println!("raw_tokens={raw} presented_tokens={shown} saved_tokens={}", raw.saturating_sub(shown));
+            no_extra_args(&mut args);
+            let log = session_log(&session).unwrap_or_else(|error| fail(error));
+            let (raw, shown) = log.totals().unwrap_or_else(|error| fail(error));
+            println!("raw_units={raw} presented_units={shown} net_saved_units={}", i128::from(raw) - i128::from(shown));
+            println!("note=word_based_or_caller_supplied_not_model_token_measurement");
         }
         Some("bench") => {
-            let text = fs::read_to_string("eval/corpus/sample.log").unwrap_or_else(|e| fail(e));
-            let shown = text.lines().filter(|line| line.contains("ERROR") || line.contains("FAILED") || line.contains("passed") || line.contains("exit=")).count();
-            println!("corpus_lines={} retained_lines={} mode=deterministic", text.lines().count(), shown);
+            let path = args.next().unwrap_or_else(|| "eval/corpus/sample.log".into());
+            let budget: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(700);
+            if budget == 0 || budget > 100_000 { usage(); }
+            no_extra_args(&mut args);
+            let raw_bytes = fs::read(&path).unwrap_or_else(|error| fail(error));
+            if raw_bytes.is_empty() { fail("benchmark corpus must not be empty"); }
+            let raw_text = std::str::from_utf8(&raw_bytes).unwrap_or_else(|error| fail(error));
+            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
+            let temp_home = env::temp_dir().join(format!("conduit-bench-{}-{stamp}", process::id()));
+            let config = RunConfig { compact: true, budget, ..preset("compact").expect("preset") };
+            let result = RunResult {
+                stdout: Capture { bytes: raw_bytes.clone(), total_bytes: raw_bytes.len() as u64, truncated: false },
+                stderr: Capture { bytes: Vec::new(), total_bytes: 0, truncated: false },
+                exit_code: 0,
+                timed_out: false,
+                elapsed: Duration::ZERO,
+            };
+            let (shown_bytes, savings) = render_stdout(&result, config, &temp_home);
+            let (raw_words, presented_words, compact, lossless) = if let Some((handle, raw, shown)) = savings {
+                let store = BlobStore::open(temp_home.join("blobs")).unwrap_or_else(|error| fail(error));
+                let lossless = store.get(&handle).map(|bytes| bytes == raw_bytes).unwrap_or(false);
+                (raw, shown, true, lossless)
+            } else {
+                let words = raw_text.split_whitespace().count();
+                (words, words, false, shown_bytes == raw_bytes)
+            };
+            if !lossless { fail("benchmark expansion failed to recover original bytes"); }
+            if let Err(error) = fs::remove_dir_all(&temp_home) {
+                if error.kind() != io::ErrorKind::NotFound {
+                    eprintln!("conduit: unable to clean benchmark temp state: {error}");
+                }
+            }
+            println!("raw_words={raw_words} presented_words={presented_words} raw_lines={} presented_lines={} compact={compact} raw_recoverable={lossless} mode=rust_cli_effective_word_estimate", raw_text.lines().count(), String::from_utf8_lossy(&shown_bytes).lines().count());
         }
         _ => usage(),
     }
 }
 
-fn fail(error: impl std::fmt::Display) -> ! { eprintln!("conduit: {error}"); process::exit(1) }
+fn fail(error: impl std::fmt::Display) -> ! {
+    eprintln!("conduit: {error}");
+    process::exit(1)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn rejects_session_path_injection() {
+        for bad in ["../sessions", ".", "../../etc/passwd", "a/b", "", "a\\b", "a\n"] {
+            assert!(session_log(bad).is_err());
+        }
+    }
+
+    #[test]
+    fn presets_have_distinct_budgets_and_modes() {
+        assert!(!preset("safe").unwrap().compact);
+        assert!(preset("compact").unwrap().compact);
+        assert!(preset("quick").unwrap().timeout_ms < preset("deep").unwrap().timeout_ms);
+        assert!(preset("unknown").is_none());
+    }
+
+    #[test]
+    fn command_flag_overrides_preset_regardless_of_order() {
+        let before = vec!["--timeout-ms", "1500", "--preset", "quick", "--", "echo"];
+        let after = vec!["--preset", "quick", "--timeout-ms", "1500", "--", "echo"];
+        for tokens in [before, after] {
+            let (config, command, remaining) = parse_run(tokens.into_iter().map(ToOwned::to_owned));
+            assert_eq!(config.timeout_ms, 1500);
+            assert_eq!(config.max_output_bytes, 4 * 1024 * 1024);
+            assert_eq!(command, "echo");
+            assert!(remaining.is_empty());
+        }
+    }
+
+    #[test]
+    fn rejects_large_batch_manifest_file() {
+        let path = env::temp_dir().join(format!("conduit-manifest-{}.json", process::id()));
+        fs::write(&path, vec![b'x'; 1024 * 1024 + 1]).unwrap();
+        assert_eq!(read_manifest(&path).unwrap_err().kind(), io::ErrorKind::InvalidInput);
+        let _ = fs::remove_file(path);
+    }
+}
diff --git a/src/run.rs b/src/run.rs
new file mode 100644
index 0000000..e879bed
--- /dev/null
+++ b/src/run.rs
@@ -0,0 +1,153 @@
+//! A bounded local subprocess adapter, not an OS sandbox.
+use std::{
+    io::{self, Read},
+    process::{Command, Stdio},
+    sync::mpsc,
+    thread,
+    time::{Duration, Instant},
+};
+
+#[derive(Debug, Clone, Copy)]
+pub struct RunOptions {
+    pub timeout: Duration,
+    /// Maximum bytes retained independently for stdout and stderr.
+    pub max_stream_bytes: usize,
+}
+
+#[derive(Debug)]
+pub struct Capture {
+    pub bytes: Vec<u8>,
+    pub total_bytes: u64,
+    pub truncated: bool,
+}
+
+#[derive(Debug)]
+pub struct RunResult {
+    pub stdout: Capture,
+    pub stderr: Capture,
+    pub exit_code: i32,
+    pub timed_out: bool,
+    pub elapsed: Duration,
+}
+
+fn capture(mut input: impl Read, maximum: usize) -> io::Result<Capture> {
+    let mut bytes = Vec::new();
+    let mut total_bytes = 0u64;
+    let mut buffer = [0u8; 8192];
+    loop {
+        let amount = input.read(&mut buffer)?;
+        if amount == 0 { break; }
+        total_bytes = total_bytes.saturating_add(amount as u64);
+        let retained = maximum.saturating_sub(bytes.len()).min(amount);
+        bytes.extend_from_slice(&buffer[..retained]);
+    }
+    Ok(Capture { truncated: total_bytes > bytes.len() as u64, bytes, total_bytes })
+}
+
+/// Captures both streams independently and drains over-limit output rather
+/// than deadlocking the child on a full pipe. A timeout kills the direct
+/// child; descendant-process containment requires an OS sandbox/supervisor.
+pub fn run(program: &str, args: &[String], options: RunOptions) -> io::Result<RunResult> {
+    if options.timeout.is_zero() || options.max_stream_bytes == 0 {
+        return Err(io::Error::new(io::ErrorKind::InvalidInput, "run resource limits must be positive"));
+    }
+    let start = Instant::now();
+    let mut child = Command::new(program)
+        .args(args)
+        .stdout(Stdio::piped())
+        .stderr(Stdio::piped())
+        .spawn()?;
+    let stdout = child.stdout.take().ok_or_else(|| io::Error::other("missing stdout pipe"))?;
+    let stderr = child.stderr.take().ok_or_else(|| io::Error::other("missing stderr pipe"))?;
+    let (tx, rx) = mpsc::channel();
+    for (index, stream) in [
+        (0usize, Box::new(stdout) as Box<dyn Read + Send>),
+        (1usize, Box::new(stderr) as Box<dyn Read + Send>),
+    ] {
+        let tx = tx.clone();
+        let cap = options.max_stream_bytes;
+        thread::spawn(move || { let _ = tx.send((index, capture(stream, cap))); });
+    }
+    drop(tx);
+
+    let mut timed_out = false;
+    let status = loop {
+        if let Some(status) = child.try_wait()? { break status; }
+        if start.elapsed() >= options.timeout {
+            timed_out = true;
+            let _ = child.kill();
+            break child.wait()?;
+        }
+        thread::sleep(Duration::from_millis(20));
+    };
+
+    // A descendant may inherit a pipe and hold it open after the direct child
+    // exits. Bound the join via a channel instead of waiting forever.
+    let mut captured = [None, None];
+    for _ in 0..2 {
+        let (index, result) = rx.recv_timeout(Duration::from_secs(2))
+            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "output pipe remained open after child exit"))?;
+        captured[index] = Some(result?);
+    }
+    Ok(RunResult {
+        stdout: captured[0].take().expect("stdout captured"),
+        stderr: captured[1].take().expect("stderr captured"),
+        exit_code: status.code().unwrap_or(1),
+        timed_out,
+        elapsed: start.elapsed(),
+    })
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn limits_retention_but_drains_entire_stream() {
+        let actual = capture(&b"abcdefgh"[..], 3).unwrap();
+        assert_eq!(actual.bytes, b"abc");
+        assert_eq!(actual.total_bytes, 8);
+        assert!(actual.truncated);
+    }
+
+    #[test]
+    fn retains_raw_bytes_including_invalid_utf8() {
+        let input = [b'a', 0xff, 0x00, b'b'];
+        let actual = capture(&input[..], 20).unwrap();
+        assert_eq!(actual.bytes, input);
+        assert!(!actual.truncated);
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn subprocess_preserves_stdout_stderr_and_nonzero_status() {
+        let args = vec!["-c".to_owned(), "printf output; printf errors >&2; exit 7".to_owned()];
+        let result = run("sh", &args, RunOptions {
+            timeout: Duration::from_secs(5),
+            max_stream_bytes: 1024,
+        }).unwrap();
+        assert_eq!(result.exit_code, 7);
+        assert_eq!(result.stdout.bytes, b"output");
+        assert_eq!(result.stderr.bytes, b"errors");
+        assert!(!result.timed_out);
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn subprocess_overflow_retains_partial_data_and_reports_truncation() {
+        let args = vec!["-c".to_owned(), "printf 1234567890; exit 0".to_owned()];
+        let result = run("sh", &args, RunOptions {
+            timeout: Duration::from_secs(5),
+            max_stream_bytes: 3,
+        }).unwrap();
+        assert_eq!(result.stdout.bytes, b"123");
+        assert_eq!(result.stdout.total_bytes, 10);
+        assert!(result.stdout.truncated);
+    }
+
+    #[test]
+    fn rejects_zero_limits() {
+        let options = RunOptions { timeout: Duration::ZERO, max_stream_bytes: 1 };
+        assert_eq!(run("should-not-launch", &[], options).unwrap_err().kind(), io::ErrorKind::InvalidInput);
+    }
+}
diff --git a/tests/cli.rs b/tests/cli.rs
new file mode 100644
index 0000000..0f0a0d2
--- /dev/null
+++ b/tests/cli.rs
@@ -0,0 +1,145 @@
+//! End-to-end CLI behavior checks; run by `cargo test --workspace --all-targets`.
+//! All tests use isolated temporary workspaces and local processes only.
+use std::{
+    fs,
+    path::{Path, PathBuf},
+    process::{Command, Output},
+    sync::atomic::{AtomicU64, Ordering},
+};
+
+static NEXT_TEST: AtomicU64 = AtomicU64::new(0);
+
+struct Workspace(PathBuf);
+impl Workspace {
+    fn new() -> Self {
+        let dir = std::env::temp_dir().join(format!(
+            "conduit-cli-{}-{}", std::process::id(), NEXT_TEST.fetch_add(1, Ordering::Relaxed)
+        ));
+        fs::create_dir_all(&dir).unwrap();
+        Self(dir)
+    }
+    fn path(&self) -> &Path { &self.0 }
+    fn command(&self, args: &[&str]) -> Output {
+        Command::new(env!("CARGO_BIN_EXE_conduit"))
+            .args(args)
+            .env("CONDUIT_HOME", self.0.join(".conduit"))
+            .env("CONDUIT_WORKSPACE", &self.0)
+            .output()
+            .unwrap()
+    }
+}
+impl Drop for Workspace {
+    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
+}
+fn utf8(bytes: &[u8]) -> &str { std::str::from_utf8(bytes).unwrap() }
+fn hash_from_peek(output: &Output) -> String {
+    assert!(output.status.success(), "{}", utf8(&output.stderr));
+    utf8(&output.stdout)
+        .split_whitespace()
+        .find_map(|field| field.strip_prefix("hash="))
+        .expect("peek emits a hash")
+        .to_owned()
+}
+
+#[test]
+fn store_expand_and_doctor_round_trip() {
+    let root = Workspace::new();
+    let saved = root.command(&["store", "roundtrip bytes"]);
+    assert!(saved.status.success(), "{}", utf8(&saved.stderr));
+    let handle = utf8(&saved.stdout).trim();
+    assert!(handle.starts_with("cx:o:"));
+    let fetched = root.command(&["expand", handle]);
+    assert!(fetched.status.success());
+    assert_eq!(&fetched.stdout, b"roundtrip bytes");
+    let doctor = root.command(&["doctor"]);
+    assert!(doctor.status.success(), "{}", utf8(&doctor.stderr));
+    assert!(utf8(&doctor.stdout).contains("integrity_problems=0"));
+    let shard_dir = root.path().join(".conduit/blobs");
+    let shard = fs::read_dir(&shard_dir).unwrap().next().unwrap().unwrap().path();
+    let blob = fs::read_dir(&shard).unwrap().next().unwrap().unwrap().path();
+    fs::write(&blob, b"wrong payload").unwrap();
+    let corrupt = root.command(&["doctor"]);
+    assert!(!corrupt.status.success(), "corrupted blob must not pass a health scan");
+    assert!(utf8(&corrupt.stdout).contains("integrity_problems=1"));
+    assert!(!root.command(&["expand", handle]).status.success());
+    let bogus = root.command(&["expand", "cx:o:../../etc/passwd"]);
+    assert!(!bogus.status.success());
+}
+
+#[test]
+fn find_peek_and_batch_edit_preflight_and_commit() {
+    let root = Workspace::new();
+    fs::write(root.path().join("app.rs"), "fn before() {}\n").unwrap();
+    fs::write(root.path().join(".env"), "fn before() {}\n").unwrap();
+    let find = root.command(&["find", "before", "20"]);
+    assert!(find.status.success(), "{}", utf8(&find.stderr));
+    assert!(utf8(&find.stdout).contains("app.rs:1:"));
+    assert!(!utf8(&find.stdout).contains(".env:"));
+    let peek = root.command(&["peek", "app.rs", "1", "20"]);
+    let hash = hash_from_peek(&peek);
+
+    let manifest = format!(
+        r#"{{"operations":[{{"type":"search_replace","path":"app.rs","expected_hash":"{hash}","old":"before","new":"after"}}]}}"#
+    );
+    let manifest_path = root.path().join("plan.json");
+    fs::write(&manifest_path, manifest).unwrap();
+    let path_string = manifest_path.to_str().unwrap();
+    let dry_run = root.command(&["batch-edit", "--check", path_string]);
+    assert!(dry_run.status.success(), "{}", utf8(&dry_run.stderr));
+    assert_eq!(fs::read_to_string(root.path().join("app.rs")).unwrap(), "fn before() {}\n");
+    assert!(utf8(&dry_run.stdout).contains("validated=app.rs"));
+
+    let edit = root.command(&["batch-edit", path_string]);
+    assert!(edit.status.success(), "{}", utf8(&edit.stderr));
+    assert_eq!(fs::read_to_string(root.path().join("app.rs")).unwrap(), "fn after() {}\n");
+    let stale = root.command(&["batch-edit", path_string]);
+    assert!(!stale.status.success(), "stale hash must reject reapplication");
+    let escape = root.command(&["peek", "../outside", "1", "20"]);
+    assert!(!escape.status.success());
+}
+
+#[test]
+fn batch_preflight_rejects_stale_second_target_before_first_mutation() {
+    let root = Workspace::new();
+    fs::write(root.path().join("first.rs"), "one").unwrap();
+    fs::write(root.path().join("second.rs"), "two").unwrap();
+    let first = hash_from_peek(&root.command(&["peek", "first.rs"]));
+    let manifest = format!(r#"{{"operations":[
+        {{"type":"search_replace","path":"first.rs","expected_hash":"{first}","old":"one","new":"changed"}},
+        {{"type":"search_replace","path":"second.rs","expected_hash":"{}","old":"two","new":"changed"}}
+    ]}}"#, "0".repeat(64));
+    let path = root.path().join("stale.json");
+    fs::write(&path, manifest).unwrap();
+    let result = root.command(&["batch-edit", path.to_str().unwrap()]);
+    assert!(!result.status.success());
+    assert_eq!(fs::read_to_string(root.path().join("first.rs")).unwrap(), "one");
+    assert_eq!(fs::read_to_string(root.path().join("second.rs")).unwrap(), "two");
+}
+
+#[test]
+fn session_repair_discards_only_unterminated_tail() {
+    let root = Workspace::new();
+    assert!(root.command(&["record", "session1", "test", "20", "10"]).status.success());
+    let log_path = root.path().join(".conduit/sessions/session1.jsonl");
+    use std::io::Write;
+    fs::OpenOptions::new().append(true).open(&log_path).unwrap().write_all(b"{\"type\":").unwrap();
+    assert!(!root.command(&["stats", "session1"]).status.success());
+    assert!(!root.command(&["record", "session1", "test", "1", "1"]).status.success());
+    let fixed = root.command(&["repair-session", "session1"]);
+    assert!(fixed.status.success(), "{}", utf8(&fixed.stderr));
+    assert!(utf8(&fixed.stdout).contains("partial_tail_removed=true"));
+    assert!(root.command(&["record", "session1", "test", "2", "1"]).status.success());
+    let stats = root.command(&["stats", "session1"]);
+    assert!(stats.status.success());
+    assert!(utf8(&stats.stdout).contains("raw_units=22 presented_units=11"));
+}
+
+#[test]
+fn run_preserves_command_status_and_streams() {
+    let root = Workspace::new();
+    // Launch a second Conduit process; no shell dependency on Windows.
+    let executable = env!("CARGO_BIN_EXE_conduit");
+    let output = root.command(&["run", "--preset", "quick", "--timeout-ms", "5000", "--", executable, "store", "subprocess"]);
+    assert!(output.status.success(), "{}", utf8(&output.stderr));
+    assert!(utf8(&output.stdout).starts_with("cx:o:"));
+}
