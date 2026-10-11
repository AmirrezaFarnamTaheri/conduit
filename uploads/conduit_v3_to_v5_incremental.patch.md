diff --git a/.github/workflows/checks.yml b/.github/workflows/checks.yml
index 342e79a9c9340180a1febe2ba179793ea70e70da..8171954f66f40e7bfc0ab2ef11ff719b3dedd268 100644
--- a/.github/workflows/checks.yml
+++ b/.github/workflows/checks.yml
@@ -29,6 +29,9 @@ jobs:
           python-version: '3.12'
       - run: cargo fmt --all -- --check
       - run: cargo test --workspace --all-targets
+      # Explicit interrupted-process recovery test. The crash hook is
+      # absent from normal production builds.
+      - run: cargo test -p conduit --features test-fault-injection --test cli
       - run: cargo clippy --workspace --all-targets -- -D warnings
       - run: cargo build --bin conduit
       - run: python3 -m unittest discover -s eval -p "test_*.py"
diff --git a/Cargo.toml b/Cargo.toml
index d6873c3d64dd5163d60e9df0591db4e1cc59c42b..60c07c421deb2af70d1e21a6391fea8201d6135c 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -8,6 +8,9 @@ version = "0.1.0"
 edition = "2021"
 license = "Apache-2.0"
 
+[features]
+test-fault-injection = ["conduit-edit/test-fault-injection"]
+
 [dependencies]
 conduit-store = { path = "crates/store" }
 conduit-scope = { path = "crates/scope" }
diff --git a/README.md b/README.md
index 4a5e2cd68c8fb97c4b9f4f822fb9e2c0b7771160..e97695f0d85ce5cf74d66cef03a3acb16aa404a7 100644
--- a/README.md
+++ b/README.md
@@ -4,7 +4,7 @@ Conduit is a **local coding-agent tool-output and source-navigation prototype**.
 
 ## Implementation status (source snapshot)
 
-The CLI currently implements `run`, `capabilities`, `find`, `peek`, `edit`, `batch-edit`, `store`, `expand`, `doctor`, `record`, `replay`, `stats`, `repair-session`, and `bench`. The independent `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, and `fanout` crates are **not wired into the CLI**, and their behavior is **not an execution security boundary**. No daemon, provider gateway, UI, MCP server, remote API, OS sandbox, verified multi-file crash transaction, or data-retention controller is supplied.
+The CLI currently implements `run`, `capabilities`, `find`, `peek`, `edit`, `batch-edit`, `recover-edits`, `store`, `inspect`, `expand`, `doctor`, `record`, `replay`, `stats`, `repair-session`, and `bench`. The independent `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, and `fanout` crates are **not wired into the CLI**, and their behavior is **not an execution security boundary**. No daemon, provider gateway, UI, MCP server, remote API, OS sandbox, verified multi-file crash transaction, or data-retention controller is supplied.
 
 The CLI launches local commands with the **same privileges and environment as the caller**. It does not enforce the `vault` policy module or isolate the child process. Run only trusted commands within an appropriately isolated operating-system account or managed sandbox. A timeout kills the direct child but cannot guarantee descendant process cleanup. The wrapper is not a PTY and does not preserve stdout/stderr temporal interleaving.
 
@@ -50,7 +50,10 @@ conduit batch-edit batch.json
 conduit store 'raw text'
 # Read binary or secret input from stdin instead of leaking it into argv
 producer_of_bytes | conduit store -
+conduit inspect --json cx:o:OUTPUT_HANDLE
+conduit expand cx:o:OUTPUT_HANDLE --offset 0 --length 1024
 conduit expand cx:o:OUTPUT_HANDLE
+conduit recover-edits --list # check interrupted edit batches
 conduit doctor             # verify blob integrity without deleting objects
 conduit doctor --json      # typed v1 diagnostic summary
 conduit record SESSION MODULE RAW_WORDS SHOWN_WORDS
@@ -65,12 +68,23 @@ conduit bench eval/corpus/sample.log 700
 
 `run` retains a bounded amount from stdout and stderr **independently**, requests early termination when either limit is exceeded, and returns an explicit incomplete-output error. A stuck or escaped descendant may still retain a pipe, triggering a bounded failure. The wrapper returns `124` when the direct child times out. Such incomplete output is **not losslessly recoverable**. When successful stdout is reduced, the original stdout bytes are persisted first and the expansion handle appears in stdout along with the compacted content so stdout-only agent adapters can recover it. Structured adapters must distinguish this presentation from unmodified child stdout. If storage fails or reduction has no net value after counting the recovery hint, stdout passes through unchanged. For non-successful child exits the program leaves stdout uncompressed; the effective exit status is propagated for ordinary exits, with platform limitations for signal termination. Stderr is never reduced.
 
-`edit` and `batch-edit` refuse stale BLAKE3 hashes, duplicate canonical targets, escape paths, protected metadata paths, hardlinked targets, and edits over 8 MiB per file. A batch accepts 1–128 operations up to a combined 64 MiB of original and replacement text. `--check` performs the same validations but does not modify source files; it still creates the cooperative lock. The commit writes with per-file staging. It **does not** guarantee all-or-nothing multi-file transactions across crashes, concurrent writes from other tools, or power loss. See `docs/wiki/batch-edit.md` for a manifest example.
+`edit` and `batch-edit` refuse stale BLAKE3 hashes, duplicate canonical targets, escape paths, protected metadata paths, hardlinked targets, and edits over 8 MiB per file. A batch accepts 1–128 operations up to a combined 64 MiB of original and replacement text. `--check` performs the same validations but does not modify source files; it still creates the cooperative lock. The commit writes with per-file staging. Before the first replacement, it synchronizes a recovery receipt and original-byte backups to the local workspace. Interrupted batches block new edits and support explicit hash-fenced rollback via `recover-edits`. It **does not** guarantee all-or-nothing multi-file visibility, protection from uncooperative writers, or certified cross-platform power-loss behavior. See `docs/wiki/batch-edit.md` for a manifest example.
 
 Session records are bounded to 8 KiB each and session logs to 64 MiB. `stats` streams and validates records without materializing the history. If a crash interrupts a JSONL append, `record` and `stats` refuse to gloss over the incomplete tail. `repair-session` only discards the unterminated final fragment after verifying all preceding completed events; it cannot reconstruct missing data and must not be mistaken for an audit-log recovery guarantee. `doctor` provides an on-demand, read-only blob integrity scan; it does not perform retention or GC.
 
 The `find` command orders paths deterministically within the walker; hit limits and skipped/binary/unreadable files still make results potentially incomplete. `find --require-complete` exits 75 if any scan limit, skipped file, or hit limit prevents a complete claim. Read-only `find`, `peek`, `doctor`, `stats` and `capabilities` support `--json` with a documented versioned response schema. Paths not representable as UTF-8 are lossy in JSON; `path_utf8=false` explicitly marks them as unsuitable for direct editing. See `docs/wiki/structured-output.md`.
 
+## Recovery and inspection improvements
+
+The storage inspector and byte-window expansion validate full BLAKE3 digests
+using bounded buffers. Incomplete edit receipts support both verified rollback
+and verified finalization after **all** updated hashes are present; neither
+command makes a partially committed batch atomically visible. Explicit orphan
+backup cleanup is available separately after incident evidence is preserved.
+Session repair retains valid unterminated JSONL events or quarantines malformed
+tails before truncation. These operations do not provide an OS sandbox or
+protect against uncooperative filesystem mutation.
+
 ## Repository map
 
 | Path | Responsibility | Current CLI integration |
@@ -96,7 +110,7 @@ The `find` command orders paths deterministically within the walker; hit limits
 
 - **No untrusted command isolation:** unrestricted caller privileges; no sandbox enforcement or provider egress policy.
 - **No durable content-retention policy:** the local blob store can grow without garbage collection. Sensitive observations may persist even after a command finishes.
-- **No certified multi-file crash recovery:** stage/rename rollback is best-effort and protects only cooperative Conduit instances.
+- **No certified multi-file atomicity:** recovery receipts now permit explicit, hash-fenced rollback of compatible partially applied edits, but readers can observe mixed versions. Ownership, xattrs and ACL preservation and hostile filesystem races are not fully handled. Fault-injection tests are added to CI but their results are not yet available in this audit environment.
 - **No exact-token accounting:** `record` accepts caller-supplied numbers; `stats` reports sums of estimates or claims, including negative net savings, not verified model usage.
 - **No workload non-inferiority evidence:** no live-agent success/accuracy or production p95 performance comparisons have been supplied.
 
diff --git a/crates/edit/Cargo.toml b/crates/edit/Cargo.toml
index fa370563c5c6eb931ac42d9182cd66113d45d498..b52331a5d3b1c6ebb9978071632e80123748bc38 100644
--- a/crates/edit/Cargo.toml
+++ b/crates/edit/Cargo.toml
@@ -4,11 +4,12 @@ version = "0.1.0"
 edition = "2021"
 license = "Apache-2.0"
 
+[features]
+test-fault-injection = []
+
 [dependencies]
 blake3 = "1.5"
 fs2 = "0.4"
 libc = "0.2"
 serde = { version = "1", features = ["derive"] }
-
-[dev-dependencies]
 serde_json = "1"
diff --git a/crates/edit/src/lib.rs b/crates/edit/src/lib.rs
index 48748f44677b7cf633bd8376ee8122282a9be87a..ff780b1fdb17f6123a6268a493b08aba361aee53 100644
--- a/crates/edit/src/lib.rs
+++ b/crates/edit/src/lib.rs
@@ -1,7 +1,10 @@
 //! Hash-anchored best-effort batch edits. Multi-file crash atomicity is NOT
-//! provided; a journaled filesystem transaction remains a separate project.
+//! provided; persisted receipts allow explicit, hash-fenced rollback after a crash.
 //! Preflight validation is shared by previews and commits; a preview does not
 //! reserve files against non-cooperating writers.
