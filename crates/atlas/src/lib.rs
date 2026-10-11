//! Ignore-aware, bounded text search snapshots. This is not yet an AST index.
use blake3::Hasher;
use ignore::WalkBuilder;
use std::{fs, io::{self, Read}, path::{Path, PathBuf}};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_INDEX_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    files: Vec<(PathBuf, String, String)>,
    skipped: usize,
    truncated: bool,
}

impl Snapshot {
    pub fn build(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().canonicalize()?;
        let mut files = Vec::new();
        let mut skipped = 0usize;
        let mut truncated = false;
        let mut retained_bytes = 0usize;
        let walker = build_walker(&root);

        for item in walker {
            let entry = match item {
                Ok(entry) => entry,
                Err(_) => { skipped += 1; continue; }
            };
            let Some(kind) = entry.file_type() else { continue };
            if !kind.is_file() { continue; }
            let path = entry.path();
            let metadata = match fs::metadata(path) {
                Ok(metadata) => metadata,
                Err(_) => { skipped += 1; continue; }
            };
            if metadata.len() > MAX_FILE_BYTES { skipped += 1; continue; }
            if files.len() >= MAX_FILES || retained_bytes.saturating_add(metadata.len() as usize) > MAX_INDEX_BYTES {
                truncated = true;
                break;
            }
            let bytes = match read_bounded(path, MAX_FILE_BYTES as usize) {
                Ok(bytes) => bytes,
                Err(_) => { skipped += 1; continue; }
            };
            if bytes.len() > MAX_FILE_BYTES as usize || looks_binary(&bytes) {
                skipped += 1;
                continue;
            }
            let text = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => { skipped += 1; continue; }
            };
            retained_bytes += text.len();
            let mut hasher = Hasher::new();
            hasher.update(text.as_bytes());
            let relative = path.strip_prefix(&root).unwrap_or(path).to_path_buf();
            files.push((relative, text, hasher.finalize().to_hex().to_string()));
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(Self { files, skipped, truncated })
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        if query.is_empty() || limit == 0 { return Vec::new(); }
        let needle = query.to_lowercase();
        let mut hits = Vec::new();
        for (path, contents, hash) in &self.files {
            for (index, line) in contents.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    hits.push(Hit {
                        path: path.clone(),
                        line: index + 1,
                        text: line.to_owned(),
                        hash: hash.clone(),
                    });
                    if hits.len() == limit { return hits; }
                }
            }
        }
        hits
    }

    pub fn files(&self) -> usize { self.files.len() }
    pub fn skipped(&self) -> usize { self.skipped }
    pub fn truncated(&self) -> bool { self.truncated }
}

/// File-by-file bounded search without materializing the repository contents.
/// File iteration is filesystem-dependent; results are not a ranking signal.
/// `limit_reached` means results may remain even when `truncated` is false.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub hits: Vec<Hit>,
    pub scanned_files: usize,
    pub scanned_bytes: usize,
    pub skipped: usize,
    pub truncated: bool,
    pub limit_reached: bool,
}

