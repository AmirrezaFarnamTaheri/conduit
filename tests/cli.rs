//! End-to-end CLI behavior checks; run by `cargo test --workspace --all-targets`.
//! All tests use isolated temporary workspaces and local processes only.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "conduit-cli-{}-{}", std::process::id(), NEXT_TEST.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path { &self.0 }
    fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conduit"))
            .args(args)
            .env("CONDUIT_HOME", self.0.join(".conduit"))
            .env("CONDUIT_WORKSPACE", &self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}
fn utf8(bytes: &[u8]) -> &str { std::str::from_utf8(bytes).unwrap() }
fn hash_from_peek(output: &Output) -> String {
    assert!(output.status.success(), "{}", utf8(&output.stderr));
    utf8(&output.stdout)
        .split_whitespace()
        .find_map(|field| field.strip_prefix("hash="))
        .expect("peek emits a hash")
        .to_owned()
}

#[test]
fn store_expand_and_doctor_round_trip() {
    let root = Workspace::new();
    let saved = root.command(&["store", "roundtrip bytes"]);
    assert!(saved.status.success(), "{}", utf8(&saved.stderr));
    let handle = utf8(&saved.stdout).trim();
    assert!(handle.starts_with("cx:o:"));
    let fetched = root.command(&["expand", handle]);
    assert!(fetched.status.success());
    assert_eq!(&fetched.stdout, b"roundtrip bytes");
    let metadata = root.command(&["inspect", handle]);
    assert!(metadata.status.success());
    assert!(utf8(&metadata.stdout).contains("bytes=15"));
    let window = root.command(&["expand", handle, "--offset", "5", "--length", "4"]);
    assert!(window.status.success());
    assert_eq!(window.stdout, b"trip");
    assert!(!root.command(&["expand", handle, "--offset", "15", "--length", "1"]).status.success());
    let doctor = root.command(&["doctor"]);
    assert!(doctor.status.success(), "{}", utf8(&doctor.stderr));
    assert!(utf8(&doctor.stdout).contains("integrity_problems=0"));
    let shard_dir = root.path().join(".conduit/blobs");
    let shard = fs::read_dir(&shard_dir).unwrap().next().unwrap().unwrap().path();
    let blob = fs::read_dir(&shard).unwrap().next().unwrap().unwrap().path();
    fs::write(&blob, b"wrong payload").unwrap();
    let corrupt = root.command(&["doctor"]);
    assert!(!corrupt.status.success(), "corrupted blob must not pass a health scan");
    assert!(utf8(&corrupt.stdout).contains("integrity_problems=1"));
    assert!(!root.command(&["expand", handle]).status.success());
    let damaged_window = root.command(&["expand", handle, "--offset", "0", "--length", "4"]);
    assert!(!damaged_window.status.success());
    assert!(damaged_window.stdout.is_empty());
    assert!(!root.command(&["inspect", handle]).status.success());
    let bogus = root.command(&["expand", "cx:o:../../etc/passwd"]);
    assert!(!bogus.status.success());
}

#[test]
fn find_peek_and_batch_edit_preflight_and_commit() {
    let root = Workspace::new();
    fs::write(root.path().join("app.rs"), "fn before() {}\n").unwrap();
    fs::write(root.path().join(".env"), "fn before() {}\n").unwrap();
    let find = root.command(&["find", "before", "20"]);
    assert!(find.status.success(), "{}", utf8(&find.stderr));
    assert!(utf8(&find.stdout).contains("app.rs:1:"));
    assert!(!utf8(&find.stdout).contains(".env:"));
    let peek = root.command(&["peek", "app.rs", "1", "20"]);
    let hash = hash_from_peek(&peek);

    let manifest = format!(
        r#"{{"operations":[{{"type":"search_replace","path":"app.rs","expected_hash":"{hash}","old":"before","new":"after"}}]}}"#
    );
    let manifest_path = root.path().join("plan.json");
    fs::write(&manifest_path, manifest).unwrap();
    let path_string = manifest_path.to_str().unwrap();
    let dry_run = root.command(&["batch-edit", "--check", path_string]);
    assert!(dry_run.status.success(), "{}", utf8(&dry_run.stderr));
    assert_eq!(fs::read_to_string(root.path().join("app.rs")).unwrap(), "fn before() {}\n");
    assert!(utf8(&dry_run.stdout).contains("validated=app.rs"));

    let edit = root.command(&["batch-edit", path_string]);
    assert!(edit.status.success(), "{}", utf8(&edit.stderr));
    assert_eq!(fs::read_to_string(root.path().join("app.rs")).unwrap(), "fn after() {}\n");
    let stale = root.command(&["batch-edit", path_string]);
    assert!(!stale.status.success(), "stale hash must reject reapplication");
    let escape = root.command(&["peek", "../outside", "1", "20"]);
    assert!(!escape.status.success());
}

#[test]
fn batch_preflight_rejects_stale_second_target_before_first_mutation() {
    let root = Workspace::new();
    fs::write(root.path().join("first.rs"), "one").unwrap();
    fs::write(root.path().join("second.rs"), "two").unwrap();
    let first = hash_from_peek(&root.command(&["peek", "first.rs"]));
    let manifest = format!(r#"{{"operations":[
        {{"type":"search_replace","path":"first.rs","expected_hash":"{first}","old":"one","new":"changed"}},
        {{"type":"search_replace","path":"second.rs","expected_hash":"{}","old":"two","new":"changed"}}
    ]}}"#, "0".repeat(64));
    let path = root.path().join("stale.json");
    fs::write(&path, manifest).unwrap();
    let result = root.command(&["batch-edit", path.to_str().unwrap()]);
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(root.path().join("first.rs")).unwrap(), "one");
    assert_eq!(fs::read_to_string(root.path().join("second.rs")).unwrap(), "two");
}

#[test]
fn session_repair_discards_only_unterminated_tail() {
    let root = Workspace::new();
    assert!(root.command(&["record", "session1", "test", "20", "10"]).status.success());
    let log_path = root.path().join(".conduit/sessions/session1.jsonl");
    use std::io::Write;
    fs::OpenOptions::new().append(true).open(&log_path).unwrap().write_all(b"{\"type\":").unwrap();
    assert!(!root.command(&["stats", "session1"]).status.success());
    assert!(!root.command(&["record", "session1", "test", "1", "1"]).status.success());
    let fixed = root.command(&["repair-session", "session1"]);
    assert!(fixed.status.success(), "{}", utf8(&fixed.stderr));
    assert!(utf8(&fixed.stdout).contains("partial_tail_removed=true"));
    assert!(root.command(&["record", "session1", "test", "2", "1"]).status.success());
    let stats = root.command(&["stats", "session1"]);
    assert!(stats.status.success());
    assert!(utf8(&stats.stdout).contains("raw_units=22 presented_units=11"));
}

#[test]
fn run_preserves_command_status_and_streams() {
    let root = Workspace::new();
    // Launch a second Conduit process; no shell dependency on Windows.
    let executable = env!("CARGO_BIN_EXE_conduit");
    let output = root.command(&["run", "--preset", "quick", "--timeout-ms", "5000", "--", executable, "store", "subprocess"]);
    assert!(output.status.success(), "{}", utf8(&output.stderr));
    assert!(utf8(&output.stdout).starts_with("cx:o:"));
}