+mod recovery;
+pub use recovery::PendingEdit;
+
 use blake3::Hasher;
 use fs2::FileExt;
 use std::{
@@ -34,7 +37,6 @@ struct Staged {
     expected: String,
     updated: String,
     previous: Vec<u8>,
-    permissions: Permissions,
 }
 
 struct Planned {
@@ -51,6 +53,7 @@ struct Planned {
 pub fn preview(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
     let workspace = workspace.as_ref().canonicalize()?;
     let _lock = lock_workspace(&workspace)?;
+    recovery::ensure_no_pending(&workspace)?;
     let planned = validate(&workspace, operations)?;
     Ok(planned.iter().map(|item| Applied {
         path: item.relative.clone(), hash: hash(item.updated.as_bytes()),
@@ -60,6 +63,7 @@ pub fn preview(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Res
 pub fn apply(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
     let workspace = workspace.as_ref().canonicalize()?;
     let _lock = lock_workspace(&workspace)?;
+    recovery::ensure_no_pending(&workspace)?;
     let planned = validate(&workspace, operations)?;
     let mut staged = Vec::with_capacity(planned.len());
     let result = (|| {
@@ -68,26 +72,55 @@ pub fn apply(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Resul
             staged.push(Staged {
                 relative: item.relative, path: item.path, temp,
                 expected: item.expected, updated: item.updated,
-                previous: item.previous, permissions: item.permissions,
+                previous: item.previous,
             });
         }
-        // Cooperative validation narrows races, but external writers can
-        // modify files between this check and the OS rename operation.
-        for item in &staged { verify_expected(&item.path, &item.expected)?; }
-        let mut committed = 0usize;
-        for item in &staged {
-            let result = verify_expected(&item.path, &item.expected)
-                .and_then(|_| fs::rename(&item.temp, &item.path));
-            if result.is_ok() { committed += 1; }
-            let result = result.and_then(|_| sync_parent(&item.path));
-            if let Err(error) = result {
-                let mut rollback_error = None;
-                for prior in staged[..committed].iter().rev() {
-                    if let Err(failed) = restore(prior) { rollback_error.get_or_insert(failed); }
+        // All original bytes and the edit plan are durable before the first
+        // target rename. An interrupted batch can be explicitly rolled back.
+        let journal = recovery::Journal::begin(&workspace, &staged)?;
+        let committed = (|| {
+            // Cooperative validation narrows races, but external writers can
+            // modify files between checking and the OS rename operation.
+            for item in &staged { verify_expected(&item.path, &item.expected)?; }
+            for (index, item) in staged.iter().enumerate() {
+                verify_expected(&item.path, &item.expected)?;
+                if item.updated.as_bytes() != item.previous.as_slice() {
+                    fs::rename(&item.temp, &item.path)?;
+                    sync_parent(&item.path)?;
+                }
+                // Only available in opt-in fault-injection test builds. Exit
+                // bypasses Drop, simulating a crash after a durable rename.
+                #[cfg(feature = "test-fault-injection")]
+                if index == 0 && std::env::var_os("CONDUIT_TEST_EXIT_AFTER_FIRST_EDIT").is_some() {
+                    std::process::exit(97);
                 }
-                return Err(rollback_error.unwrap_or(error));
+                #[cfg(not(feature = "test-fault-injection"))]
+                let _ = index;
             }
+            // Test-only crash after *every* file has been published but
+            // before transaction cleanup, exercising explicit finalization.
+            #[cfg(feature = "test-fault-injection")]
+            if std::env::var_os("CONDUIT_TEST_EXIT_AFTER_ALL_EDITS").is_some() {
+                std::process::exit(98);
+            }
+            Ok::<(), io::Error>(())
+        })();
+        if let Err(error) = committed {
+            // Rollback is idempotent and hash-fenced. If a third-party writer
+            // made a conflicting edit, leave the receipt intact for inspection.
+            if let Err(rollback_error) = journal.rollback(&workspace) {
+                return Err(io::Error::other(format!(
+                    "{error}; recovery remains pending: {rollback_error}"
+                )));
+            }
+            return Err(error);
         }
+        // Do NOT roll back a successfully committed transaction merely because
+        // cleanup fails. The manifest may already be unlinked at that point.
+        // The surviving recovery receipt, when present, remains inspectable.
+        journal.complete(&workspace).map_err(|error| io::Error::other(format!(
+            "source edits committed, but recovery cleanup failed: {error}; inspect hashes before retrying"
+        )))?;
         Ok(staged.iter().map(|item| Applied {
             path: item.relative.clone(), hash: hash(item.updated.as_bytes()),
         }).collect())
@@ -102,9 +135,54 @@ fn lock_workspace(workspace: &Path) -> io::Result<Lock> {
     if !fs::symlink_metadata(&state_dir)?.file_type().is_dir() {
         return Err(io::Error::new(io::ErrorKind::PermissionDenied, "workspace state directory must not be a symlink"));
     }
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        fs::set_permissions(&state_dir, fs::Permissions::from_mode(0o700))?;
+    }
     Lock::acquire(&state_dir.join("edit.lock"))
 }
 
+/// List incomplete edit receipts without modifying source files.
+pub fn pending_edits(workspace: impl AsRef<Path>) -> io::Result<Vec<PendingEdit>> {
+    let workspace = workspace.as_ref().canonicalize()?;
+    let _lock = lock_workspace(&workspace)?;
+    recovery::list(&workspace)
+}
+
+/// Count retained recovery directories without a published manifest.
+/// These directories may contain confidential backup debris.
+pub fn orphaned_recovery_dirs(workspace: impl AsRef<Path>) -> io::Result<usize> {
+    let workspace = workspace.as_ref().canonicalize()?;
+    let _lock = lock_workspace(&workspace)?;
+    recovery::orphan_count(&workspace)
+}
+
+/// Explicitly roll back an interrupted batch. Refuses all conflicts with
+/// third-party writes and never silently chooses which version to keep.
+pub fn rollback_edit(workspace: impl AsRef<Path>, id: &str) -> io::Result<()> {
+    let workspace = workspace.as_ref().canonicalize()?;
+    let _lock = lock_workspace(&workspace)?;
+    recovery::recover(&workspace, id)
+}
+
+/// Explicitly accept a fully committed, receipt-backed edit transaction.
+/// This *only* releases retained recovery metadata and backups; it does not
+/// apply missing edits or resolve conflicts with external writers.
+pub fn finalize_edit(workspace: impl AsRef<Path>, id: &str) -> io::Result<()> {
+    let workspace = workspace.as_ref().canonicalize()?;
+    let _lock = lock_workspace(&workspace)?;
+    recovery::accept(&workspace, id)
+}
+
+/// Explicitly delete manifest-less backup debris after validating its layout.
+/// Never deletes pending transactions and never changes source files.
+pub fn prune_orphaned_edits(workspace: impl AsRef<Path>) -> io::Result<usize> {
+    let workspace = workspace.as_ref().canonicalize()?;
+    let _lock = lock_workspace(&workspace)?;
+    recovery::prune_orphans(&workspace)
+}
+
 fn validate(workspace: &Path, operations: &[Operation]) -> io::Result<Vec<Planned>> {
     if operations.is_empty() || operations.len() > MAX_OPERATIONS {
         return Err(invalid("edit batch must contain 1-128 operations"));
@@ -193,16 +271,6 @@ fn create_staged(path: &Path, bytes: &[u8], permissions: &Permissions) -> io::Re
     Err(invalid("unable to create unique edit staging file"))
 }
 
-fn restore(item: &Staged) -> io::Result<()> {
-    // If an external writer changed a committed target, do not destroy those
-    // newer bytes during best-effort rollback. Manual repair is required.
-    verify_expected(&item.path, &hash(item.updated.as_bytes()))?;
-    let temp = create_staged(&item.path, &item.previous, &item.permissions)?;
-    let result = fs::rename(&temp, &item.path).and_then(|_| sync_parent(&item.path));
-    if result.is_err() { let _ = fs::remove_file(temp); }
-    result
-}
-
 fn prepare(workspace: &Path, operation: &Operation) -> io::Result<(PathBuf, String, String)> {
     match operation {
         Operation::SearchReplace { path, expected_hash, old, new } => {
diff --git a/crates/edit/src/recovery.rs b/crates/edit/src/recovery.rs
new file mode 100644
index 0000000000000000000000000000000000000000..3351340f09169d431c91bbbfc62f0e80507abf22
--- /dev/null
+++ b/crates/edit/src/recovery.rs
@@ -0,0 +1,649 @@
+//! Persisted, inspectable edit recovery receipts.
+//!
+//! A receipt is made durable *before* the first target rename. Rollback is an
+//! explicit, hash-fenced operation; it never overwrites a third-party change.
+//! This does NOT make concurrent readers see a multi-file atomic transaction.
+use super::{
+    checked_path, create_staged, hash, invalid, read_bounded, sync_parent, Staged, MAX_EDIT_FILE_BYTES,
+    MAX_OPERATIONS,
+};
+use serde::{Deserialize, Serialize};
+use std::{
+    collections::HashSet,
+    fs::{self, DirBuilder, File, OpenOptions, Permissions},
+    io::{self, Read, Write},
+    path::{Path, PathBuf},
+    sync::atomic::{AtomicU64, Ordering},
+    time::{SystemTime, UNIX_EPOCH},
+};
+
+static NEXT_TX: AtomicU64 = AtomicU64::new(0);
+const MAX_MANIFEST_BYTES: usize = 64 * 1024;
+
+#[derive(Debug, Clone, Serialize, Deserialize)]
+#[serde(deny_unknown_fields)]
+struct Entry {
+    path: PathBuf,
+    original_hash: String,
+    updated_hash: String,
+    backup: String,
+    staged: PathBuf,
+    readonly: bool,
+    unix_mode: Option<u32>,
+}
+
+#[derive(Debug, Serialize, Deserialize)]
+#[serde(deny_unknown_fields)]
+struct Receipt {
+    version: u32,
+    entries: Vec<Entry>,
+}
+
+pub(super) struct Journal {
+    directory: PathBuf,
+    receipt: Receipt,
+}
+
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct PendingEdit {
+    pub id: String,
+    pub files: usize,
+    pub original: usize,
+    pub updated: usize,
+    pub conflicting: usize,
+}
+
+fn recovery_root(workspace: &Path) -> io::Result<PathBuf> {
+    let path = workspace.join(".conduit").join("edit-recovery");
+    fs::create_dir_all(&path)?;
+    if !fs::symlink_metadata(&path)?.file_type().is_dir() {
+        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "edit recovery directory must not be a symlink"));
+    }
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
+    }
+    Ok(path)
+}
+
+fn sync_dir(directory: &Path) -> io::Result<()> {
+    #[cfg(unix)]
+    File::open(directory)?.sync_all()?;
+    #[cfg(not(unix))]
+    let _ = directory;
+    Ok(())
+}
+
+fn valid_id(id: &str) -> bool {
+    id.starts_with("txn-") && id.len() <= 100 &&
+        id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
+}
+
+fn recorded_mode(permissions: &Permissions) -> Option<u32> {
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        Some(permissions.mode())
+    }
+    #[cfg(not(unix))]
+    {
+        let _ = permissions;
+        None
+    }
+}
+
+fn restored_permissions(entry: &Entry, current: &Permissions) -> Permissions {
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        if let Some(mode) = entry.unix_mode {
+            return Permissions::from_mode(mode);
+        }
+    }
+    let mut permissions = current.clone();
+    permissions.set_readonly(entry.readonly);
+    permissions
+}
+
+fn valid_hash(hash: &str) -> bool {
+    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
+}
+
+fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
+    let mut options = OpenOptions::new();
+    options.write(true).create_new(true);
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::OpenOptionsExt;
+        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
+    }
+    let mut file = options.open(path)?;
+    file.write_all(bytes)?;
+    file.sync_all()
+}
+
+fn read_private(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
+    let mut options = OpenOptions::new();
+    options.read(true);
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::OpenOptionsExt;
+        options.custom_flags(libc::O_NOFOLLOW);
+    }
+    let file = options.open(path)?;
+    let metadata = file.metadata()?;
+    if !metadata.is_file() || metadata.len() > limit as u64 {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid recovery object type or size"));
+    }
+    let mut bytes = Vec::new();
+    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
+    if bytes.len() > limit {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "recovery object exceeds bound"));
+    }
+    Ok(bytes)
+}
+
+fn validate_receipt(_workspace: &Path, receipt: &Receipt) -> io::Result<()> {
+    if receipt.version != 1 || receipt.entries.is_empty() || receipt.entries.len() > MAX_OPERATIONS {
+        return Err(invalid("unsupported or empty edit recovery receipt"));
+    }
+    let mut paths = HashSet::new();
+    for (index, entry) in receipt.entries.iter().enumerate() {
+        if entry.backup != format!("{index}.backup") ||
+            !valid_hash(&entry.original_hash) || !valid_hash(&entry.updated_hash) {
+            return Err(invalid("malformed recovery receipt hashes or backup name"));
+        }
+        // Staged files must live alongside their exact target, with a
+        // predictable private-temp naming convention; receipts never gain a
+        // second arbitrary path that recovery can delete.
+        let stage = super::safe_relative(&entry.staged)?;
+        let name = stage.file_name().and_then(|name| name.to_str())
+            .ok_or_else(|| invalid("invalid recovery staging filename"))?;
+        if !name.starts_with(".conduit-edit-") || !name.ends_with(".tmp") ||
+            stage.parent() != entry.path.parent() {
+            return Err(invalid("recovery staging path does not match its target"));
+        }
+        // Lexical receipt validation must work even if an interrupted edit's
+        // target has been deleted. Canonical/existence checks happen before
+        // accessing any source or applying rollback.
+        let path = super::safe_relative(&entry.path)?;
+        if !paths.insert(path.clone()) || path.components().any(|component| {
+            matches!(component, std::path::Component::Normal(name) if name == ".git" || name == ".conduit")
+        }) {
+            return Err(invalid("malformed recovery target"));
+        }
+    }
+    Ok(())
+}
+
+fn read_journal(workspace: &Path, directory: PathBuf) -> io::Result<Journal> {
+    if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
+        return Err(invalid("recovery transaction is not a real directory"));
+    }
+    let bytes = read_private(&directory.join("manifest.json"), MAX_MANIFEST_BYTES)?;
+    let receipt: Receipt = serde_json::from_slice(&bytes)
+        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
+    validate_receipt(workspace, &receipt)?;
+    Ok(Journal { directory, receipt })
+}
+
+fn transaction_directories(workspace: &Path) -> io::Result<Vec<PathBuf>> {
+    let mut found = Vec::new();
+    for entry in fs::read_dir(recovery_root(workspace)?)? {
+        let entry = entry?;
+        let name = entry.file_name();
+        let Some(id) = name.to_str() else { return Err(invalid("non-UTF8 recovery entry")); };
+        if !valid_id(id) {
+            return Err(invalid("unexpected edit-recovery directory entry"));
+        }
+        if !entry.file_type()?.is_dir() {
+            return Err(invalid("recovery entry is not a directory"));
+        }
+        let manifest = entry.path().join("manifest.json");
+        match fs::symlink_metadata(manifest) {
+            Ok(_) => found.push(entry.path()),
+            Err(error) if error.kind() == io::ErrorKind::NotFound => {
+                // A crash before publishing the durable manifest leaves only
+                // uncommitted backup debris. No source mutation started then.
+            }
+            Err(error) => return Err(error),
+        }
+    }
+    found.sort();
+    Ok(found)
+}
+
+/// Recovery directories without a manifest can only be pre-commit debris or
+/// debris left after the manifest was durably removed. They are never deleted
+/// automatically because retained source backups may be incident evidence.
+pub(super) fn orphan_count(workspace: &Path) -> io::Result<usize> {
+    let mut orphans = 0;
+    for entry in fs::read_dir(recovery_root(workspace)?)? {
+        let entry = entry?;
+        let name = entry.file_name();
+        let Some(id) = name.to_str() else { return Err(invalid("non-UTF8 recovery entry")); };
+        if !valid_id(id) || !entry.file_type()?.is_dir() {
+            return Err(invalid("invalid recovery entry"));
+        }
+        match fs::symlink_metadata(entry.path().join("manifest.json")) {
+            Ok(_) => {}
+            Err(error) if error.kind() == io::ErrorKind::NotFound => orphans += 1,
+            Err(error) => return Err(error),
+        }
+    }
+    Ok(orphans)
+}
+
+pub(super) fn ensure_no_pending(workspace: &Path) -> io::Result<()> {
+    if !transaction_directories(workspace)?.is_empty() {
+        return Err(io::Error::new(io::ErrorKind::WouldBlock,
+            "unfinished edit transaction; inspect with recover-edits --list before writing"));
+    }
+    Ok(())
+}
+
+impl Journal {
+    pub(super) fn begin(workspace: &Path, staged: &[Staged]) -> io::Result<Self> {
+        ensure_no_pending(workspace)?;
+        let base = recovery_root(workspace)?;
+        let mut created = None;
+        for _ in 0..16 {
+            let nonce = SystemTime::now().duration_since(UNIX_EPOCH)
+                .unwrap_or_default().as_nanos();
+            let serial = NEXT_TX.fetch_add(1, Ordering::Relaxed);
+            let directory = base.join(format!("txn-{nonce}-{}-{serial}", std::process::id()));
+            #[allow(unused_mut)]
+            let mut builder = DirBuilder::new();
+            #[cfg(unix)]
+            {
+                use std::os::unix::fs::DirBuilderExt;
+                builder.mode(0o700);
+            }
+            match builder.create(&directory) {
+                Ok(()) => { created = Some(directory); break; }
+                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
+                Err(error) => return Err(error),
+            }
+        }
+        let directory = created.ok_or_else(|| invalid("unable to allocate unique recovery transaction"))?;
+        let prepare = (|| {
+            sync_dir(&base)?;
+            let mut entries = Vec::with_capacity(staged.len());
+            for (index, item) in staged.iter().enumerate() {
+                let relative = item.path.strip_prefix(workspace)
+                    .map_err(|_| invalid("edit target outside workspace"))?.to_path_buf();
+                let backup = format!("{index}.backup");
+                let staged = item.temp.strip_prefix(workspace)
+                    .map_err(|_| invalid("edit staging path outside workspace"))?.to_path_buf();
+                let permissions = fs::metadata(&item.path)?.permissions();
+                write_private(&directory.join(&backup), &item.previous)?;
+                entries.push(Entry {
+                    path: relative,
+                    original_hash: item.expected.clone(),
+                    updated_hash: hash(item.updated.as_bytes()),
+                    backup,
+                    staged,
+                    readonly: permissions.readonly(),
+                    unix_mode: recorded_mode(&permissions),
+                });
+            }
+            let receipt = Receipt { version: 1, entries };
+            validate_receipt(workspace, &receipt)?;
+            let data = serde_json::to_vec(&receipt).map_err(io::Error::other)?;
+            if data.len() > MAX_MANIFEST_BYTES { return Err(invalid("edit recovery receipt too large")); }
+            write_private(&directory.join("manifest.json"), &data)?;
+            sync_dir(&directory)?;
+            Ok(Self { directory: directory.clone(), receipt })
+        })();
+        if prepare.is_err() {
+            // No source rename is possible before begin has completed.
+            let _ = fs::remove_dir_all(&directory);
+            let _ = sync_dir(&base);
+        }
+        prepare
+    }
+
+    fn entry_target(workspace: &Path, entry: &Entry) -> io::Result<PathBuf> {
+        let target = checked_path(workspace, &entry.path)?;
+        if target.strip_prefix(workspace).ok() != Some(entry.path.as_path()) {
+            return Err(invalid("recovery target identity changed"));
+        }
+        Ok(target)
+    }
+
+    /// Accept a fully applied batch after verifying *all* final target hashes,
+    /// every original backup, and the structural receipt. No source files are
+    /// changed; only retained recovery artifacts are removed. The check is
+    /// protected against cooperating Conduit edits by the workspace lock.
+    pub(super) fn finalize(&self, workspace: &Path) -> io::Result<()> {
+        validate_receipt(workspace, &self.receipt)?;
+        for entry in &self.receipt.entries {
+            let target = Self::entry_target(workspace, entry)?;
+            if hash(&read_bounded(&target)?) != entry.updated_hash {
+                return Err(io::Error::new(io::ErrorKind::WouldBlock,
+                    "cannot finalize: not all edit targets match the committed hashes"));
+            }
+            let original = read_private(&self.directory.join(&entry.backup), MAX_EDIT_FILE_BYTES)?;
+            if hash(&original) != entry.original_hash {
+                return Err(io::Error::new(io::ErrorKind::InvalidData,
+                    "cannot finalize: edit recovery backup hash mismatch"));
+            }
+        }
+        self.complete(workspace)
+    }
+
+    pub(super) fn rollback(&self, workspace: &Path) -> io::Result<()> {
+        validate_receipt(workspace, &self.receipt)?;
+        // Preflight *every* target and backup before touching any target. A
+        // missing backup or a third-party edit blocks rollback without loss.
+        let mut restores = Vec::new();
+        for entry in &self.receipt.entries {
+            let target = Self::entry_target(workspace, entry)?;
+            let current = hash(&read_bounded(&target)?);
+            if current == entry.original_hash { continue; }
+            if current != entry.updated_hash {
+                return Err(io::Error::new(io::ErrorKind::WouldBlock,
+                    "recovery conflicts with a third-party source change"));
+            }
+            let original = read_private(&self.directory.join(&entry.backup), MAX_EDIT_FILE_BYTES)?;
+            if hash(&original) != entry.original_hash {
+                return Err(io::Error::new(io::ErrorKind::InvalidData, "recovery backup digest mismatch"));
+            }
+            let current_permissions = fs::metadata(&target)?.permissions();
+            restores.push((target, original,
+                restored_permissions(entry, &current_permissions), entry.updated_hash.clone()));
+        }
+        let mut staged = Vec::new();
+        let result = (|| {
+            for (target, bytes, perms, hash) in &restores {
+                let temp = create_staged(target, bytes, perms)?;
+                staged.push((target.clone(), temp, hash.clone()));
+            }
+            for (target, temp, expected) in &staged {
+                // An uncooperative writer can still race the rename; no
+                // claimed serializability against hostile filesystem actors.
+                if hash(&read_bounded(target)?) != *expected {
+                    return Err(io::Error::new(io::ErrorKind::WouldBlock,
+                        "source changed while recovering; manual resolution required"));
+                }
+                fs::rename(temp, target)?;
+                sync_parent(target)?;
+            }
+            self.complete(workspace)
+        })();
+        for (_, temp, _) in &staged { let _ = fs::remove_file(temp); }
+        result
+    }
+
+    pub(super) fn complete(&self, workspace: &Path) -> io::Result<()> {
+        // Clear retained staged updates *before* unlinking the receipt.
+        // A modified or replaced stage is treated as a conflict, not deleted.
+        for entry in &self.receipt.entries {
+            let stage = workspace.join(&entry.staged);
+            match fs::symlink_metadata(&stage) {
+                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
+                Err(error) => return Err(error),
+                Ok(meta) if !meta.file_type().is_file() => {
+                    return Err(invalid("unexpected edit staging object type"));
+                }
+                Ok(_) => {}
+            }
+            if hash(&read_bounded(&stage)?) != entry.updated_hash {
+                return Err(io::Error::new(io::ErrorKind::WouldBlock,
+                    "edit staging content differs from recovery receipt"));
+            }
+            fs::remove_file(&stage)?;
+            sync_parent(&stage)?;
+        }
+        // Unlink the manifest first. Once its removal has been synced, an
+        // interrupted backup cleanup is debris, never a false pending edit.
+        fs::remove_file(self.directory.join("manifest.json"))?;
+        sync_dir(&self.directory)?;
+        for entry in &self.receipt.entries {
+            fs::remove_file(self.directory.join(&entry.backup))?;
+        }
+        sync_dir(&self.directory)?;
+        fs::remove_dir(&self.directory)?;
+        sync_parent(&self.directory)
+    }
+}
+
+pub(super) fn list(workspace: &Path) -> io::Result<Vec<PendingEdit>> {
+    let mut pending = Vec::new();
+    for directory in transaction_directories(workspace)? {
+        let journal = read_journal(workspace, directory)?;
+        let mut summary = PendingEdit {
+            id: journal.directory.file_name()
+                .ok_or_else(|| invalid("recovery transaction has no name"))?
+                .to_string_lossy().into_owned(),
+            files: journal.receipt.entries.len(), original: 0, updated: 0, conflicting: 0,
+        };
+        for entry in &journal.receipt.entries {
+            let state = Journal::entry_target(workspace, entry)
+                .and_then(|target| read_bounded(&target))
+                .map(|bytes| hash(&bytes));
+            match state {
+                Ok(current) if current == entry.original_hash => summary.original += 1,
+                Ok(current) if current == entry.updated_hash => summary.updated += 1,
+                _ => summary.conflicting += 1,
+            }
+        }
+        pending.push(summary);
+    }
+    Ok(pending)
+}
+
+fn get_transaction(workspace: &Path, id: &str) -> io::Result<Journal> {
+    if !valid_id(id) { return Err(invalid("invalid edit transaction ID")); }
+    let pending = transaction_directories(workspace)?;
+    let path = recovery_root(workspace)?.join(id);
+    if !pending.contains(&path) {
+        return Err(io::Error::new(io::ErrorKind::NotFound, "edit transaction not found"));
+    }
+    read_journal(workspace, path)
+}
+
+pub(super) fn recover(workspace: &Path, id: &str) -> io::Result<()> {
+    get_transaction(workspace, id)?.rollback(workspace)
+}
+
+pub(super) fn accept(workspace: &Path, id: &str) -> io::Result<()> {
+    get_transaction(workspace, id)?.finalize(workspace)
+}
+
+/// Remove explicitly selected, receipt-less backup debris. A manifest must
+/// have been durably absent before source mutation started or after a commit
+/// finished. Never delete a valid pending receipt. All orphan contents are
+/// inspected before ANY directory is cleaned; unknown objects fail closed.
+pub(super) fn prune_orphans(workspace: &Path) -> io::Result<usize> {
+    ensure_no_pending(workspace)?;
+    let base = recovery_root(workspace)?;
+    let mut orphans: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
+    for entry in fs::read_dir(&base)? {
+        let entry = entry?;
+        let name = entry.file_name();
+        let id = name.to_str().ok_or_else(|| invalid("non-UTF8 edit recovery directory"))?;
+        if !valid_id(id) || !entry.file_type()?.is_dir() {
+            return Err(invalid("unexpected edit recovery entry"));
+        }
+        match fs::symlink_metadata(entry.path().join("manifest.json")) {
+            Ok(_) => return Err(invalid("pending transaction forbids orphan cleanup")),
+            Err(error) if error.kind() == io::ErrorKind::NotFound => {},
+            Err(error) => return Err(error),
+        }
+        let mut backups = Vec::new();
+        for object in fs::read_dir(entry.path())? {
+            let object = object?;
+            let name = object.file_name();
+            let file_name = name.to_str().ok_or_else(|| invalid("non-UTF8 backup name"))?;
+            let index = file_name.strip_suffix(".backup")
+                .and_then(|digits| digits.parse::<usize>().ok());
+            if !index.is_some_and(|index| index < MAX_OPERATIONS && file_name == format!("{index}.backup")) ||
+                !object.file_type()?.is_file() || object.metadata()?.len() > MAX_EDIT_FILE_BYTES as u64 {
+                return Err(invalid("unexpected object in orphaned transaction; refusing deletion"));
+            }
+            backups.push(object.path());
+        }
+        orphans.push((entry.path(), backups));
+    }
+    for (directory, backups) in &orphans {
+        for file in backups { fs::remove_file(file)?; }
+        sync_dir(directory)?;
+        fs::remove_dir(directory)?;
+        sync_dir(&base)?;
+    }
+    Ok(orphans.len())
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::{lock_workspace, preview, Operation};
+
+    fn fixture(label: &str) -> PathBuf {
+        let root = std::env::temp_dir().join(format!(
+            "conduit-recovery-{label}-{}-{}",
+            std::process::id(), NEXT_TX.fetch_add(1, Ordering::Relaxed),
+        ));
+        fs::create_dir_all(&root).unwrap();
+        root
+    }
+
+    fn staged(root: &Path, name: &str, original: &str, changed: &str) -> Staged {
+        let path = root.join(name);
+        fs::write(&path, original).unwrap();
+        let permissions = fs::metadata(&path).unwrap().permissions();
+        Staged {
+            relative: name.into(),
+            path: path.clone(),
+            temp: create_staged(&path, changed.as_bytes(), &permissions).unwrap(),
+            expected: hash(original.as_bytes()),
+            updated: changed.into(),
+            previous: original.as_bytes().to_vec(),
+        }
+    }
+
+    #[test]
+    fn interrupted_partial_commit_can_be_rolled_back_after_reopen() {
+        let root = fixture("partial");
+        let first = staged(&root, "a.rs", "before a", "after a");
+        let second = staged(&root, "b.rs", "before b", "after b");
+        let lock = lock_workspace(&root).unwrap();
+        let journal = Journal::begin(&root, &[first, second]).unwrap();
+        let id = journal.directory.file_name().unwrap().to_str().unwrap().to_owned();
+        // Simulate the first rename committing, then abruptly losing the
+        // process before the second commit or receipt cleanup.
+        fs::write(root.join("a.rs"), "after a").unwrap();
+        drop(journal);
+        drop(lock);
+
+        let summaries = super::super::pending_edits(&root).unwrap();
+        assert_eq!(summaries.len(), 1);
+        assert_eq!(summaries[0].id, id);
+        assert_eq!(summaries[0].updated, 1);
+        assert_eq!(summaries[0].original, 1);
+        let edit = [Operation::SearchReplace {
+            path: "b.rs".into(), expected_hash: hash(b"before b"),
+            old: "before".into(), new: "modified".into(),
+        }];
+        assert!(preview(&root, &edit).is_err(), "pending receipt fences all new edits");
+        super::super::rollback_edit(&root, &id).unwrap();
+        assert_eq!(fs::read_to_string(root.join("a.rs")).unwrap(), "before a");
+        assert_eq!(fs::read_to_string(root.join("b.rs")).unwrap(), "before b");
+        assert!(super::super::pending_edits(&root).unwrap().is_empty());
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn third_party_change_blocks_recovery_without_overwriting_it() {
+        let root = fixture("conflict");
+        let first = staged(&root, "a.rs", "before", "after");
+        let lock = lock_workspace(&root).unwrap();
+        let journal = Journal::begin(&root, &[first]).unwrap();
+        let id = journal.directory.file_name().unwrap().to_str().unwrap().to_owned();
+        fs::write(root.join("a.rs"), "third-party update").unwrap();
+        drop(journal);
+        drop(lock);
+        let summaries = super::super::pending_edits(&root).unwrap();
+        assert_eq!(summaries[0].conflicting, 1);
+        assert!(super::super::rollback_edit(&root, &id).is_err());
+        assert_eq!(fs::read_to_string(root.join("a.rs")).unwrap(), "third-party update");
+        assert_eq!(super::super::pending_edits(&root).unwrap().len(), 1);
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn fully_committed_interruption_can_be_finalized_without_rollback() {
+        let root = fixture("accept");
+        let first = staged(&root, "a.rs", "before a", "after a");
+        let second = staged(&root, "b.rs", "before b", "after b");
+        let lock = lock_workspace(&root).unwrap();
+        let journal = Journal::begin(&root, &[first, second]).unwrap();
+        let id = journal.directory.file_name().unwrap().to_str().unwrap().to_owned();
+        // Simulate a crash after both files were published, before cleanup.
+        fs::write(root.join("a.rs"), "after a").unwrap();
+        fs::write(root.join("b.rs"), "after b").unwrap();
+        drop(journal);
+        drop(lock);
+        super::super::finalize_edit(&root, &id).unwrap();
+        assert_eq!(fs::read_to_string(root.join("a.rs")).unwrap(), "after a");
+        assert_eq!(fs::read_to_string(root.join("b.rs")).unwrap(), "after b");
+        assert_eq!(super::super::orphaned_recovery_dirs(&root).unwrap(), 0);
+        assert!(super::super::pending_edits(&root).unwrap().is_empty());
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn finalize_fails_closed_on_partial_commit_or_tampered_backup() {
+        let root = fixture("accept-partial");
+        let first = staged(&root, "a.rs", "before a", "after a");
+        let second = staged(&root, "b.rs", "before b", "after b");
+        let lock = lock_workspace(&root).unwrap();
+        let journal = Journal::begin(&root, &[first, second]).unwrap();
+        let id = journal.directory.file_name().unwrap().to_str().unwrap().to_owned();
+        fs::write(root.join("a.rs"), "after a").unwrap();
+        assert!(journal.finalize(&root).is_err());
+        assert_eq!(fs::read_to_string(root.join("a.rs")).unwrap(), "after a");
+        fs::write(root.join("b.rs"), "after b").unwrap();
+        fs::write(journal.directory.join("0.backup"), "malicious").unwrap();
+        assert!(journal.finalize(&root).is_err());
+        assert!(root.join(".conduit/edit-recovery").join(id).join("manifest.json").exists());
+        drop(journal);
+        drop(lock);
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn orphan_prune_is_explicit_and_rejects_unrecognized_files() {
+        let root = fixture("orphan-prune");
+        let recovery = recovery_root(&root).unwrap();
+        let orphan = recovery.join("txn-10-1-0");
+        fs::create_dir(&orphan).unwrap();
+        fs::write(orphan.join("0.backup"), "retained backup").unwrap();
+        assert_eq!(super::super::orphaned_recovery_dirs(&root).unwrap(), 1);
+        fs::write(orphan.join("unknown.txt"), "not ours").unwrap();
+        assert!(super::super::prune_orphaned_edits(&root).is_err());
+        assert!(orphan.join("0.backup").exists());
+        fs::remove_file(orphan.join("unknown.txt")).unwrap();
+        assert_eq!(super::super::prune_orphaned_edits(&root).unwrap(), 1);
+        assert!(!orphan.exists());
+        let _ = fs::remove_dir_all(root);
+    }
+
+    #[test]
+    fn missing_backup_blocks_rollback_of_updated_file() {
+        let root = fixture("backup");
+        let first = staged(&root, "a.rs", "before", "after");
+        let lock = lock_workspace(&root).unwrap();
+        let journal = Journal::begin(&root, &[first]).unwrap();
+        let id = journal.directory.file_name().unwrap().to_str().unwrap().to_owned();
+        fs::write(root.join("a.rs"), "after").unwrap();
+        fs::remove_file(journal.directory.join("0.backup")).unwrap();
+        drop(journal);
+        drop(lock);
+        assert!(super::super::rollback_edit(&root, &id).is_err());
+        assert_eq!(fs::read_to_string(root.join("a.rs")).unwrap(), "after");
+        let _ = fs::remove_dir_all(root);
+    }
+}
diff --git a/crates/scope/src/lib.rs b/crates/scope/src/lib.rs
index c947551b4b99bb8f01272da2d803a4c3c5163981..f1a03c1e81d0ad77d93949788782a018a9743d1a 100644
--- a/crates/scope/src/lib.rs
+++ b/crates/scope/src/lib.rs
@@ -7,6 +7,8 @@ use std::{
     fs::{self, File, OpenOptions},
     io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
     path::{Path, PathBuf},
+    sync::atomic::{AtomicU64, Ordering},
+    time::{SystemTime, UNIX_EPOCH},
 };
 
 #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
@@ -38,6 +40,53 @@ impl fmt::Display for Event {
 const MAX_LOG_BYTES: u64 = 64 * 1024 * 1024;
 const MAX_RECORD_BYTES: usize = 8 * 1024;
 const MAX_MODULE_BYTES: usize = 512;
+static NEXT_TAIL: AtomicU64 = AtomicU64::new(0);
+
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum TailRepair {
+    Unchanged,
+    /// Final record was complete JSON but lacked only a trailing newline.
+    CompletedRecord,
+    /// Malformed fragment preserved verbatim before truncation.
+    Quarantined { backup: PathBuf },
+}
+
+fn quarantine_tail(path: &Path, fragment: &[u8]) -> io::Result<PathBuf> {
+    let parent = path.parent().ok_or_else(|| io::Error::other("session file has no parent"))?;
+    for _ in 0..16 {
+        let mut name = path.file_name().ok_or_else(|| io::Error::other("session file has no name"))?.to_os_string();
+        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
+        let serial = NEXT_TAIL.fetch_add(1, Ordering::Relaxed);
+        name.push(format!(".torn-{}-{nonce}-{serial}.bak", std::process::id()));
+        let backup = parent.join(name);
+        let mut options = OpenOptions::new();
+        options.write(true).create_new(true);
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::OpenOptionsExt;
+            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
+        }
+        let mut output = match options.open(&backup) {
+            Ok(file) => file,
+            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
+            Err(error) => return Err(error),
+        };
+        let write: io::Result<()> = (|| {
+            output.write_all(fragment)?;
+            output.sync_all()?;
+            #[cfg(unix)]
+            File::open(parent)?.sync_all()?;
+            Ok(())
+        })();
+        if let Err(error) = write {
+            let _ = fs::remove_file(&backup);
+            return Err(error);
+        }
+        return Ok(backup);
+    }
+    Err(io::Error::new(io::ErrorKind::AlreadyExists, "unable to allocate a unique session tail backup"))
+}
+
 
 pub struct EventLog {
     path: PathBuf,
@@ -173,10 +222,11 @@ impl EventLog {
         Ok(lines)
     }
 
-    /// Explicitly discard only a final unterminated record after validating
-    /// that all preceding complete records remain valid. Never repairs a
-    /// completed malformed line or infers missing accounting data.
-    pub fn repair_partial_tail(&self) -> io::Result<bool> {
+    /// Safely handle an unterminated JSONL tail. Complete JSON events only
+    /// need a newline and are preserved. Incomplete or malformed tails are
+    /// backed up to a private synced file BEFORE truncation. A completed
+    /// malformed interior record is never rewritten or repaired.
+    pub fn repair_tail(&self) -> io::Result<TailRepair> {
         let mut options = OpenOptions::new();
         options.read(true).write(true);
         #[cfg(unix)]
@@ -194,27 +244,45 @@ impl EventLog {
             file.seek(SeekFrom::Start(0))?;
             let mut bytes = Vec::new();
             file.read_to_end(&mut bytes)?;
-            if bytes.is_empty() || bytes.last() == Some(&b'\n') { return Ok(false); }
+            if bytes.is_empty() || bytes.last() == Some(&b'\n') { return Ok(TailRepair::Unchanged); }
             let prefix_end = bytes.iter().rposition(|byte| *byte == b'\n')
                 .map_or(0, |index| index + 1);
             if prefix_end != 0 {
-                // Exclude only the final terminator, not blank middle records.
                 for (index, line) in bytes[..prefix_end - 1].split(|byte| *byte == b'\n').enumerate() {
                     if line.len() + 1 > MAX_RECORD_BYTES || serde_json::from_slice::<Event>(line).is_err() {
                         return Err(io::Error::new(io::ErrorKind::InvalidData,
-                            format!("refusing to discard tail because preceding record {} is invalid", index + 1)));
+                            format!("refusing to modify tail because preceding record {} is invalid", index + 1)));
                     }
                 }
             }
+            let tail = &bytes[prefix_end..];
+            // The only missing byte may be the JSONL terminator. Appending it
+            // preserves the event and its accounting contribution.
+            if tail.len() + 1 <= MAX_RECORD_BYTES &&
+                serde_json::from_slice::<Event>(tail).is_ok() {
+                if length == MAX_LOG_BYTES {
+                    return Err(io::Error::new(io::ErrorKind::InvalidData, "cannot append record delimiter beyond log limit"));
+                }
+                file.seek(SeekFrom::End(0))?;
+                file.write_all(b"\n")?;
+                file.sync_all()?;
+                return Ok(TailRepair::CompletedRecord);
+            }
+            let backup = quarantine_tail(&self.path, tail)?;
             file.set_len(prefix_end as u64)?;
             file.sync_all()?;
-            Ok(true)
+            Ok(TailRepair::Quarantined { backup })
         })();
         let unlocked = FileExt::unlock(&file);
         unlocked?;
         result
     }
 
+    /// Backward-compatible boolean API: true when any tail was repaired.
+    pub fn repair_partial_tail(&self) -> io::Result<bool> {
+        Ok(!matches!(self.repair_tail()?, TailRepair::Unchanged))
+    }
+
 }
 
 #[cfg(test)]
@@ -294,11 +362,55 @@ mod recovery_tests {
         file.write_all(b"{\"type\":\"sav").unwrap();
         assert_eq!(log.totals().unwrap_err().kind(), io::ErrorKind::InvalidData);
         assert_eq!(log.append(event.clone()).unwrap_err().kind(), io::ErrorKind::InvalidData);
-        assert!(log.repair_partial_tail().unwrap());
-        assert!(!log.repair_partial_tail().unwrap());
+        let backup = match log.repair_tail().unwrap() {
+            TailRepair::Quarantined { backup } => backup,
+            other => panic!("expected a quarantined invalid fragment, got {other:?}"),
+        };
+        assert_eq!(fs::read(&backup).unwrap(), b"{\"type\":\"sav");
+        assert_eq!(log.repair_tail().unwrap(), TailRepair::Unchanged);
         log.append(event).unwrap();
         assert_eq!(log.totals().unwrap(), (4, 2));
         let _ = fs::remove_file(path);
+        let _ = fs::remove_file(backup);
+    }
+
+    #[test]
+    fn valid_unterminated_event_is_finished_without_losing_accounting() {
+        let path = std::env::temp_dir().join(format!("conduit-tail-valid-{}.jsonl", std::process::id()));
+        let log = EventLog::open(&path).unwrap();
+        let event = Event::Savings(Savings {
+            module: "preserved".into(), raw_tokens: 11, presented_tokens: 7, measured: false,
+        });
+        let bytes = serde_json::to_vec(&event).unwrap();
+        fs::write(&path, bytes).unwrap();
+        assert_eq!(log.repair_tail().unwrap(), TailRepair::CompletedRecord);
+        assert_eq!(log.totals().unwrap(), (11, 7));
+        assert_eq!(log.repair_tail().unwrap(), TailRepair::Unchanged);
+        let _ = fs::remove_file(path);
+    }
+
+    #[test]
+    fn malformed_tail_is_quarantined_verbatim_before_truncation() {
+        let path = std::env::temp_dir().join(format!("conduit-tail-backup-{}.jsonl", std::process::id()));
+        let log = EventLog::open(&path).unwrap();
+        let event = Event::Savings(Savings { module: "good".into(), raw_tokens: 3, presented_tokens: 2, measured: false });
+        log.append(event).unwrap();
+        let mut handle = OpenOptions::new().append(true).open(&path).unwrap();
+        handle.write_all(b"{bad-final-fragment").unwrap();
+        drop(handle);
+        let backup = match log.repair_tail().unwrap() {
+            TailRepair::Quarantined { backup } => backup,
+            other => panic!("expected quarantined fragment, got {other:?}"),
+        };
+        assert_eq!(fs::read(&backup).unwrap(), b"{bad-final-fragment");
+        assert_eq!(log.totals().unwrap(), (3, 2));
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::PermissionsExt;
+            assert_eq!(fs::metadata(&backup).unwrap().permissions().mode() & 0o077, 0);
+        }
+        let _ = fs::remove_file(path);
+        let _ = fs::remove_file(backup);
     }
 
     #[test]
diff --git a/crates/store/src/lib.rs b/crates/store/src/lib.rs
index 8fb595dd69db25f5457e96703e596878e1b3cdf2..675665a4dd4d701f00cac5f1fc4090af2ca0ab95 100644
--- a/crates/store/src/lib.rs
+++ b/crates/store/src/lib.rs
@@ -8,6 +8,7 @@ use std::{
 };
 
 const MAX_BLOB_BYTES: usize = 64 * 1024 * 1024;
+const MAX_WINDOW_BYTES: usize = 1024 * 1024;
 static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
 
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -56,10 +57,10 @@ impl BlobStore {
         let path = directory.join(&hash);
         if path.exists() {
             // An existing digest name is not proof of intact content.
-            verify_blob(&path, &hash)?;
+            verify_blob_window(&path, &hash, Some((0, 0)))?;
         } else {
             self.publish(&directory, &path, data)?;
-            verify_blob(&path, &hash)?;
+            verify_blob_window(&path, &hash, Some((0, 0)))?;
         }
         Ok(BlobRef {
             handle: handle_for_hash(&hash),
@@ -148,10 +149,10 @@ impl BlobStore {
                     report.problem(format!("unexpected blob entry {shard}/{name}"));
                     continue;
                 }
-                match verify_blob(&entry.path(), name) {
-                    Ok(bytes) => {
+                match verify_blob_window(&entry.path(), name, Some((0, 0))) {
+                    Ok((_, bytes)) => {
                         report.checked_blobs = report.checked_blobs.saturating_add(1);
-                        report.verified_bytes = report.verified_bytes.saturating_add(bytes.len() as u64);
+                        report.verified_bytes = report.verified_bytes.saturating_add(bytes);
                     }
                     Err(error) => report.problem(format!("unverifiable blob {shard}/{name}: {error}")),
                 }
@@ -160,7 +161,33 @@ impl BlobStore {
         Ok(report)
     }
 
+    /// Hash-verify the entire object with fixed-size buffers. Inspecting a
+    /// 64 MiB blob does not allocate a second 64 MiB copy.
+    pub fn inspect(&self, handle: &str) -> io::Result<BlobRef> {
+        let (path, hash) = self.resolve(handle)?;
+        let (_, bytes) = verify_blob_window(&path, &hash, Some((0, 0)))?;
+        Ok(BlobRef { handle: handle.to_owned(), hash, bytes })
+    }
+
+    /// Retrieve a bounded byte window *only after* validating the complete
+    /// object's digest, including all bytes outside the requested window.
+    /// The returned vector cannot exceed 1 MiB even for a 64 MiB object.
+    pub fn get_range(&self, handle: &str, offset: usize, length: usize) -> io::Result<Vec<u8>> {
+        if length == 0 || length > MAX_WINDOW_BYTES {
+            return Err(invalid("window length must be 1-1048576 bytes"));
+        }
+        let end = offset.checked_add(length).ok_or_else(|| invalid("window offset overflow"))?;
+        let (path, hash) = self.resolve(handle)?;
+        let (bytes, _) = verify_blob_window(&path, &hash, Some((offset, end)))?;
+        Ok(bytes)
+    }
+
     pub fn get(&self, handle: &str) -> io::Result<Vec<u8>> {
+        let (path, hash) = self.resolve(handle)?;
+        Ok(verify_blob_window(&path, &hash, None)?.0)
+    }
+
+    fn resolve(&self, handle: &str) -> io::Result<(PathBuf, String)> {
         let encoded = handle
             .strip_prefix("cx:o:")
             .ok_or_else(|| invalid("invalid output handle"))?;
@@ -177,24 +204,17 @@ impl BlobStore {
         let mut match_path = None;
         for entry in fs::read_dir(&directory)? {
             let entry = entry?;
-            if !entry.file_type()?.is_file() {
-                continue;
-            }
+            if !entry.file_type()?.is_file() { continue; }
             let name = entry.file_name();
             let Some(name) = name.to_str() else { continue };
-            // Strictly exclude staging files and arbitrarily named path aliases.
-            if !is_full_hash(name) || handle_for_hash(name) != handle {
-                continue;
-            }
+            if !is_full_hash(name) || handle_for_hash(name) != handle { continue; }
             if match_path.replace((entry.path(), name.to_owned())).is_some() {
                 return Err(invalid("ambiguous output handle"));
             }
         }
-        let (path, hash) = match_path.ok_or_else(|| {
-            io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown")
-        })?;
-        verify_blob(&path, &hash)
+        match_path.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown"))
     }
+
 }
 
 fn digest(data: &[u8]) -> String {
@@ -203,28 +223,55 @@ fn digest(data: &[u8]) -> String {
     hasher.finalize().to_hex().to_string()
 }
 
-fn verify_blob(path: &Path, expected_hash: &str) -> io::Result<Vec<u8>> {
+// `window = None` retains the full verified object. `Some((start,end))`
+// retains only the requested half-open window; Some((0,0)) verifies metadata
+// without materializing any payload. All modes hash the entire opened object.
+fn verify_blob_window(path: &Path, expected_hash: &str, window: Option<(usize, usize)>) -> io::Result<(Vec<u8>, u64)> {
     let mut options = OpenOptions::new();
     options.read(true);
-    // Checking the path then opening it is susceptible to symlink swaps.
-    // O_NOFOLLOW protects the final component on Unix. Validate the *opened*
-    // descriptor, not separately acquired path metadata.
     #[cfg(unix)]
     {
         use std::os::unix::fs::OpenOptionsExt;
         options.custom_flags(libc::O_NOFOLLOW);
     }
-    let file = options.open(path)?;
+    let mut file = options.open(path)?;
     let metadata = file.metadata()?;
     if !metadata.is_file() || metadata.len() > MAX_BLOB_BYTES as u64 {
         return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid stored blob type or size"));
     }
+    if let Some((start, end)) = window {
+        if start > end || end > metadata.len() as usize {
+            return Err(invalid("requested byte window exceeds blob bounds"));
+        }
+    }
+    let mut hasher = Hasher::new();
     let mut bytes = Vec::new();
-    file.take(MAX_BLOB_BYTES as u64 + 1).read_to_end(&mut bytes)?;
-    if bytes.len() > MAX_BLOB_BYTES || digest(&bytes) != expected_hash {
-        return Err(io::Error::new(io::ErrorKind::InvalidData, "stored blob failed size or digest verification"));
+    let mut offset = 0usize;
+    let mut buffer = [0u8; 8192];
+    loop {
+        let length = file.read(&mut buffer)?;
+        if length == 0 { break; }
+        offset = offset.checked_add(length).ok_or_else(|| invalid("blob byte count overflow"))?;
+        if offset > MAX_BLOB_BYTES {
+            return Err(io::Error::new(io::ErrorKind::InvalidData, "stored blob exceeds size limit"));
+        }
+        hasher.update(&buffer[..length]);
+        match window {
+            None => bytes.extend_from_slice(&buffer[..length]),
+            Some((start, end)) => {
+                let chunk_start = offset - length;
+                let first = start.max(chunk_start);
+                let last = end.min(offset);
+                if first < last {
+                    bytes.extend_from_slice(&buffer[first - chunk_start..last - chunk_start]);
+                }
+            }
+        }
+    }
+    if offset as u64 != metadata.len() || hasher.finalize().to_hex().as_str() != expected_hash {
+        return Err(io::Error::new(io::ErrorKind::InvalidData, "stored blob failed length or digest verification"));
     }
-    Ok(bytes)
+    Ok((bytes, offset as u64))
 }
 
 fn is_hex(name: &str) -> bool {
@@ -447,3 +494,35 @@ mod final_component_safety_tests {
         let _ = fs::remove_dir_all(root);
     }
 }
+
+#[cfg(test)]
+mod streaming_inspection_tests {
+    use super::*;
+
+    #[test]
+    fn window_validates_full_blob_and_preserves_exact_slice() {
+        let path = std::env::temp_dir().join(format!(
+            "conduit-window-{}-{}", std::process::id(), NEXT_TEMP.fetch_add(1, Ordering::Relaxed)));
+        let store = BlobStore::open(&path).unwrap();
+        let bytes: Vec<u8> = (0..3 * 1024 * 1024).map(|offset| (offset % 251) as u8).collect();
+        let reference = store.put(&bytes).unwrap();
+        let inspection = store.inspect(&reference.handle).unwrap();
+        assert_eq!(inspection.bytes, bytes.len() as u64);
+        assert_eq!(inspection.hash, reference.hash);
+        assert_eq!(store.get_range(&reference.handle, 1024 * 1024 - 3, 7).unwrap(), bytes[1024 * 1024 - 3..1024 * 1024 + 4].to_vec());
+        assert!(store.get_range(&reference.handle, 0, 0).is_err());
+        assert!(store.get_range(&reference.handle, 0, 1024 * 1024 + 1).is_err());
+        assert!(store.get_range(&reference.handle, usize::MAX, 2).is_err());
+        assert!(store.get_range(&reference.handle, bytes.len(), 1).is_err());
+        let blob_path = path.join(&reference.hash[..2]).join(&reference.hash);
+        let mut corrupt = bytes;
+        let last = corrupt.len() - 1;
+        corrupt[last] ^= 1;
+        fs::write(blob_path, corrupt).unwrap();
+        // The tampered byte is outside the requested range; no partial read
+        // may be returned without complete integrity validation.
+        assert_eq!(store.get_range(&reference.handle, 0, 4).unwrap_err().kind(), io::ErrorKind::InvalidData);
+        assert!(store.inspect(&reference.handle).is_err());
+        let _ = fs::remove_dir_all(path);
+    }
+}
diff --git a/docs/wiki/architecture.md b/docs/wiki/architecture.md
index d409a50a6140293c58b4fbcf27f34d0390c767cc..44d5e79d8aa601e40ad1aea6b50501f1a0d1836b 100644
--- a/docs/wiki/architecture.md
+++ b/docs/wiki/architecture.md
@@ -10,7 +10,7 @@ Conduit currently is a local CLI and independent Rust library crates. The follow
 6. Emit the expansion hint on stderr. If persisting fails, pass stdout through unchanged.
 7. If `CONDUIT_SESSION` is set, record a JSONL word-estimate event (not actual model tokens).
 
-`find` now scans bounded files without retaining a repository-wide snapshot; the library still exposes a snapshot API. `peek` is a bounded hash-anchored read. Neither implements syntax trees or a persistent index. `edit` and `batch-edit` share preflight validation and perform per-file staged renames with cooperative locking, but not crash-atomic transactions across files. `doctor` verifies storage integrity on demand; `repair-session` safely truncates only a torn final JSONL record after validating the preceding records.
+`find` now scans bounded files without retaining a repository-wide snapshot; the library still exposes a snapshot API. `peek` is a bounded hash-anchored read. Neither implements syntax trees or a persistent index. `edit` and `batch-edit` share preflight validation and perform per-file staged renames with cooperative locking, with persisted recovery receipts and manual rollback; they still lack multi-file atomic visibility. `doctor` verifies storage integrity on demand; `repair-session` safely truncates only a torn final JSONL record after validating the preceding records.
 
 The following crates exist but are **not integrated with CLI dispatch**: `ledger`, `relay`, `guard`, `verify`, `vault`, `memory`, `fanout`. The protobuf schema is not generated or consumed by the CLI. No OS sandbox, PTY, provider gateway, daemon or distributed state plane is available.
 
@@ -22,7 +22,7 @@ agent adapters / typed tool requests
     -> supervised execution with bounded raw spool [partially implemented]
     -> verified content store and output adapters [partially implemented]
     -> correlated accounting and deterministic retrieval [partially implemented]
-    -> version-checked edit and recovery journal [not implemented]
+    -> hash-fenced edit receipt and explicit rollback [implemented, not yet runtime-verified]
     -> evaluation/promotion gates and optional memory [not implemented]
 ```
 
diff --git a/docs/wiki/batch-edit.md b/docs/wiki/batch-edit.md
index a5ac9dc38419551c78b4e1aa8632164835c457db..38f0788c93d2355ea355f3acf7f513d9772b64a0 100644
--- a/docs/wiki/batch-edit.md
+++ b/docs/wiki/batch-edit.md
@@ -52,11 +52,54 @@ conduit batch-edit /tmp/reviewed-change.json
   the rename implementation would otherwise silently separate aliases.
 - Staging uses same-directory files and a process-lifetime cooperative OS lock;
   per-file source hashes are rechecked before replacement.
-- Multi-file all-or-nothing power-loss/crash recovery is **not** implemented.
-  Even successful dry-run checks do not change that limitation.
+- Before the first rename, Conduit writes and syncs a versioned receipt and
+  private original-byte backups in `.conduit/edit-recovery/txn-*/`. A crashed
+  batch can be inspected and explicitly rolled back if source/backup hashes
+  still match. Other edits fail closed while a receipt is pending.
+- Multi-file **atomic visibility** is not implemented. Readers can observe
+  partially updated source trees. Filesystem race safety and power-loss
+  durability have not been certified on all supported operating systems.
+- Unix permission bits and readonly status are recorded for recovery. Ownership,
+  extended attributes, Windows ACLs and other metadata are not fully preserved
+  by the per-file rename algorithm; exercise care with protected source trees.
 - Concurrent uncooperative external writers, symbolic-link races and filesystem
   semantics remain potential threats. This tool is not a filesystem sandbox.
 
-Do not use the batch API for untrusted source writes or multi-file changes
-requiring transactional crash recovery. For such operations use a versioned
-source-control workflow and a supervisory transaction/recovery mechanism.
+## Operator recovery after an interrupted batch
+
+```sh
+conduit recover-edits --list
+conduit recover-edits --json
+# For a partially applied batch, explicitly restore originals:
+conduit recover-edits --rollback txn-REPLACE_WITH_ACTUAL_ID
+# OR, for a fully applied batch whose cleanup was interrupted:
+conduit recover-edits --finalize txn-REPLACE_WITH_ACTUAL_ID
+# After preserving any incident evidence and resolving every receipt:
+conduit recover-edits --prune-orphans
+conduit doctor
+```
+
+A pending receipt blocks both new edits and dry runs until resolved. `doctor`
+exits nonzero while a transaction is pending. Rollback checks **all** targets
+and the backups it needs before changing a file, and refuses unexpected source
+hashes. If a rollback itself is interrupted, the same receipt can be retried;
+it does not authorize overwriting a newer or unrelated edit. A receipt from a
+fully applied batch may still be present if the process ended before cleanup.
+In that case, `--finalize` verifies every updated target and retained backup
+before clearing the receipt. It does not apply missing edits or change source
+files. `--rollback` instead explicitly reverts the full batch. Both actions
+require cooperative workspace locking and do not prevent external filesystem
+writers from racing the operation. Inspect the working tree first.
+
+Keep `.conduit/edit-recovery` private and include it in incident evidence as
+appropriate. Backups may contain confidential source text. A crash *before*
+receipt publication can leave an orphan backup directory; it never authorizes
+new writes, but its contents still require controlled cleanup. `--prune-orphans`
+validates recognized backup filenames and regular-file size bounds before
+removing receipt-less directories. It does **not** find or erase every possible
+staging file outside the transaction directory. Never delete a
+pending receipt or backup merely to bypass a recovery block.
+
+For applications requiring atomic visibility across files or adversarial
+filesystem isolation, use a transactional worktree or a platform-specific
+supervisor. This editor is not an OS sandbox or an ACID database.
diff --git a/docs/wiki/cli-reference.md b/docs/wiki/cli-reference.md
index b8907f6a55d8cde0169c6e2432d52228dede93eb..c0b14a5ad1ce3e093b06e8c7957e4cf5df6875c3 100644
--- a/docs/wiki/cli-reference.md
+++ b/docs/wiki/cli-reference.md
@@ -18,19 +18,49 @@ Describes implemented and absent capabilities, including an explicit `os_sandbox
 
 ## `edit <relative-path> <expected-hash> <old> <new>` and `batch-edit [--check] <manifest.json>`
 
-Hash-anchored source edits. `batch-edit` accepts `search_replace` and `replace_range` operations with per-target BLAKE3 preconditions; `--check` validates all operations without changing source content. A plan is not reserved between the preflight and commit. The shared validator blocks path escapes, protected metadata, stale hashes, duplicate canonical targets, hardlinked files, oversized inputs/outputs, and ambiguous matches. Limits: 1–128 operations, 8 MiB per target, 64 MiB combined input/output, and a 1 MiB manifest. Uses a process-lifetime cooperative lock under the workspace `.conduit` directory and staged file replacement. Multi-file crash atomicity and safety from uncooperative writers are not provided. See [batch edit](batch-edit.md).
-
-## `store <text>|-` and `expand <handle>`
-
-Persists bytes in a local BLAKE3-addressed store or recovers them by handle. `store -` reads up to 64 MiB from stdin, without exposing content in command-line arguments. Retrieval verifies the full digest. Handles are not access-control credentials. There is no automatic retention or deletion.
+Hash-anchored source edits. `batch-edit` accepts `search_replace` and `replace_range` operations with per-target BLAKE3 preconditions; `--check` validates all operations without changing source content. A plan is not reserved between the preflight and commit. The shared validator blocks path escapes, protected metadata, stale hashes, duplicate canonical targets, hardlinked files, oversized inputs/outputs, and ambiguous matches. Limits: 1–128 operations, 8 MiB per target, 64 MiB combined input/output, and a 1 MiB manifest. Uses a process-lifetime cooperative lock under the workspace `.conduit` directory and staged file replacement. A private, versioned receipt with source backups is synced before the first rename. Incomplete batches block new edits and can be manually rolled back with hash fencing. Multi-file atomic visibility and safety from uncooperative writers are still not provided. See [batch edit](batch-edit.md).
+
+## `recover-edits [--list|--json|--rollback ID|--finalize ID|--prune-orphans]`
+
+Lists incomplete batch-edit recovery receipts or explicitly reverts an
+interrupted batch to its original file bytes. Each target must still have its
+recorded original or updated hash; missing/different targets block rollback.
+Backups required for restoration must pass their hash checks. Conduit does not
+auto-resume or silently delete conflict evidence. Transaction IDs are restricted
+to the generated `txn-*` format. A batch that reached **all** of its
+recorded updated hashes before a crash can instead be completed with
+`recover-edits --finalize ID`: this verifies every updated source hash and the
+original backups, then deletes the recovery receipt without modifying source
+files. Any missing/changed file or backup blocks finalization. This is not a
+mechanism for applying the remaining steps of a partially committed edit.
+
+`recover-edits --prune-orphans` is a separate **explicitly destructive**
+cleanup for receipt-less transaction directories. It refuses to run while any
+receipt remains and rejects unknown contents; use it only after preserving
+incident evidence. It does not remove untracked edit staging files elsewhere
+in the workspace.
+
+## `store <text>|-`, `inspect [--json] <handle>`, `expand <handle> [--offset N --length N]`
+
+`store` persists bytes in a local BLAKE3-addressed store. `store -` reads up
+to 64 MiB from stdin without exposing content in command-line arguments.
+`inspect` validates the entire object and returns its length and BLAKE3 digest,
+not its contents. Inspection streams verification using fixed-size buffers.
+`expand` verifies the full digest before emitting exact raw bytes. Providing both `--offset` and `--length` yields an exact byte window of
+1–1,048,576 bytes; out-of-bounds windows are rejected, not silently shortened.
+Byte-window expansion now reads the full blob for integrity validation but
+retains only the selected window in memory rather than materializing the
+complete blob.
+With no window options, `expand` emits the full original blob. Handles are
+identifiers, not authorization tokens. No automatic retention is configured.
 
 ## `doctor [--json]`
 
-Read-only integrity scan of the local blob store. Reports verified objects/bytes, observed staging files, and detected errors (up to 32 examples), failing with a nonzero exit code for integrity problems. The scan does not delete blobs or promise an atomic snapshot while other processes write.
+Read-only integrity scan of the local blob store. Reports verified objects/bytes, observed staging files, and detected errors (up to 32 examples), failing with a nonzero exit code for integrity problems or pending edit receipts. The scan does not delete blobs or promise an atomic snapshot while other processes write.
 
 ## `record`, `replay`, `stats [--json]`, `repair-session`
 
-Writes and reads synchronized JSONL session events. Session identifiers are validated. Records are bounded to 8 KiB; each session log is limited to 64 MiB. `stats` calculates totals from a streaming, validated traversal. An incomplete last record fails closed. `repair-session` explicitly removes only an unterminated final record, after validating the preceding records. It does not reconstruct missing events or repair malformed complete lines. Accounting is a caller-provided or whitespace-word estimate, **not tokenizer-derived usage**.
+Writes and reads synchronized JSONL session events. Session identifiers are validated. Records are bounded to 8 KiB; each session log is limited to 64 MiB. `stats` calculates totals from a streaming, validated traversal. An incomplete last record fails closed. `repair-session` first checks whether an unterminated final record is otherwise valid JSON: if so, it completes the newline and preserves its accounting. If the fragment is malformed, the raw tail is written to a private, synced `.torn-*.bak` file before the log is truncated. The command reports `action=completed_record`, `action=quarantined backup=...`, or `action=unchanged`. It never silently changes a malformed *complete* line or reconstructs missing events. Tail backups may be confidential and require explicit retention handling. Accounting is a caller-provided or whitespace-word estimate, **not tokenizer-derived usage**.
 
 ## `bench [file] [budget]`
 
@@ -43,7 +73,7 @@ Only inspection and diagnostic commands support `--json`. Never parse ordinary h
 ## Environment
 
 - `CONDUIT_HOME`: storage and sessions root; default `.conduit`.
-- `CONDUIT_WORKSPACE`: root for `find`, `peek`, `edit`, and `batch-edit`; default current directory.
+- `CONDUIT_WORKSPACE`: root for `find`, `peek`, `edit`, `batch-edit`, and `recover-edits`; default current directory. Edit recovery files live in `<workspace>/.conduit/edit-recovery`, independently of `CONDUIT_HOME`.
 - `CONDUIT_SESSION`: optional session name for accounting of successful compacted observations.
 - `CONDUIT_DEBUG_TIMING`: when set, emits child execution wall time.
 
diff --git a/docs/wiki/operations.md b/docs/wiki/operations.md
index ae4ff66c1c282f087bba44911ab22e1bc6425040..8287b1c2ec9e2e80442478c82cd1ebd981c4abcd 100644
--- a/docs/wiki/operations.md
+++ b/docs/wiki/operations.md
@@ -6,13 +6,13 @@ Blob storage is content-addressed and deduplicated. Session logs are append-only
 
 ## Diagnostics
 
-Run `conduit capabilities --json` to establish what Conduit **does not** enforce before granting an agent command access. Run `conduit doctor --json` to verify stored blob contents and identify unexpected shard entries. `doctor` reads object contents, so restrict access to authorized operators. It is read-only and neither deletes staging files nor races-freezes concurrently published objects. Run `conduit stats SESSION` to validate session accounting without building an in-memory list of all events. `repair-session SESSION` is explicitly destructive to a torn final record only; preserve a backup of a damaged log if forensic reconstruction matters.
+Run `conduit capabilities --json` to establish what Conduit **does not** enforce before granting an agent command access. Run `conduit doctor --json` to verify stored blob contents, identify unexpected shard entries, and detect unfinished edit transactions. An unfinished transaction causes a nonzero health-check exit status. `doctor` reads object contents, so restrict access to authorized operators. It is read-only and neither deletes staging files nor races-freezes concurrently published objects. Run `conduit stats SESSION` to validate session accounting without building an in-memory list of all events. `repair-session SESSION` preserves a complete JSON event missing only its newline; otherwise it saves the malformed trailing bytes to a private, synced `.torn-*.bak` before truncation. Retain or securely dispose of this evidence according to the workspace's data policy.
 
-When behavior is unexpected, collect the command, profile, exit code, presented output, and expansion handle—not secrets or the full raw blob by default. The handle is in reduced stdout and should be retained as part of the tool's presented result. Confirm the workspace and `CONDUIT_HOME` values first. Re-run with the raw handle only when necessary.
+When behavior is unexpected, collect the command, profile, exit code, presented output, and expansion handle—not secrets or the full raw blob by default. The handle is in reduced stdout and should be retained as part of the tool's presented result. Confirm the workspace and `CONDUIT_HOME` values first. Use `conduit inspect --json HANDLE` to verify and size a stored output before opening it. `conduit expand HANDLE --offset N --length N` returns a verified, bounded byte window without adding metadata to the raw stdout; use whole-blob expansion only when necessary.
 
 ## Recovery
 
-A missing or expired handle requires rerunning the command. A stale edit must be regenerated from a fresh atlas hit. The edit lock is held by the OS for the process lifetime and released automatically on process exit; do not delete the file while edits are active because that could create separate lock inodes. For multi-file batches, an interrupted commit may leave a partially applied set: `batch-edit --check` does not constitute crash recovery. Preserve original files/backups and reconcile hashes before retrying. A torn session tail requires explicit `repair-session` after evidence preservation.
+A missing or expired handle requires rerunning the command. A stale edit must be regenerated from a fresh atlas hit. The edit lock is held by the OS for the process lifetime and released automatically on process exit; do not delete the file while edits are active because that could create separate lock inodes. For multi-file batches, `conduit recover-edits --list` identifies receipts and classifies original/updated/conflicting targets. A receipt and backed-up original bytes are made durable before the first rename, but intermediate states remain observable. Use `conduit recover-edits --rollback ID` only after reviewing the state of the source tree: it refuses conflicting third-party hashes and requires intact backups. If **all** sources have already reached their updated digests, `conduit recover-edits --finalize ID` instead checks all targets and backups before removing the receipt without changing source files. It cannot complete partially committed edits. Do not delete pending journals to force an edit through. No automatic roll-forward or certified multi-file atomic transaction is provided. A pre-manifest crash or interrupted post-commit cleanup can leave orphan backup debris under `.conduit/edit-recovery`; `doctor` reports the count. After preserving needed incident evidence, `conduit recover-edits --prune-orphans` explicitly removes *only* receipt-less directories containing recognized bounded backup files. It refuses to operate while a pending transaction exists and leaves unrelated stage files untouched. `batch-edit --check` is not a crash simulation. A torn session tail requires explicit `repair-session` after evidence preservation.
 
 ## Release checklist
 
diff --git a/docs/wiki/structured-output.md b/docs/wiki/structured-output.md
index 5e989106a053d47cfbac009f8f0d0711bca528c3..c32f9aab43bf9c183b6803adae9cb29069f93d0a 100644
--- a/docs/wiki/structured-output.md
+++ b/docs/wiki/structured-output.md
@@ -1,13 +1,16 @@
 # Structured CLI results (schema version 1)
 
-Machine-readable diagnostic outputs are opt-in. `--json` may occur before or after positional arguments on `find`, `peek`, `doctor`, `stats`, and `capabilities`. Each response is one JSON object on stdout with `schema_version: 1` and a `kind` discriminator. Errors are reported on stderr with nonzero process status; do not parse stderr for state transitions.
+Machine-readable diagnostic outputs are opt-in. `--json` may occur before or after positional arguments on `find`, `peek`, `doctor`, `stats`, `capabilities`, and `inspect`.
+`recover-edits --json` also returns one versioned JSON object. Each response is one JSON object on stdout with `schema_version: 1` and a `kind` discriminator. Errors are reported on stderr with nonzero process status; do not parse stderr for state transitions.
 
 | Command | Fields of interest | Semantics |
 | --- | --- | --- |
 | `capabilities --json` | `os_sandbox`, `multi_file_crash_atomic`, `ast_search`, `process_group_cleanup_best_effort` | Feature declarations, **not** a host isolation attestation |
 | `find --json QUERY [MAX_HITS]` | `hits[]`, `scanned_files`, `scanned_bytes`, `skipped`, `truncated`, `limit_reached`, `complete` | Bounded, sorted lexical search; `complete` means scan was not interrupted by a cap or read/UTF-8/binary skip, **not** that gitignored files were inspected |
 | `peek --json PATH [START] [COUNT]` | `blake3`, `total_lines`, `clipped_lines`, `truncated`, `lines[]` | Full source-file hash is computed before display; individual long lines are clipped at 512 characters |
-| `doctor --json` | `verified_blobs`, `staged_blobs`, `integrity_problems`, `problem_examples`, `consistent_snapshot` | Integrity scan is read-only and is not atomic relative to concurrent writes |
+| `doctor --json` | `verified_blobs`, `staged_blobs`, `integrity_problems`, `pending_edit_transactions`, `conflicting_edit_targets`, `orphaned_recovery_directories`, `consistent_snapshot` | Integrity scan is not atomic relative to concurrent writes; pending edit receipts cause a nonzero exit code |
+| `inspect --json HANDLE` | `handle`, `blake3`, `bytes`, `verified` | Verifies entire object before returning metadata only |
+| `recover-edits --json` | `transactions[]` (`id`, `files`, `original`, `updated`, `conflicting`), `orphaned_directories`, `multi_file_atomic=false` | Recovery queue, not a transactional-commit attestation |
 | `stats --json SESSION` | `raw_units`, `presented_units`, `net_saved_units`, `measured`, `units` | Caller-provided and word estimates, **not** tokenizer-verified usage |
 
 For `find` and `peek`, `path_utf8=false` means `path` was rendered using lossy encoding; do not use it as a machine-authoritative filesystem identity. Use a byte-capable or platform-specific adapter instead. BLAKE3 hashes identify bytes, not authorization or source reliability.
@@ -17,3 +20,17 @@ For `find` and `peek`, `path_utf8=false` means `path` was rendered using lossy e
 `conduit run --preset compact -- ...` is **not** a JSON command. It deliberately emits shortened stdout plus a visible `[conduit raw_stdout=... expand='conduit expand ...']` marker when saving enough words to offset the marker. Copy the full presented stdout to preserve the expansion handle. The expanded blob recovers original child **stdout only**; stderr is separately forwarded without compacting. If output exceeds the configured cap, no lossless recovery for the missing bytes is claimed.
 
 Neither structured output nor the sample capability manifest implies an OS sandbox, durable multi-file transaction, task-level correctness proof, or model-token cost metering.
+
+## V5 extensions (opt-in CLI commands)
+
+- `capabilities --json` additionally advertises `edit_recovery_finalize=true`
+  and `explicit_orphan_pruning=true`. These report command availability, not
+  ACID multi-file atomicity or hostile-filesystem protection.
+- `recover-edits --finalize ID` emits `finalized=ID` only after its hash-gated
+  check and receipt cleanup complete.
+- `recover-edits --prune-orphans` emits `pruned_orphaned_directories=N` after
+  all candidate directory contents pass its preflight checks.
+- `repair-session SESSION` emits `action=unchanged`,
+  `action=completed_record`, or `action=quarantined backup=PATH` with the
+  legacy `partial_tail_removed` indicator. Quarantine backups are never
+  included in structured telemetry by default.
diff --git a/src/main.rs b/src/main.rs
index 30bc76faa1151a34708df31c412efe42f409a11f..08f80927a303a4b89f27250d67e06afdd193f961 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,9 +1,9 @@
 mod run;
 
 use conduit_atlas::{find_streaming, peek};
-use conduit_edit::{apply, preview, Operation};
+use conduit_edit::{apply, finalize_edit, orphaned_recovery_dirs, pending_edits, preview, prune_orphaned_edits, rollback_edit, Operation};
 use conduit_lens::distill;
-use conduit_scope::{Event, EventLog, Savings};
+use conduit_scope::{Event, EventLog, Savings, TailRepair};
 use conduit_store::BlobStore;
 use run::{Capture, RunOptions, RunResult};
 use std::{
@@ -15,7 +15,7 @@ use std::{
 };
 
 fn usage() -> ! {
-    eprintln!("usage: conduit run [--preset safe|quick|deep|compact] [--timeout-ms N] [--max-output-bytes N] [--budget N] [--compact] -- PROGRAM [ARGS...]\n       conduit <capabilities|find|peek|edit|batch-edit|store|expand|doctor|record|replay|stats|repair-session|bench> ...\n       conduit find [--json] [--require-complete] QUERY [MAX_HITS]\n       conduit peek [--json] PATH [START] [COUNT]\n       conduit doctor [--json]\n       conduit stats [--json] SESSION");
+    eprintln!("usage: conduit run [--preset safe|quick|deep|compact] [--timeout-ms N] [--max-output-bytes N] [--budget N] [--compact] -- PROGRAM [ARGS...]\n       conduit <capabilities|find|peek|edit|batch-edit|recover-edits|store|inspect|expand|doctor|record|replay|stats|repair-session|bench> ...\n       conduit find [--json] [--require-complete] QUERY [MAX_HITS]\n       conduit peek [--json] PATH [START] [COUNT]\n       conduit doctor [--json]\n       conduit recover-edits [--list|--json|--rollback ID|--finalize ID|--prune-orphans]\n       conduit inspect [--json] HANDLE\n       conduit expand HANDLE [--offset N --length N]\n       conduit stats [--json] SESSION");
     process::exit(2);
 }
 
@@ -213,7 +213,9 @@ fn main() {
                 "command_execution": true, "direct_child_timeout": true,
                 "output_retention_bounded": true, "raw_recovery_only_if_compacted": true,
                 "lexical_search": true, "ast_search": false, "semantic_index": false,
-                "batch_preflight": true, "multi_file_crash_atomic": false,
+                "batch_preflight": true, "edit_recovery_receipts": true,
+                "edit_recovery_finalize": true, "explicit_orphan_pruning": true,
+                "multi_file_crash_atomic": false,
                 "os_sandbox": false, "process_group_cleanup_best_effort": cfg!(unix),
                 "windows_process_tree_containment": false, "tokenizer_metering": false,
             });
@@ -305,6 +307,57 @@ fn main() {
                 println!("{}={} hash={}", if check { "validated" } else { "applied" }, item.path.display(), item.hash);
             }
         }
+        Some("recover-edits") => {
+            let option = args.next();
+            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from)
+                .unwrap_or_else(|| PathBuf::from("."));
+            match option.as_deref() {
+                None | Some("--list") | Some("--json") => {
+                    no_extra_args(&mut args);
+                    let pending = pending_edits(&workspace).unwrap_or_else(|error| fail(error));
+                    let orphans = orphaned_recovery_dirs(&workspace).unwrap_or_else(|error| fail(error));
+                    if option.as_deref() == Some("--json") {
+                        let entries: Vec<_> = pending.iter().map(|transaction| serde_json::json!({
+                            "id": transaction.id, "files": transaction.files,
+                            "original": transaction.original, "updated": transaction.updated,
+                            "conflicting": transaction.conflicting,
+                        })).collect();
+                        print_json(serde_json::json!({
+                            "schema_version": 1, "kind": "edit_recovery",
+                            "transactions": entries, "orphaned_directories": orphans,
+                            "multi_file_atomic": false,
+                        }));
+                    } else if pending.is_empty() {
+                        println!("pending_edits=0 orphaned_directories={orphans}");
+                    } else {
+                        for transaction in pending {
+                            println!("id={} files={} original={} updated={} conflicting={}",
+                                transaction.id, transaction.files, transaction.original,
+                                transaction.updated, transaction.conflicting);
+                        }
+                        println!("orphaned_directories={orphans}");
+                    }
+                }
+                Some("--rollback") => {
+                    let id = args.next().unwrap_or_else(|| usage());
+                    no_extra_args(&mut args);
+                    rollback_edit(workspace, &id).unwrap_or_else(|error| fail(error));
+                    println!("rolled_back={id}");
+                }
+                Some("--finalize") => {
+                    let id = args.next().unwrap_or_else(|| usage());
+                    no_extra_args(&mut args);
+                    finalize_edit(workspace, &id).unwrap_or_else(|error| fail(error));
+                    println!("finalized={id}");
+                }
+                Some("--prune-orphans") => {
+                    no_extra_args(&mut args);
+                    let removed = prune_orphaned_edits(workspace).unwrap_or_else(|error| fail(error));
+                    println!("pruned_orphaned_directories={removed}");
+                }
+                _ => usage(),
+            }
+        }
         Some("store") => {
             let argument = args.next().unwrap_or_else(|| usage());
             no_extra_args(&mut args);
@@ -320,31 +373,72 @@ fn main() {
             let blob = store.put(&bytes).unwrap_or_else(|error| fail(error));
             println!("{}", blob.handle);
         }
+        Some("inspect") => {
+            let (json, _, positional) = diagnostic_args(args, false);
+            if positional.len() != 1 { usage(); }
+            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
+            let blob = store.inspect(&positional[0]).unwrap_or_else(|error| fail(error));
+            if json {
+                print_json(serde_json::json!({
+                    "schema_version": 1, "kind": "blob", "handle": blob.handle,
+                    "blake3": blob.hash, "bytes": blob.bytes, "verified": true,
+                }));
+            } else {
+                println!("handle={} blake3={} bytes={} verified=true", blob.handle, blob.hash, blob.bytes);
+            }
+        }
         Some("expand") => {
             let handle = args.next().unwrap_or_else(|| usage());
-            no_extra_args(&mut args);
+            let (mut offset, mut length) = (None, None);
+            while let Some(flag) = args.next() {
+                match flag.as_str() {
+                    "--offset" if offset.is_none() => {
+                        offset = Some(args.next().and_then(|n| n.parse::<usize>().ok()).unwrap_or_else(|| usage()));
+                    }
+                    "--length" if length.is_none() => {
+                        length = Some(args.next().and_then(|n| n.parse::<usize>().ok()).unwrap_or_else(|| usage()));
+                    }
+                    _ => usage(),
+                }
+            }
+            if offset.is_some() != length.is_some() { usage(); }
             let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
-            let bytes = store.get(&handle).unwrap_or_else(|error| fail(error));
-            io::stdout().write_all(&bytes).unwrap_or_else(|error| fail(error));
+            // Range inspection still verifies the full digest, but retains
+            // only the selected bytes instead of materializing up to 64 MiB.
+            let output = match (offset, length) {
+                (Some(start), Some(count)) => store.get_range(&handle, start, count),
+                (None, None) => store.get(&handle),
+                _ => unreachable!("paired argument validation precedes this match"),
+            }.unwrap_or_else(|error| fail(error));
+            io::stdout().write_all(&output).unwrap_or_else(|error| fail(error));
         }
         Some("doctor") => {
             let (json, _, positional) = diagnostic_args(args, false);
             if !positional.is_empty() { usage(); }
             let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
             let report = store.audit().unwrap_or_else(|error| fail(error));
+            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from)
+                .unwrap_or_else(|| PathBuf::from("."));
+            let pending = pending_edits(&workspace).unwrap_or_else(|error| fail(error));
+            let orphans = orphaned_recovery_dirs(&workspace).unwrap_or_else(|error| fail(error));
+            let edit_conflicts: usize = pending.iter().map(|entry| entry.conflicting).sum();
             if json {
                 print_json(serde_json::json!({
                     "schema_version": 1, "kind": "doctor", "verified_blobs": report.checked_blobs,
                     "verified_bytes": report.verified_bytes, "staged_blobs": report.staging_files,
                     "integrity_problems": report.problem_count, "problem_examples": report.problems,
                     "consistent_snapshot": false,
+                    "pending_edit_transactions": pending.len(),
+                    "conflicting_edit_targets": edit_conflicts,
+                    "orphaned_recovery_directories": orphans,
                 }));
             } else {
-                println!("verified_blobs={} verified_bytes={} staged_blobs={} integrity_problems={}",
-                    report.checked_blobs, report.verified_bytes, report.staging_files, report.problem_count);
+                println!("verified_blobs={} verified_bytes={} staged_blobs={} integrity_problems={} pending_edit_transactions={} conflicting_edit_targets={} orphaned_recovery_directories={}",
+                    report.checked_blobs, report.verified_bytes, report.staging_files,
+                    report.problem_count, pending.len(), edit_conflicts, orphans);
                 for problem in &report.problems { eprintln!("conduit: {problem}"); }
             }
-            if report.problem_count > 0 { process::exit(1); }
+            if report.problem_count > 0 || !pending.is_empty() || orphans > 0 { process::exit(1); }
         }
         Some("record") => {
             let session = args.next().unwrap_or_else(|| usage());
@@ -365,7 +459,12 @@ fn main() {
             let session = args.next().unwrap_or_else(|| usage());
             no_extra_args(&mut args);
             let log = session_log(&session).unwrap_or_else(|error| fail(error));
-            println!("partial_tail_removed={}", log.repair_partial_tail().unwrap_or_else(|error| fail(error)));
+            let outcome = log.repair_tail().unwrap_or_else(|error| fail(error));
+            match outcome {
+                TailRepair::Unchanged => println!("partial_tail_removed=false action=unchanged"),
+                TailRepair::CompletedRecord => println!("partial_tail_removed=false action=completed_record"),
+                TailRepair::Quarantined { backup } => println!("partial_tail_removed=true action=quarantined backup={}", backup.display()),
+            }
         }
         Some("stats") => {
             let (json, _, positional) = diagnostic_args(args, false);
diff --git a/tests/cli.rs b/tests/cli.rs
index c6af71f46d14997d57b022cf0509708278ed0ff2..7bcd91d58d1fd77e8222f3205131dc7e2b7962b7 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -179,3 +179,159 @@ fn strict_search_reports_incompleteness_even_with_structured_results() {
     assert_eq!(info["complete"], false);
     assert_eq!(info["limit_reached"], true);
 }
+
+
+#[test]
+fn recovery_cli_reports_no_outstanding_transactions_after_successful_commit() {
+    let root = Workspace::new();
+    fs::write(root.path().join("file.rs"), "before").unwrap();
+    let hash = hash_from_peek(&root.command(&["peek", "file.rs"]));
+    let result = root.command(&["edit", "file.rs", &hash, "before", "after"]);
+    assert!(result.status.success(), "{}", utf8(&result.stderr));
+    let pending = root.command(&["recover-edits", "--json"]);
+    assert!(pending.status.success(), "{}", utf8(&pending.stderr));
+    let parsed: serde_json::Value = serde_json::from_slice(&pending.stdout).unwrap();
+    assert_eq!(parsed["kind"], "edit_recovery");
+    assert_eq!(parsed["multi_file_atomic"], false);
+    assert_eq!(parsed["transactions"].as_array().unwrap().len(), 0);
+    assert!(!root.command(&["recover-edits", "--rollback", "../../escape"]).status.success());
+}
+
+
+#[test]
+fn inspect_and_bounded_expansion_preserve_exact_bytes() {
+    let root = Workspace::new();
+    let stored = root.command(&["store", "abcdef0123456789"]);
+    assert!(stored.status.success());
+    let handle = utf8(&stored.stdout).trim();
+    let inspection = root.command(&["inspect", handle, "--json"]);
+    assert!(inspection.status.success(), "{}", utf8(&inspection.stderr));
+    let metadata: serde_json::Value = serde_json::from_slice(&inspection.stdout).unwrap();
+    assert_eq!(metadata["kind"], "blob");
+    assert_eq!(metadata["bytes"], 16);
+    assert_eq!(metadata["blake3"].as_str().unwrap().len(), 64);
+    let window = root.command(&["expand", handle, "--length", "6", "--offset", "6"]);
+    assert!(window.status.success(), "{}", utf8(&window.stderr));
+    assert_eq!(window.stdout, b"012345");
+    assert!(!root.command(&["expand", handle, "--offset", "999", "--length", "6"]).status.success());
+    assert!(!root.command(&["expand", handle, "--offset", "2"]).status.success());
+}
+
+
+#[cfg(feature = "test-fault-injection")]
+#[test]
+fn interrupted_process_persists_a_recoverable_edit_receipt() {
+    let root = Workspace::new();
+    fs::write(root.path().join("a.rs"), "before a").unwrap();
+    fs::write(root.path().join("b.rs"), "before b").unwrap();
+    let first = hash_from_peek(&root.command(&["peek", "a.rs"]));
+    let second = hash_from_peek(&root.command(&["peek", "b.rs"]));
+    let manifest = serde_json::json!({"operations": [
+        {"type": "search_replace", "path": "a.rs", "expected_hash": first, "old": "before", "new": "after"},
+        {"type": "search_replace", "path": "b.rs", "expected_hash": second, "old": "before", "new": "after"},
+    ]});
+    let manifest_path = root.path().join("plan.json");
+    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
+    let aborted = Command::new(env!("CARGO_BIN_EXE_conduit"))
+        .args(["batch-edit", manifest_path.to_str().unwrap()])
+        .env("CONDUIT_WORKSPACE", root.path())
+        .env("CONDUIT_HOME", root.path().join(".conduit"))
+        .env("CONDUIT_TEST_EXIT_AFTER_FIRST_EDIT", "1")
+        .output().unwrap();
+    assert_eq!(aborted.status.code(), Some(97));
+    assert_eq!(fs::read_to_string(root.path().join("a.rs")).unwrap(), "after a");
+    assert_eq!(fs::read_to_string(root.path().join("b.rs")).unwrap(), "before b");
+    let pending = root.command(&["recover-edits", "--json"]);
+    let info: serde_json::Value = serde_json::from_slice(&pending.stdout).unwrap();
+    let id = info["transactions"][0]["id"].as_str().unwrap();
+    assert_eq!(info["transactions"][0]["updated"], 1);
+    assert_eq!(info["transactions"][0]["original"], 1);
+    assert!(!root.command(&["batch-edit", "--check", manifest_path.to_str().unwrap()]).status.success());
+    assert!(!root.command(&["doctor"]).status.success());
+    let recovered = root.command(&["recover-edits", "--rollback", id]);
+    assert!(recovered.status.success(), "{}", utf8(&recovered.stderr));
+    assert_eq!(fs::read_to_string(root.path().join("a.rs")).unwrap(), "before a");
+    assert_eq!(fs::read_to_string(root.path().join("b.rs")).unwrap(), "before b");
+    assert!(!fs::read_dir(root.path()).unwrap().any(|entry| {
+        entry.unwrap().file_name().to_string_lossy().starts_with(".conduit-edit-")
+    }), "recovery must delete its own retained staging objects");
+    assert!(root.command(&["doctor"]).status.success());
+}
+
+#[test]
+fn orphan_cleanup_refuses_unknown_objects_and_clears_reported_debris() {
+    let root = Workspace::new();
+    let recovery = root.path().join(".conduit/edit-recovery/txn-20-100-0");
+    fs::create_dir_all(&recovery).unwrap();
+    fs::write(recovery.join("0.backup"), "old bytes").unwrap();
+    let before = root.command(&["doctor", "--json"]);
+    assert!(!before.status.success());
+    let before_json: serde_json::Value = serde_json::from_slice(&before.stdout).unwrap();
+    assert_eq!(before_json["orphaned_recovery_directories"], 1);
+    fs::write(recovery.join("README.md"), "not a recovery backup").unwrap();
+    let blocked = root.command(&["recover-edits", "--prune-orphans"]);
+    assert!(!blocked.status.success());
+    assert!(recovery.join("0.backup").exists());
+    fs::remove_file(recovery.join("README.md")).unwrap();
+    let pruned = root.command(&["recover-edits", "--prune-orphans"]);
+    assert!(pruned.status.success(), "{}", utf8(&pruned.stderr));
+    assert!(utf8(&pruned.stdout).contains("pruned_orphaned_directories=1"));
+    assert!(!recovery.exists());
+    assert!(root.command(&["doctor"]).status.success());
+}
+
+#[test]
+fn repair_session_completes_valid_json_without_losing_an_event() {
+    let root = Workspace::new();
+    assert!(root.command(&["record", "validtail", "one", "1", "1"]).status.success());
+    let path = root.path().join(".conduit/sessions/validtail.jsonl");
+    let event = serde_json::json!({
+        "type": "savings", "module": "kept", "raw_tokens": 7,
+        "presented_tokens": 2, "measured": false,
+    });
+    use std::io::Write;
+    fs::OpenOptions::new().append(true).open(&path).unwrap()
+        .write_all(serde_json::to_string(&event).unwrap().as_bytes()).unwrap();
+    assert!(!root.command(&["stats", "validtail"]).status.success());
+    let fix = root.command(&["repair-session", "validtail"]);
+    assert!(fix.status.success(), "{}", utf8(&fix.stderr));
+    assert!(utf8(&fix.stdout).contains("action=completed_record"));
+    let stats = root.command(&["stats", "validtail"]);
+    assert!(stats.status.success());
+    assert!(utf8(&stats.stdout).contains("raw_units=8 presented_units=3"));
+}
+
+#[cfg(feature = "test-fault-injection")]
+#[test]
+fn committed_crash_can_be_finalized_without_reverting_source() {
+    let root = Workspace::new();
+    fs::write(root.path().join("a.rs"), "before a").unwrap();
+    fs::write(root.path().join("b.rs"), "before b").unwrap();
+    let first = hash_from_peek(&root.command(&["peek", "a.rs"]));
+    let second = hash_from_peek(&root.command(&["peek", "b.rs"]));
+    let manifest = serde_json::json!({"operations": [
+        {"type": "search_replace", "path": "a.rs", "expected_hash": first, "old": "before", "new": "after"},
+        {"type": "search_replace", "path": "b.rs", "expected_hash": second, "old": "before", "new": "after"}
+    ]});
+    let manifest_path = root.path().join("plan.json");
+    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
+    let crashed = Command::new(env!("CARGO_BIN_EXE_conduit"))
+        .args(["batch-edit", manifest_path.to_str().unwrap()])
+        .env("CONDUIT_WORKSPACE", root.path())
+        .env("CONDUIT_HOME", root.path().join(".conduit"))
+        .env("CONDUIT_TEST_EXIT_AFTER_ALL_EDITS", "1")
+        .output().unwrap();
+    assert_eq!(crashed.status.code(), Some(98));
+    assert_eq!(fs::read_to_string(root.path().join("a.rs")).unwrap(), "after a");
+    assert_eq!(fs::read_to_string(root.path().join("b.rs")).unwrap(), "after b");
+    let pending = root.command(&["recover-edits", "--json"]);
+    let info: serde_json::Value = serde_json::from_slice(&pending.stdout).unwrap();
+    let id = info["transactions"][0]["id"].as_str().unwrap();
+    assert_eq!(info["transactions"][0]["updated"], 2);
+    let accepted = root.command(&["recover-edits", "--finalize", id]);
+    assert!(accepted.status.success(), "{}", utf8(&accepted.stderr));
+    assert!(utf8(&accepted.stdout).contains("finalized="));
+    assert_eq!(fs::read_to_string(root.path().join("a.rs")).unwrap(), "after a");
+    assert_eq!(fs::read_to_string(root.path().join("b.rs")).unwrap(), "after b");
+    assert!(root.command(&["doctor"]).status.success());
+}
