//! Hash-anchored best-effort batch edits. Multi-file crash atomicity is NOT
//! provided; a journaled filesystem transaction remains a separate project.
//! Preflight validation is shared by previews and commits; a preview does not
//! reserve files against non-cooperating writers.
use blake3::Hasher;
use fs2::FileExt;
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const MAX_EDIT_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_OPERATIONS: usize = 128;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operation {
    SearchReplace { path: PathBuf, expected_hash: String, old: String, new: String },
    ReplaceRange { path: PathBuf, expected_hash: String, start: usize, end: usize, new: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied { pub path: PathBuf, pub hash: String }

struct Staged {
    relative: PathBuf,
    path: PathBuf,
    temp: PathBuf,
    expected: String,
    updated: String,
    previous: Vec<u8>,
    permissions: Permissions,
}

struct Planned {
    relative: PathBuf,
    path: PathBuf,
    expected: String,
    updated: String,
    previous: Vec<u8>,
    permissions: Permissions,
}

/// Validate every target without writing a source file. This still creates a
/// cooperative lock under `.conduit` and does NOT reserve inputs after return.
pub fn preview(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
    let workspace = workspace.as_ref().canonicalize()?;
    let _lock = lock_workspace(&workspace)?;
    let planned = validate(&workspace, operations)?;
    Ok(planned.iter().map(|item| Applied {
        path: item.relative.clone(), hash: hash(item.updated.as_bytes()),
    }).collect())
}

pub fn apply(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
    let workspace = workspace.as_ref().canonicalize()?;
    let _lock = lock_workspace(&workspace)?;
    let planned = validate(&workspace, operations)?;
    let mut staged = Vec::with_capacity(planned.len());
    let result = (|| {
        for item in planned {
            let temp = create_staged(&item.path, item.updated.as_bytes(), &item.permissions)?;
            staged.push(Staged {
                relative: item.relative, path: item.path, temp,
                expected: item.expected, updated: item.updated,
                previous: item.previous, permissions: item.permissions,
            });
        }
        // Cooperative validation narrows races, but external writers can
        // modify files between this check and the OS rename operation.
        for item in &staged { verify_expected(&item.path, &item.expected)?; }
        let mut committed = 0usize;
        for item in &staged {
            let result = verify_expected(&item.path, &item.expected)
                .and_then(|_| fs::rename(&item.temp, &item.path));
            if result.is_ok() { committed += 1; }
            let result = result.and_then(|_| sync_parent(&item.path));
            if let Err(error) = result {
                let mut rollback_error = None;
                for prior in staged[..committed].iter().rev() {
                    if let Err(failed) = restore(prior) { rollback_error.get_or_insert(failed); }
                }
                return Err(rollback_error.unwrap_or(error));
            }
        }
        Ok(staged.iter().map(|item| Applied {
            path: item.relative.clone(), hash: hash(item.updated.as_bytes()),
        }).collect())
    })();
    for item in &staged { let _ = fs::remove_file(&item.temp); }
    result
}

fn lock_workspace(workspace: &Path) -> io::Result<Lock> {
    let state_dir = workspace.join(".conduit");
    fs::create_dir_all(&state_dir)?;
    if !fs::symlink_metadata(&state_dir)?.file_type().is_dir() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "workspace state directory must not be a symlink"));
    }
    Lock::acquire(&state_dir.join("edit.lock"))
}

fn validate(workspace: &Path, operations: &[Operation]) -> io::Result<Vec<Planned>> {
    if operations.is_empty() || operations.len() > MAX_OPERATIONS {
        return Err(invalid("edit batch must contain 1-128 operations"));
    }
    let mut planned = Vec::with_capacity(operations.len());
    let mut paths = HashSet::new();
    #[cfg(unix)]
    let mut inodes = HashSet::new();
    let mut total_bytes = 0usize;
    for operation in operations {
        let (relative, expected, updated) = prepare(workspace, operation)?;
        let path = checked_path(workspace, &relative)?;
        if !paths.insert(path.clone()) {
            return Err(invalid("two operations resolve to the same canonical file"));
        }
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_EDIT_FILE_BYTES as u64 {
            return Err(invalid("edit target must be a regular file <=8 MiB"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() > 1 {
                return Err(invalid("edit would silently break a hardlinked file identity"));
            }
            if !inodes.insert((metadata.dev(), metadata.ino())) {
                return Err(invalid("two operations reference the same hardlinked inode"));
            }
        }
        let previous = read_bounded(&path)?;
        if hash(&previous) != expected {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "file changed during edit preparation"));
        }
        if updated.len() > MAX_EDIT_FILE_BYTES {
            return Err(invalid("updated file would exceed 8 MiB"));
        }
        total_bytes = total_bytes.checked_add(previous.len()).and_then(|value| value.checked_add(updated.len()))
            .ok_or_else(|| invalid("edit batch size overflow"))?;
        if total_bytes > MAX_BATCH_BYTES {
            return Err(invalid("edit batch input/output exceeds 64 MiB"));
        }
        planned.push(Planned {
            relative, path, expected, updated, previous,
            permissions: metadata.permissions(),
        });
    }
    Ok(planned)
}

