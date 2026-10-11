use blake3::Hasher;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_BLOB_BYTES: usize = 64 * 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRef {
    pub handle: String,
    pub hash: String,
    pub bytes: u64,
}

pub struct BlobStore {
    root: PathBuf,
}

#[derive(Debug, Default)]
pub struct Audit {
    pub checked_blobs: u64,
    pub verified_bytes: u64,
    pub staging_files: u64,
    pub problem_count: u64,
    /// The first 32 problems. This list is intentionally bounded.
    pub problems: Vec<String>,
}

impl Audit {
    fn problem(&mut self, description: String) {
        self.problem_count = self.problem_count.saturating_add(1);
        if self.problems.len() < 32 { self.problems.push(description); }
    }
}


impl BlobStore {
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        private_directory(&root)?;
        Ok(Self { root })
    }

    pub fn put(&self, data: &[u8]) -> io::Result<BlobRef> {
        if data.len() > MAX_BLOB_BYTES {
            return Err(invalid("blob exceeds the 64 MiB store limit"));
        }
        let hash = digest(data);
        let directory = self.root.join(&hash[..2]);
        private_directory(&directory)?;
        let path = directory.join(&hash);
        if path.exists() {
            // An existing digest name is not proof of intact content.
            verify_blob(&path, &hash)?;
        } else {
            self.publish(&directory, &path, data)?;
            verify_blob(&path, &hash)?;
        }
        Ok(BlobRef {
            handle: handle_for_hash(&hash),
            hash,
            bytes: data.len() as u64,
        })
    }

    fn publish(&self, directory: &Path, final_path: &Path, data: &[u8]) -> io::Result<()> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut opened = None;
        for _ in 0..8 {
            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let temp = directory.join(format!(
                ".conduit-{}-{nonce}-{serial}.tmp",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&temp) {
                Ok(file) => {
                    opened = Some((temp, file));
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        let (temp, mut file) = opened.ok_or_else(|| invalid("unable to create unique blob staging file"))?;
        let result = (|| {
            private_file(&file)?;
            file.write_all(data)?;
            file.sync_all()?;
            drop(file);
            // Unlike rename, hard_link refuses to replace a concurrently committed blob.
            match fs::hard_link(&temp, final_path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
            sync_directory(directory)?;
            Ok(())
        })();
        let _ = fs::remove_file(&temp);
        result
    }

    /// Explicit on-demand integrity scan. Does not delete objects, follow
    /// symlinks, or promise a stable snapshot during concurrent writers.
    /// Runs in O(total stored bytes) time with bounded per-file memory.
    pub fn audit(&self) -> io::Result<Audit> {
        let mut report = Audit::default();
        for item in fs::read_dir(&self.root)? {
            let item = item?;
            let name = item.file_name();
            let Some(shard) = name.to_str() else {
                report.problem("non-UTF8 shard directory".into());
                continue;
            };
            if shard.len() != 2 || !is_hex(shard) || !item.file_type()?.is_dir() {
                report.problem(format!("unexpected object in store root: {shard}"));
                continue;
            }
            for entry in fs::read_dir(item.path())? {
                let entry = entry?;
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    report.problem(format!("non-UTF8 blob name in {shard}"));
                    continue;
                };
                if name.starts_with(".conduit-") && name.ends_with(".tmp") {
                    report.staging_files = report.staging_files.saturating_add(1);
                    continue;
                }
                if !is_full_hash(name) || !name.starts_with(shard) || !entry.file_type()?.is_file() {
                    report.problem(format!("unexpected blob entry {shard}/{name}"));
                    continue;
                }
                match verify_blob(&entry.path(), name) {
                    Ok(bytes) => {
                        report.checked_blobs = report.checked_blobs.saturating_add(1);
                        report.verified_bytes = report.verified_bytes.saturating_add(bytes.len() as u64);
                    }
                    Err(error) => report.problem(format!("unverifiable blob {shard}/{name}: {error}")),
                }
            }
        }
        Ok(report)
    }

    pub fn get(&self, handle: &str) -> io::Result<Vec<u8>> {
        let encoded = handle
            .strip_prefix("cx:o:")
            .ok_or_else(|| invalid("invalid output handle"))?;
        let shard = shard_from_handle(encoded)?;
        let directory = self.root.join(shard);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Err(invalid("blob shard is not a directory")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown"));
            }
            Err(error) => return Err(error),
        }
        let mut match_path = None;
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            // Strictly exclude staging files and arbitrarily named path aliases.
            if !is_full_hash(name) || handle_for_hash(name) != handle {
                continue;
            }
            if match_path.replace((entry.path(), name.to_owned())).is_some() {
                return Err(invalid("ambiguous output handle"));
            }
        }
        let (path, hash) = match_path.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown")
        })?;
        verify_blob(&path, &hash)
    }
}

fn digest(data: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize().to_hex().to_string()
}

fn verify_blob(path: &Path, expected_hash: &str) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_BLOB_BYTES as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid stored blob type or size"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "stored blob is not a regular file"));
    }
    let mut bytes = Vec::new();
    use io::Read;
    file.take(MAX_BLOB_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BLOB_BYTES || digest(&bytes) != expected_hash {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "stored blob failed digest verification"));
    }
    Ok(bytes)
}

