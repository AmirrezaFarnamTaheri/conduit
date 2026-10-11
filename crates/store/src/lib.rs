use blake3::Hasher;
use std::{fs, io, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

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
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
            let temp = path.with_extension(format!("tmp-{}-{}", std::process::id(), nonce));
            fs::write(&temp, data)?;
            if let Err(error) = fs::rename(&temp, &path) {
                let _ = fs::remove_file(&temp);
                if !path.exists() { return Err(error); }
            }
        }
        Ok(BlobRef { handle: handle_for_hash(&hash), hash, bytes: data.len() as u64 })
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
                    if item.file_name().and_then(|name| name.to_str()).map(|name| handle_for_hash(name) == handle).unwrap_or(false) { matches.push(item); }
                }
            }
        }
        match matches.as_slice() { [path] => fs::read(path), [] => Err(io::Error::new(io::ErrorKind::NotFound, "handle expired or unknown")), _ => Err(invalid("ambiguous handle")) }
    }
}

fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }
fn handle_for_hash(hash: &str) -> String {
    let mut bytes = [0u8; 12];
    for (index, chunk) in hash.as_bytes().chunks(2).take(12).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("00"), 16).unwrap_or(0);
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