pub fn find_streaming(root: impl AsRef<Path>, query: &str, limit: usize) -> io::Result<SearchResult> {
    if query.trim().is_empty() || query.len() > 512 || !(1..=500).contains(&limit) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "search requires a nonempty query <=512 bytes and 1-500 hits"));
    }
    let root = root.as_ref().canonicalize()?;
    let needle = query.to_lowercase();
    let mut result = SearchResult {
        hits: Vec::new(), scanned_files: 0, scanned_bytes: 0,
        skipped: 0, truncated: false, limit_reached: false,
    };
    'entries: for entry in build_walker(&root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => { result.skipped += 1; continue; }
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) { continue; }
        let metadata = match fs::metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(_) => { result.skipped += 1; continue; }
        };
        if metadata.len() > MAX_FILE_BYTES { result.skipped += 1; continue; }
        if result.scanned_files >= MAX_FILES ||
            result.scanned_bytes.saturating_add(metadata.len() as usize) > MAX_INDEX_BYTES
        {
            result.truncated = true;
            break;
        }
        let bytes = match read_bounded(entry.path(), MAX_FILE_BYTES as usize) {
            Ok(bytes) => bytes,
            Err(_) => { result.skipped += 1; continue; }
        };
        result.scanned_files += 1;
        result.scanned_bytes += bytes.len();
        if looks_binary(&bytes) { result.skipped += 1; continue; }
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => { result.skipped += 1; continue; }
        };
        let hash = {
            let mut hasher = Hasher::new();
            hasher.update(text.as_bytes());
            hasher.finalize().to_hex().to_string()
        };
        let relative = entry.path().strip_prefix(&root).unwrap_or(entry.path()).to_path_buf();
        for (index, line) in text.lines().enumerate() {
            let lowercase = line.to_lowercase();
            let Some(match_byte_offset) = lowercase.find(&needle) else { continue };
            // A hit is a navigation aid, not a second copy of a megabyte line.
            // Map a Unicode-lowercased match back to source character offsets;
            // lowercase transformations can expand one source character.
            let mut lowered_bytes = 0usize;
            let mut match_chars = 0usize;
            for character in line.chars() {
                if lowered_bytes >= match_byte_offset { break; }
                lowered_bytes += character.to_lowercase().map(char::len_utf8).sum::<usize>();
                match_chars += 1;
            }
            let start = match_chars.saturating_sub(80);
            let excerpt_body: String = line.chars().skip(start).take(240).collect();
            let end = start + excerpt_body.chars().count();
            let excerpt = format!("{}{}{}",
                if start > 0 { "..." } else { "" },
                excerpt_body,
                if line.chars().count() > end { "..." } else { "" },
            );
            result.hits.push(Hit {
                path: relative.clone(), line: index + 1,
                text: excerpt, hash: hash.clone(),
            });
            if result.hits.len() == limit {
                result.limit_reached = true;
                break 'entries;
            }
        }
    }
    Ok(result)
}

fn build_walker(root: &Path) -> ignore::Walk {
    WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .filter_entry(|entry| {
            if entry.depth() == 0 { return true; }
            let name = entry.file_name().to_string_lossy();
            !matches!(name.as_ref(),
                ".git" | ".conduit" | ".hg" | ".svn" | ".venv" | "venv" |
                "node_modules" | "target" | "dist" | "build" | "__pycache__" |
                ".next" | ".cache" | ".ssh" | ".aws" | ".secrets" | "secrets") &&
            !name.starts_with(".env") && !name.starts_with(".conduit-edit-")
        })
        .build()
}

fn read_bounded(path: &Path, max_bytes: usize) -> io::Result<Vec<u8>> {
    let mut input = fs::File::open(path)?.take(max_bytes as u64 + 1);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file grew beyond indexing limit"));
    }
    Ok(bytes)
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|byte| *byte == 0 || (*byte < 0x09) || (*byte > 0x0d && *byte < 0x20))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "conduit-atlas-{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn returns_stable_hashed_hits() {
        let root = temp_root("search");
        fs::write(root.join("a.rs"), "fn target() {}\n").unwrap();
        let snapshot = Snapshot::build(&root).unwrap();
        let hits = snapshot.search("TARGET", 10);
        assert_eq!(snapshot.files(), 1);
        assert_eq!(hits[0].path, PathBuf::from("a.rs"));
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[0].hash.len(), 64);
        assert!(snapshot.search("", 1).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn respects_gitignore_and_excludes_secrets_and_binary_files() {
        let root = temp_root("ignores");
        fs::write(root.join(".gitignore"), "*.generated\n").unwrap();
        fs::write(root.join("safe.rs"), "needle\n").unwrap();
        fs::write(root.join("secret.generated"), "needle\n").unwrap();
        fs::write(root.join(".env"), "needle\n").unwrap();
        fs::write(root.join("binary"), b"needle\0more").unwrap();
        let snapshot = Snapshot::build(&root).unwrap();
        let hits = snapshot.search("needle", 100);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, PathBuf::from("safe.rs"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_utf8_is_skipped_not_a_global_search_failure() {
        let root = temp_root("nonutf8");
        fs::write(root.join("invalid.txt"), [0xff, 0xfe]).unwrap();
        fs::write(root.join("ok.txt"), "needle\n").unwrap();
        let snapshot = Snapshot::build(&root).unwrap();
        assert_eq!(snapshot.search("needle", 10).len(), 1);
        assert!(snapshot.skipped() >= 1);
        let _ = fs::remove_dir_all(root);
    }
}

/// Bounded, hash-anchored read for the search -> inspect -> edit workflow.
/// Explicit reads are independent of whole-repository snapshot construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peek {
    pub hash: String,
    pub total_lines: usize,
    pub lines: Vec<(usize, String)>,
    pub truncated: bool,
}

pub fn peek(root: impl AsRef<Path>, relative: impl AsRef<Path>, start: usize, count: usize) -> io::Result<Peek> {
    use std::path::Component;
    if start == 0 || count == 0 || count > 200 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "peek requires a 1-based start and 1-200 lines"));
    }
    let relative = relative.as_ref();
    if relative.as_os_str().is_empty() || relative.is_absolute() || relative.components().any(|part| matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peek path escapes workspace"));
    }
    let root = root.as_ref().canonicalize()?;
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(&root) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peek path escapes workspace via symlink"));
    }
    let logical = path.strip_prefix(&root).unwrap_or(&path);
    for component in relative.components().chain(logical.components()) {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), ".git" | ".conduit" | ".ssh" | ".aws" | ".secrets" | "secrets") || name.starts_with(".env") {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peek of sensitive path is disabled"));
            }
        }
    }
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "peek supports regular files up to 1 MiB"));
    }
    let bytes = read_bounded(&path, MAX_FILE_BYTES as usize)?;
    if looks_binary(&bytes) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "peek requires a bounded text file"));
    }
    let text = String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "peek requires UTF-8"))?;
    let mut hasher = Hasher::new();
    hasher.update(text.as_bytes());
    let all: Vec<&str> = text.lines().collect();
    let end = start.saturating_add(count).saturating_sub(1).min(all.len());
    let selected = all.iter().enumerate().skip(start - 1).take(count).map(|(index, line)| (index + 1, (*line).to_owned())).collect();
    Ok(Peek {
        hash: hasher.finalize().to_hex().to_string(),
        total_lines: all.len(),
        lines: selected,
        truncated: end < all.len(),
    })
}