fn is_hex(name: &str) -> bool {
    name.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_full_hash(name: &str) -> bool { name.len() == 64 && is_hex(name) }

fn shard_from_handle(encoded: &str) -> io::Result<String> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    if encoded.len() != 20 || !encoded.bytes().all(|c| ALPHABET.contains(&c)) {
        return Err(invalid("invalid output handle"));
    }
    let bytes = encoded.as_bytes();
    let first = ALPHABET.iter().position(|c| *c == bytes[0]).expect("validated alphabet") as u8;
    let second = ALPHABET.iter().position(|c| *c == bytes[1]).expect("validated alphabet") as u8;
    let first_hash_byte = (first << 3) | (second >> 2);
    Ok(format!("{first_hash_byte:02x}"))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn handle_for_hash(hash: &str) -> String {
    let mut bytes = [0u8; 12];
    for (index, chunk) in hash.as_bytes().chunks(2).take(12).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("00"), 16)
            .unwrap_or(0);
    }
    format!("cx:o:{}", base32_prefix(&bytes))
}

fn base32_prefix(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for &byte in bytes {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
            buffer &= (1u32 << bits).saturating_sub(1);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "blob directory is a symlink or non-directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn private_file(file: &File) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = file;
    Ok(())
}

fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "conduit-store-{name}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn stores_deduplicated_content_and_expands_handle() {
        let dir = temp_root("dedup");
        let store = BlobStore::open(&dir).unwrap();
        let first = store.put(b"same").unwrap();
        let second = store.put(b"same").unwrap();
        assert_eq!(first, second);
        assert_eq!(store.get(&first.handle).unwrap(), b"same");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn excludes_staging_files_from_handle_lookup() {
        let dir = temp_root("staging");
        let store = BlobStore::open(&dir).unwrap();
        let blob = store.put(b"content").unwrap();
        let stage = dir.join(&blob.hash[..2]).join(format!("{}.tmp-race", blob.hash));
        fs::write(stage, b"partial").unwrap();
        assert_eq!(store.get(&blob.handle).unwrap(), b"content");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_corrupted_existing_blob_and_expansion() {
        let dir = temp_root("corrupt");
        let store = BlobStore::open(&dir).unwrap();
        let blob = store.put(b"original").unwrap();
        fs::write(dir.join(&blob.hash[..2]).join(&blob.hash), b"tampered").unwrap();
        assert_eq!(store.get(&blob.handle).unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert!(store.put(b"original").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_non_output_handles() {
        let dir = temp_root("invalid");
        let store = BlobStore::open(&dir).unwrap();
        for bad in ["nope", "cx:o:../../xx", "cx:o:aaaaaaaaaaaaaaa00000"] {
            assert!(store.get(bad).is_err());
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_blob_without_following_it() {
        use std::os::unix::fs::symlink;
        let dir = temp_root("symlink");
        let store = BlobStore::open(&dir).unwrap();
        let blob = store.put(b"private").unwrap();
        let path = dir.join(&blob.hash[..2]).join(&blob.hash);
        let external = dir.join("external");
        fs::rename(&path, &external).unwrap();
        symlink(&external, &path).unwrap();
        assert!(store.get(&blob.handle).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrently_commits_identical_content() {
        let dir = temp_root("concurrent");
        let store = std::sync::Arc::new(BlobStore::open(&dir).unwrap());
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let store = store.clone();
                std::thread::spawn(move || store.put(b"shared content").unwrap())
            })
            .collect();
        let blobs: Vec<_> = threads.into_iter().map(|thread| thread.join().unwrap()).collect();
        assert!(blobs.iter().all(|blob| blob == &blobs[0]));
        assert_eq!(store.get(&blobs[0].handle).unwrap(), b"shared content");
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn audit_detects_corruption_without_deleting_or_rewriting_objects() {
        let root = std::env::temp_dir().join(format!("conduit-blob-audit-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let store = BlobStore::open(&root).unwrap();
        let valid = store.put(b"good blob").unwrap();
        let invalid = store.put(b"different blob").unwrap();
        let before = store.audit().unwrap();
        assert_eq!(before.checked_blobs, 2);
        assert_eq!(before.problem_count, 0);
        assert_eq!(before.verified_bytes, 23); // 9 + 14
        fs::write(root.join(&invalid.hash[..2]).join(&invalid.hash), b"changed").unwrap();
        let after = store.audit().unwrap();
        assert_eq!(after.checked_blobs, 1);
        assert_eq!(after.problem_count, 1);
        assert!(store.get(&valid.handle).is_ok());
        assert!(store.get(&invalid.handle).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn audit_counts_staging_without_following_fake_entries() {
        let root = std::env::temp_dir().join(format!("conduit-blob-stage-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let store = BlobStore::open(&root).unwrap();
        let valid = store.put(b"x").unwrap();
        fs::write(root.join(&valid.hash[..2]).join(".conduit-123-456.tmp"), b"partial").unwrap();
        let audit = store.audit().unwrap();
        assert_eq!(audit.checked_blobs, 1);
        assert_eq!(audit.staging_files, 1);
        assert_eq!(audit.problem_count, 0);
        let _ = fs::remove_dir_all(root);
    }
}
