use blake3::Hasher;
use std::{fs, io, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRef { pub handle: String, pub hash: String, pub bytes: u64 }

pub struct BlobStore { root: PathBuf }

impl BlobStore {
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn put(&self, data: &[u8]) -> io::Result<BlobRef> {
        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize().to_hex().to_string();
        let path = self.root.join(&hash[0..2]).join(&hash);
        if !path.exists() {
            fs::create_dir_all(path.parent().expect("hash path has parent"))?;
            let temp = path.with_extension("tmp");
            fs::write(&temp, data)?;
            fs::rename(temp, &path)?;
        }
        Ok(BlobRef { handle: format!("cx:o:{}", base32_prefix(&hasher.finalize().as_bytes()[..12])), hash, bytes: data.len() as u64 })
    }

    pub fn get(&self, handle: &str) -> io::Result<Vec<u8>> {
        let hash = handle.strip_prefix("cx:o:").ok_or_else(|| invalid("invalid output handle"))?;
        if hash.len() != 20 || !hash.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) { return Err(invalid("invalid output handle")); }
        let mut matches = Vec::new();
        for prefix in fs::read_dir(&self.root)? {
            let prefix = prefix?.path();
            if prefix.is_dir() {
                for item in fs::read_dir(prefix)? {
                    let item = item?.path();
                    if item.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with(hash)).unwrap_or(false) { matches.push(item); }
                }
            }
        }
        match matches.as_slice() { [path] => fs::read(path), [] => Err(io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown")), _ => Err(invalid("ambiguous handle")) }
    }
}

fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }
fn base32_prefix(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut out = String::new(); let mut buffer = 0u16; let mut bits = 0u8;
    for &byte in bytes { buffer = (buffer << 8) | byte as u16; bits += 8; while bits >= 5 { bits -= 5; out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char); } }
    if bits > 0 { out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stores_deduplicated_content_and_expands_handle() {
        let dir = std::env::temp_dir().join(format!("conduit-store-{}", std::process::id()));
        let store = BlobStore::open(&dir).unwrap();
        let first = store.put(b"same").unwrap(); let second = store.put(b"same").unwrap();
        assert_eq!(first, second); assert_eq!(store.get(&first.handle).unwrap(), b"same");
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn rejects_non_output_handles() { let dir = std::env::temp_dir().join("conduit-store-invalid"); let store = BlobStore::open(&dir).unwrap(); assert!(store.get("nope").is_err()); let _ = fs::remove_dir_all(dir); }
}
