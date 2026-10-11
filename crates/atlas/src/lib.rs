use blake3::Hasher;
use std::{fs, io, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit { pub path: PathBuf, pub line: usize, pub text: String, pub hash: String }

#[derive(Debug, Clone)]
pub struct Snapshot { files: Vec<(PathBuf, String, String)> }

impl Snapshot {
    pub fn build(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut files = Vec::new();
        collect(&root, &root, &mut files)?;
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(Self { files })
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        if query.is_empty() || limit == 0 { return Vec::new(); }
        let needle = query.to_ascii_lowercase();
        let mut hits = Vec::new();
        for (path, contents, hash) in &self.files {
            for (index, line) in contents.lines().enumerate() {
                if line.to_ascii_lowercase().contains(&needle) {
                    hits.push(Hit { path: path.clone(), line: index + 1, text: line.to_owned(), hash: hash.clone() });
                    if hits.len() == limit { return hits; }
                }
            }
        }
        hits
    }

    pub fn files(&self) -> usize { self.files.len() }
}

fn collect(root: &Path, directory: &Path, output: &mut Vec<(PathBuf, String, String)>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() { continue; }
        let path = entry.path();
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        if name == ".git" || name == ".conduit" || name == "target" { continue; }
        if path.is_dir() { collect(root, &path, output)?; continue; }
        if !path.is_file() || fs::metadata(&path)?.len() > 2 * 1024 * 1024 { continue; }
        let bytes = fs::read(&path)?;
        if bytes.contains(&0) { continue; }
        let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-utf8 file"))?;
        let mut hasher = Hasher::new(); hasher.update(text.as_bytes());
        output.push((path.strip_prefix(root).unwrap_or(&path).to_path_buf(), text, hasher.finalize().to_hex().to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_returns_stable_hashed_hits() {
        let root = std::env::temp_dir().join(format!("conduit-atlas-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap(); fs::write(root.join("a.rs"), "fn target() {}\n").unwrap();
        let snapshot = Snapshot::build(&root).unwrap(); let hits = snapshot.search("TARGET", 10);
        assert_eq!(snapshot.files(), 1); assert_eq!(hits[0].path, PathBuf::from("a.rs")); assert_eq!(hits[0].line, 1); assert_eq!(hits[0].hash.len(), 64);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn limit_and_empty_queries_are_deterministic() { let root = std::env::temp_dir().join(format!("conduit-atlas-empty-{}", std::process::id())); fs::create_dir_all(&root).unwrap(); fs::write(root.join("a"), "x\nx\n").unwrap(); let s = Snapshot::build(&root).unwrap(); assert!(s.search("", 1).is_empty()); assert_eq!(s.search("x", 1).len(), 1); let _ = fs::remove_dir_all(root); }
}