#[cfg(test)]
mod peek_tests {
    use super::*;

    #[test]
    fn bounded_peek_includes_stable_hash_and_line_numbers() {
        let root = std::env::temp_dir().join(format!("conduit-peek-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("file.rs"), "one\ntwo\nthree\n").unwrap();
        let result = peek(&root, "file.rs", 2, 1).unwrap();
        assert_eq!(result.lines, vec![(2, "two".to_owned())]);
        assert_eq!(result.total_lines, 3);
        assert!(result.truncated);
        assert_eq!(result.hash.len(), 64);
        assert!(peek(&root, ".env", 1, 10).is_err());
        assert!(peek(&root, "../outside", 1, 10).is_err());
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "conduit-streaming-{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn streaming_find_respects_ignores_and_returns_edit_hash() {
        let dir = root("ignores");
        fs::write(dir.join(".gitignore"), "*.generated\n").unwrap();
        fs::write(dir.join("a.rs"), "first\nTARGET here\n").unwrap();
        fs::write(dir.join("b.generated"), "TARGET\n").unwrap();
        fs::write(dir.join(".env"), "TARGET\n").unwrap();
        let found = find_streaming(&dir, "target", 4).unwrap();
        assert_eq!(found.hits.len(), 1);
        assert_eq!(found.hits[0].path, PathBuf::from("a.rs"));
        assert_eq!(found.hits[0].line, 2);
        assert_eq!(found.hits[0].hash, peek(&dir, "a.rs", 1, 2).unwrap().hash);
        assert!(!found.limit_reached);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn streaming_find_bounds_hit_count_and_excerpt() {
        let dir = root("bounded");
        let long = format!("{}needle\nneedle{}\n", "x".repeat(900), "y".repeat(900));
        fs::write(dir.join("a.txt"), long).unwrap();
        let found = find_streaming(&dir, "needle", 1).unwrap();
        assert!(found.limit_reached);
        assert_eq!(found.hits.len(), 1);
        assert!(found.hits[0].text.chars().count() <= 246);
        assert!(found.hits[0].text.contains("needle"));
        for bad in ["", " ", "x"] {
            if bad.trim().is_empty() { assert!(find_streaming(&dir, bad, 1).is_err()); }
        }
        assert!(find_streaming(&dir, "needle", 0).is_err());
        let _ = fs::remove_dir_all(dir);
    }
}