fn verify_expected(path: &Path, expected: &str) -> io::Result<()> {
    if hash(&read_bounded(path)?) != expected {
        return Err(io::Error::new(io::ErrorKind::WouldBlock, "file changed during edit transaction"));
    }
    Ok(())
}

fn create_staged(path: &Path, bytes: &[u8], permissions: &Permissions) -> io::Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| invalid("target has no parent"))?;
    for _ in 0..16 {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".conduit-edit-{}-{serial}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let result = (|| {
            file.write_all(bytes)?;
            file.set_permissions(permissions.clone())?;
            file.sync_all()?;
            sync_parent(&temp)
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
        return Ok(temp);
    }
    Err(invalid("unable to create unique edit staging file"))
}

fn restore(item: &Staged) -> io::Result<()> {
    let temp = create_staged(&item.path, &item.previous, &item.permissions)?;
    let result = fs::rename(&temp, &item.path).and_then(|_| sync_parent(&item.path));
    if result.is_err() { let _ = fs::remove_file(temp); }
    result
}

fn prepare(workspace: &Path, operation: &Operation) -> io::Result<(PathBuf, String, String)> {
    match operation {
        Operation::SearchReplace { path, expected_hash, old, new } => {
            if old.is_empty() { return Err(invalid("search text must not be empty")); }
            let relative = safe_relative(path)?;
            let bytes = read_bounded(&checked_path(workspace, &relative)?)?;
            let actual = hash(&bytes);
            if actual != *expected_hash {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed"));
            }
            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not UTF-8"))?;
            if text.matches(old).count() != 1 {
                return Err(invalid("replacement must match exactly once"));
            }
            Ok((relative, actual, text.replacen(old, new, 1)))
        }
        Operation::ReplaceRange { path, expected_hash, start, end, new } => {
            let relative = safe_relative(path)?;
            let bytes = read_bounded(&checked_path(workspace, &relative)?)?;
            let actual = hash(&bytes);
            if actual != *expected_hash {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed"));
            }
            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not UTF-8"))?;
            if start > end || *end > text.len() || !text.is_char_boundary(*start) || !text.is_char_boundary(*end) {
                return Err(invalid("invalid UTF-8 range"));
            }
            Ok((relative, actual, format!("{}{}{}", &text[..*start], new, &text[*end..])))
        }
    }
}

fn safe_relative(path: &Path) -> io::Result<PathBuf> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace"));
    }
    Ok(path.to_path_buf())
}

fn checked_path(workspace: &Path, relative: &Path) -> io::Result<PathBuf> {
    let target = workspace.join(relative).canonicalize()?;
    if !target.starts_with(workspace) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace through symlink"));
    }
    let inner = target.strip_prefix(workspace).unwrap_or(&target);
    for component in relative.components().chain(inner.components()) {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), ".git" | ".conduit" | ".noqte" | ".claude" | ".ssh" | ".aws" | ".secrets" | "secrets") || name.starts_with(".env") {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "editing protected workspace paths is disabled"));
            }
        }
    }
    Ok(target)
}

fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_EDIT_FILE_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_EDIT_FILE_BYTES {
        return Err(invalid("edit target exceeds 8 MiB"));
    }
    Ok(bytes)
}

fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }
fn hash(bytes: &[u8]) -> String { let mut hasher = Hasher::new(); hasher.update(bytes); hasher.finalize().to_hex().to_string() }

fn sync_parent(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path.parent().ok_or_else(|| invalid("no parent directory"))?)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

