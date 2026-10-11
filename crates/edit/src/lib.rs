use blake3::Hasher;
use std::{collections::HashSet, fs, io, path::{Component, Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Clone)]
pub enum Operation { SearchReplace { path: PathBuf, expected_hash: String, old: String, new: String }, ReplaceRange { path: PathBuf, expected_hash: String, start: usize, end: usize, new: String } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied { pub path: PathBuf, pub hash: String }

pub fn apply(workspace: impl AsRef<Path>, operations: &[Operation]) -> io::Result<Vec<Applied>> {
    let workspace = workspace.as_ref().canonicalize()?;
    let lock_path = workspace.join(".conduit-edit.lock");
    let _lock = Lock::acquire(&lock_path)?;
    let mut staged = Vec::new();
    let mut paths = HashSet::new();
    for operation in operations {
        let prepared = match prepare(&workspace, operation) {
            Ok(value) => value,
            Err(error) => { cleanup(&staged); return Err(error); }
        };
        let (relative, expected, updated) = prepared;
        if !paths.insert(relative.clone()) { cleanup(&staged); return Err(io::Error::new(io::ErrorKind::InvalidInput, "duplicate path in transaction")); }
        let path = match checked_path(&workspace, &relative) { Ok(value) => value, Err(error) => { cleanup(&staged); return Err(error); } };
        let metadata = match fs::metadata(&path) { Ok(value) => value, Err(error) => { cleanup(&staged); return Err(error); } };
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let temp = path.with_extension(format!("conduit-tmp-{}-{}", std::process::id(), nonce));
        if let Err(error) = fs::write(&temp, updated.as_bytes()).and_then(|_| fs::set_permissions(&temp, metadata.permissions())) {
            let _ = fs::remove_file(&temp); cleanup(&staged); return Err(error);
        }
        staged.push((relative, path, temp, expected, updated));
    }
    let mut committed: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (_, path, _, expected, _) in &staged {
        let current = match fs::read(path) { Ok(value) => value, Err(error) => { cleanup(&staged); return Err(error); } };
        if hash(&current) != *expected { cleanup(&staged); return Err(io::Error::new(io::ErrorKind::WouldBlock, "file changed during transaction")); }
    }
    for (_, path, temp, _, _) in &staged {
        let old = match fs::read(path) { Ok(value) => value, Err(error) => { cleanup(&staged); rollback(&committed); return Err(error); } };
        if let Err(error) = fs::rename(temp, path) { cleanup(&staged); rollback(&committed); return Err(error); }
        committed.push((path.clone(), old));
    }
    Ok(staged.into_iter().map(|(relative, _, _, _, updated)| Applied { path: relative, hash: hash(updated.as_bytes()) }).collect())
}

fn prepare(workspace: &Path, operation: &Operation) -> io::Result<(PathBuf, String, String)> {
    let (relative, expected, updated) = match operation {
        Operation::SearchReplace { path, expected_hash, old, new } => {
            let relative = safe_relative(path)?; let target = checked_path(workspace, &relative)?; let bytes = fs::read(target)?; let actual = hash(&bytes);
            if actual != *expected_hash { return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed")); }
            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not utf-8"))?;
            let count = text.matches(old).count(); if count != 1 { return Err(io::Error::new(io::ErrorKind::InvalidInput, "replacement must match exactly once")); }
            (relative, actual, text.replacen(old, new, 1))
        }
        Operation::ReplaceRange { path, expected_hash, start, end, new } => {
            let relative = safe_relative(path)?; let target = checked_path(workspace, &relative)?; let bytes = fs::read(target)?; let actual = hash(&bytes);
            if actual != *expected_hash { return Err(io::Error::new(io::ErrorKind::WouldBlock, "file hash changed")); }
            let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not utf-8"))?;
            if *start > *end || *end > text.len() || !text.is_char_boundary(*start) || !text.is_char_boundary(*end) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid UTF-8 range")); }
            (relative, actual, format!("{}{}{}", &text[..*start], new, &text[*end..]))
        }
    };
    Ok((relative, expected, updated))
}

fn safe_relative(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() || path.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace")); }
    Ok(path.to_path_buf())
}
fn checked_path(workspace: &Path, relative: &Path) -> io::Result<PathBuf> {
    let target = workspace.join(relative).canonicalize()?;
    if !target.starts_with(workspace) { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workspace through symlink")); }
    Ok(target)
}

fn hash(bytes: &[u8]) -> String { let mut hasher = Hasher::new(); hasher.update(bytes); hasher.finalize().to_hex().to_string() }
fn cleanup(staged: &[(PathBuf, PathBuf, PathBuf, String, String)]) { for (_, _, temp, _, _) in staged { let _ = fs::remove_file(temp); } }
fn rollback(committed: &[(PathBuf, Vec<u8>)]) { for (path, old) in committed { let _ = fs::write(path, old); } }

struct Lock { path: PathBuf }
impl Lock { fn acquire(path: &Path) -> io::Result<Self> { fs::create_dir_all(path.parent().expect("lock path has parent"))?; fs::OpenOptions::new().write(true).create_new(path).open(path)?; Ok(Self { path: path.to_path_buf() }) } }
impl Drop for Lock { fn drop(&mut self) { let _ = fs::remove_file(&self.path); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn applies_unique_edit_and_rejects_stale_hash() { let root = std::env::temp_dir().join(format!("conduit-edit-{}", std::process::id())); fs::create_dir_all(&root).unwrap(); let path = root.join("a.txt"); fs::write(&path, "before").unwrap(); let expected = hash(b"before"); let applied = apply(&root, &[Operation::SearchReplace { path: PathBuf::from("a.txt"), expected_hash: expected.clone(), old: "before".into(), new: "after".into() }]).unwrap(); assert_eq!(applied[0].path, PathBuf::from("a.txt")); assert_eq!(fs::read_to_string(&path).unwrap(), "after"); assert!(apply(&root, &[Operation::SearchReplace { path: PathBuf::from("a.txt"), expected_hash: expected, old: "after".into(), new: "bad".into() }]).is_err()); let _ = fs::remove_dir_all(root); }
    #[test]
    fn rejects_ambiguous_and_escape_edits() { let root = std::env::temp_dir().join(format!("conduit-edit-reject-{}", std::process::id())); fs::create_dir_all(&root).unwrap(); let path = root.join("a"); fs::write(&path, "x x").unwrap(); let expected = hash(b"x x"); assert!(apply(&root, &[Operation::SearchReplace { path: PathBuf::from("a"), expected_hash: expected, old: "x".into(), new: "y".into() }]).is_err()); assert!(safe_relative(Path::new("../a")).is_err()); let _ = fs::remove_dir_all(root); }
}