// The lock is held by the OS and released on process exit. An ordinary
// lockfile survives crashes without permanently preventing later edits.
// This coordinates Conduit instances, not external processes editing files.
struct Lock { file: File }
impl Lock {
    fn acquire(path: &Path) -> io::Result<Self> {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if !metadata.file_type().is_file() {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "edit lock path is not a regular file"));
            }
        }
        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }
}
impl Drop for Lock {
    fn drop(&mut self) { let _ = FileExt::unlock(&self.file); }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp_root(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!("conduit-edit-{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&root).unwrap();
        root
    }
    fn replace(path: &str, expected: &str, old: &str, new: &str) -> Operation {
        Operation::SearchReplace { path: path.into(), expected_hash: expected.into(), old: old.into(), new: new.into() }
    }
    #[test]
    fn preexisting_unlocked_lockfile_does_not_block_future_edits() {
        let root = temp_root("orphan-lock");
        let path = root.join(".conduit-edit.lock");
        fs::write(&path, b"left by crashed process").unwrap();
        let first = Lock::acquire(&path).unwrap();
        drop(first);
        let second = Lock::acquire(&path).unwrap();
        drop(second);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn edits_one_file_and_rejects_stale_hash() {
        let root = temp_root("single");
        let path = root.join("a.txt");
        fs::write(&path, "before").unwrap();
        let expected = hash(b"before");
        assert_eq!(apply(&root, &[replace("a.txt", &expected, "before", "after")]).unwrap().len(), 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), "after");
        assert!(apply(&root, &[replace("a.txt", &expected, "after", "bad")]).is_err());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn rejects_duplicate_target_and_empty_search() {
        let root = temp_root("duplicate");
        fs::write(root.join("a.txt"), "before").unwrap();
        let expected = hash(b"before");
        assert!(apply(&root, &[replace("a.txt", &expected, "before", "one"), replace("a.txt", &expected, "before", "two")]).is_err());
        assert!(apply(&root, &[replace("a.txt", &expected, "", "two")]).is_err());
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "before");
        let _ = fs::remove_dir_all(root);
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_alias_to_same_target() {
        use std::os::unix::fs::symlink;
        let root = temp_root("alias");
        fs::write(root.join("real.rs"), "before").unwrap();
        symlink(root.join("real.rs"), root.join("alias.rs")).unwrap();
        let expected = hash(b"before");
        let result = apply(&root, &[replace("real.rs", &expected, "before", "one"), replace("alias.rs", &expected, "before", "two")]);
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(root.join("real.rs")).unwrap(), "before");
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn prevents_path_escape() {
        let root = temp_root("escape");
        assert!(safe_relative(Path::new("../a")).is_err());
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "before").unwrap();
        assert!(apply(&root, &[replace(".git/config", &hash(b"before"), "before", "after")]).is_err());
        fs::create_dir(root.join(".noqte")).unwrap();
        fs::write(root.join(".noqte/config"), "before").unwrap();
        fs::write(root.join(".env.local"), "before").unwrap();
        for name in [".noqte/config", ".env.local"] {
            assert_eq!(apply(&root, &[replace(name, &hash(b"before"), "before", "after")]).unwrap_err().kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(fs::read_to_string(root.join(name)).unwrap(), "before");
        }
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn preview_hash_matches_commit_without_modifying_source() {
        let root = std::env::temp_dir().join(format!("conduit-preview-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.txt"), "one two").unwrap();
        let ops = [Operation::SearchReplace {
            path: "file.txt".into(), expected_hash: hash(b"one two"),
            old: "two".into(), new: "three".into(),
        }];
        let preview = preview(&root, &ops).unwrap();
        assert_eq!(fs::read_to_string(root.join("file.txt")).unwrap(), "one two");
        assert_eq!(preview[0].hash, hash(b"one three"));
        let applied = apply(&root, &ops).unwrap();
        assert_eq!(preview, applied);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_oversized_input_and_empty_batch_before_editing() {
        let root = std::env::temp_dir().join(format!("conduit-large-edit-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        assert!(preview(&root, &[]).is_err());
        fs::write(root.join("huge.txt"), vec![b'x'; MAX_EDIT_FILE_BYTES + 1]).unwrap();
        assert!(apply(&root, &[Operation::SearchReplace {
            path: "huge.txt".into(), expected_hash: "0".repeat(64),
            old: "x".into(), new: "y".into(),
        }]).is_err());
        assert_eq!(fs::metadata(root.join("huge.txt")).unwrap().len(), (MAX_EDIT_FILE_BYTES + 1) as u64);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_typed_batch_manifest_variants() {
        let manifest = r#"[
          {"type":"search_replace","path":"a.rs","expected_hash":"abc","old":"foo","new":"bar"},
          {"type":"replace_range","path":"b.rs","expected_hash":"def","start":0,"end":2,"new":"x"}
        ]"#;
        let operations: Vec<Operation> = serde_json::from_str(manifest).unwrap();
        assert_eq!(operations.len(), 2);
    }
}
