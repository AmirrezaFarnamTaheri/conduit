//! Validated, bounded append-only session events. Readers reject malformed or
//! torn records. A damaged final fragment requires explicit recovery.
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Savings {
    pub module: String,
    pub raw_tokens: u32,
    pub presented_tokens: u32,
    pub measured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Savings(Savings),
}

impl fmt::Display for Event {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Savings(s) => write!(
                output,
                "savings module={} raw_tokens={} presented_tokens={} measured={}",
                s.module, s.raw_tokens, s.presented_tokens, s.measured
            ),
        }
    }
}

const MAX_LOG_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 8 * 1024;
const MAX_MODULE_BYTES: usize = 512;
static NEXT_TAIL: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TailRepair {
    Unchanged,
    CompletedRecord,
    Quarantined { backup: PathBuf },
}

fn quarantine_tail(path: &Path, fragment: &[u8]) -> io::Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| io::Error::other("session file has no parent"))?;
    for _ in 0..16 {
        let mut name = path.file_name().ok_or_else(|| io::Error::other("session file has no name"))?.to_os_string();
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let serial = NEXT_TAIL.fetch_add(1, Ordering::Relaxed);
        name.push(format!(".torn-{}-{nonce}-{serial}.bak", std::process::id()));
        let backup = parent.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let mut output = match options.open(&backup) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let write: io::Result<()> = (|| {
            output.write_all(fragment)?;
            output.sync_all()?;
            #[cfg(unix)]
            File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if let Err(error) = write {
            let _ = fs::remove_file(&backup);
            return Err(error);
        }
        return Ok(backup);
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "unable to allocate a unique session tail backup"))
}

pub struct EventLog {
    path: PathBuf,
}

impl EventLog {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let parent = path.parent().ok_or_else(|| io::Error::other("log path has no parent"))?;
        if !parent.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                fs::DirBuilder::new().recursive(true).mode(0o700).create(parent)?;
            }
            #[cfg(not(unix))]
            fs::create_dir_all(parent)?;
        }
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if !metadata.file_type().is_file() {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "session path is not a file"));
            }
        }
        let file = Self::open_file(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        Ok(Self { path })
    }

    fn open_file(path: &Path) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true).append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        options.open(path)
    }

    pub fn append(&self, event: Event) -> io::Result<()> {
        let Event::Savings(savings) = &event;
        if savings.module.is_empty() || savings.module.len() > MAX_MODULE_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "event module must contain 1-512 bytes"));
        }
        let line = serde_json::to_vec(&event).map_err(io::Error::other)?;
        if line.len() + 1 > MAX_RECORD_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "session event exceeds 8 KiB"));
        }
        let mut file = Self::open_file(&self.path)?;
        FileExt::lock_exclusive(&file)?;
        let result = (|| {
            let length = file.metadata()?.len();
            if length.saturating_add(line.len() as u64 + 1) > MAX_LOG_BYTES {
                return Err(io::Error::other("session log reached 64 MiB limit"));
            }
            // Do not append onto a torn record: that would permanently poison
            // valid JSONL and silently undercount previously recorded events.
            if length != 0 {
                file.seek(SeekFrom::End(-1))?;
                let mut tail = [0u8];
                file.read_exact(&mut tail)?;
                if tail[0] != b'\n' {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "session log has a torn final record; run repair-session"));
                }
            }
            file.write_all(&line)?;
            file.write_all(b"\n")?;
            file.sync_data()
        })();
        let unlocked = FileExt::unlock(&file);
        result.and(unlocked)
    }

    fn each_event(&self, mut observe: impl FnMut(Event) -> io::Result<()>) -> io::Result<()> {
        let file = Self::open_file(&self.path)?;
        FileExt::lock_shared(&file)?;
        let result = (|| {
            if file.metadata()?.len() > MAX_LOG_BYTES {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "session log exceeds 64 MiB limit"));
            }
            let mut reader = BufReader::new(&file);
            let mut record = Vec::new();
            let mut number = 0usize;
            loop {
                record.clear();
                let length = reader.read_until(b'\n', &mut record)?;
                if length == 0 { break; }
                number += 1;
                if length > MAX_RECORD_BYTES || record.last() != Some(&b'\n') {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                        format!("oversized or incomplete session record at line {number}")));
                }
                let event = serde_json::from_slice::<Event>(&record[..length - 1]).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidData,
                        format!("invalid event at line {number}: {error}"))
                })?;
                observe(event)?;
            }
            Ok(())
        })();
        let unlocked = FileExt::unlock(&file);
        result.and(unlocked)
    }

    pub fn events(&self) -> io::Result<Vec<Event>> {
        let mut events = Vec::new();
        self.each_event(|event| { events.push(event); Ok(()) })?;
        Ok(events)
    }

    /// Stream event totals without allocating a full history vector.
    pub fn totals(&self) -> io::Result<(u64, u64)> {
        let (mut raw, mut presented) = (0u64, 0u64);
        self.each_event(|event| {
            let Event::Savings(savings) = event;
            raw = raw.checked_add(u64::from(savings.raw_tokens)).ok_or_else(|| io::Error::other("raw total overflow"))?;
            presented = presented.checked_add(u64::from(savings.presented_tokens)).ok_or_else(|| io::Error::other("presented total overflow"))?;
            Ok(())
        })?;
        Ok((raw, presented))
    }

    pub fn read(&self) -> io::Result<Vec<String>> {
        let mut lines = Vec::new();
        self.each_event(|event| {
            lines.push(serde_json::to_string(&event).map_err(io::Error::other)?);
            Ok(())
        })?;
        Ok(lines)
    }

    /// Preserve complete events and quarantine incomplete tails before truncation.
    pub fn repair_tail(&self) -> io::Result<TailRepair> {
        let mut file = Self::open_file(&self.path)?;
        FileExt::lock_exclusive(&file)?;
        let result = (|| {
            let length = file.metadata()?.len();
            if length > MAX_LOG_BYTES {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "session log exceeds 64 MiB limit"));
            }
            file.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            if bytes.is_empty() || bytes.last() == Some(&b'\n') { return Ok(TailRepair::Unchanged); }
            let prefix_end = bytes.iter().rposition(|byte| *byte == b'\n')
                .map_or(0, |index| index + 1);
            if prefix_end != 0 {
                for (index, line) in bytes[..prefix_end - 1].split(|byte| *byte == b'\n').enumerate() {
                    if line.len() + 1 > MAX_RECORD_BYTES || serde_json::from_slice::<Event>(line).is_err() {
                        return Err(io::Error::new(io::ErrorKind::InvalidData,
                            format!("refusing to modify tail because preceding record {} is invalid", index + 1)));
                    }
                }
            }
            let tail = &bytes[prefix_end..];
            if tail.len() + 1 <= MAX_RECORD_BYTES && serde_json::from_slice::<Event>(tail).is_ok() {
                if length == MAX_LOG_BYTES {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "cannot append record delimiter beyond log limit"));
                }
                file.write_all(b"\n")?;
                file.sync_all()?;
                return Ok(TailRepair::CompletedRecord);
            }
            let backup = quarantine_tail(&self.path, tail)?;
            file.set_len(prefix_end as u64)?;
            file.sync_all()?;
            Ok(TailRepair::Quarantined { backup })
        })();
        let unlocked = FileExt::unlock(&file);
        unlocked?;
        result
    }

    pub fn repair_partial_tail(&self) -> io::Result<bool> {
        Ok(!matches!(self.repair_tail()?, TailRepair::Unchanged))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "conduit-scope-{label}-{}-{}.jsonl",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[cfg(unix)]
    #[test]
    fn opening_log_does_not_change_existing_parent_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let parent = std::env::temp_dir();
        let before = fs::metadata(&parent).unwrap().permissions().mode();
        let file = path("shared-parent");
        let _log = EventLog::open(&file).unwrap();
        assert_eq!(fs::metadata(&parent).unwrap().permissions().mode(), before);
        let _ = fs::remove_file(file);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_session_symlink_replaced_after_open() {
        use std::os::unix::fs::symlink;
        let session = path("symlink");
        let log = EventLog::open(&session).unwrap();
        let other = path("symlink-target");
        fs::rename(&session, &other).unwrap();
        symlink(&other, &session).unwrap();
        assert!(log.append(Event::Savings(Savings {
            module: "test".into(), raw_tokens: 1, presented_tokens: 1, measured: false,
        })).is_err());
        let _ = fs::remove_file(session);
        let _ = fs::remove_file(other);
    }

    #[test]
    fn sums_typed_events() {
        let path = path("totals");
        let log = EventLog::open(&path).unwrap();
        log.append(Event::Savings(Savings { module: "lens".into(), raw_tokens: 100, presented_tokens: 40, measured: true })).unwrap();
        log.append(Event::Savings(Savings { module: "atlas".into(), raw_tokens: 20, presented_tokens: 20, measured: false })).unwrap();
        assert_eq!(log.totals().unwrap(), (120, 60));
        assert_eq!(log.events().unwrap().len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn preserves_modules_with_json_metacharacters() {
        let path = path("escape");
        let log = EventLog::open(&path).unwrap();
        let event = Event::Savings(Savings { module: "a\"b\n\\\tc".into(), raw_tokens: 1, presented_tokens: 1, measured: false });
        log.append(event.clone()).unwrap();
        assert_eq!(log.events().unwrap(), vec![event]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_malformed_or_partial_events_instead_of_silent_undercount() {
        let path = path("malformed");
        let log = EventLog::open(&path).unwrap();
        fs::write(&path, b"{\"type\":\"savings\",\"raw_tokens\":4}\n").unwrap();
        assert_eq!(log.totals().unwrap_err().kind(), io::ErrorKind::InvalidData);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn independent_instances_serialize_concurrent_appends() {
        let path = path("concurrent");
        EventLog::open(&path).unwrap();
        let writers: Vec<_> = (0..4).map(|_| {
            let path = path.clone();
            std::thread::spawn(move || {
                let log = EventLog::open(path).unwrap();
                for _ in 0..25 {
                    log.append(Event::Savings(Savings { module: "test".into(), raw_tokens: 2, presented_tokens: 1, measured: false })).unwrap();
                }
            })
        }).collect();
        for writer in writers { writer.join().unwrap(); }
        assert_eq!(EventLog::open(&path).unwrap().totals().unwrap(), (200, 100));
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn torn_tail_blocks_appends_and_requires_explicit_repair() {
        let path = std::env::temp_dir().join(format!("conduit-torn-session-{}.jsonl", std::process::id()));
        let log = EventLog::open(&path).unwrap();
        let event = Event::Savings(Savings { module: "source".into(), raw_tokens: 2, presented_tokens: 1, measured: false });
        log.append(event.clone()).unwrap();
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"type\":\"sav").unwrap();
        assert_eq!(log.totals().unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(log.append(event.clone()).unwrap_err().kind(), io::ErrorKind::InvalidData);
        let backup = match log.repair_tail().unwrap() {
            TailRepair::Quarantined { backup } => backup,
            other => panic!("expected quarantined tail, got {other:?}"),
        };
        assert_eq!(fs::read(&backup).unwrap(), b"{\"type\":\"sav");
        assert!(!log.repair_partial_tail().unwrap());
        log.append(event).unwrap();
        assert_eq!(log.totals().unwrap(), (4, 2));
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(backup);
    }

    #[test]
    fn valid_unterminated_event_is_preserved() {
        let path = std::env::temp_dir().join(format!("conduit-tail-valid-{}.jsonl", std::process::id()));
        let log = EventLog::open(&path).unwrap();
        let event = Event::Savings(Savings { module: "source".into(), raw_tokens: 11, presented_tokens: 7, measured: false });
        fs::write(&path, serde_json::to_vec(&event).unwrap()).unwrap();
        assert_eq!(log.repair_tail().unwrap(), TailRepair::CompletedRecord);
        assert_eq!(log.totals().unwrap(), (11, 7));
        assert_eq!(log.repair_tail().unwrap(), TailRepair::Unchanged);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_complete_record_is_not_silently_repaired() {
        let path = std::env::temp_dir().join(format!("conduit-corrupt-session-{}.jsonl", std::process::id()));
        let log = EventLog::open(&path).unwrap();
        fs::write(&path, b"not-json\ntruncated").unwrap();
        assert!(log.repair_partial_tail().is_err());
        assert!(log.totals().is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_large_or_empty_event_module() {
        let path = std::env::temp_dir().join(format!("conduit-module-session-{}.jsonl", std::process::id()));
        let log = EventLog::open(&path).unwrap();
        for module in ["".to_owned(), "x".repeat(MAX_MODULE_BYTES + 1)] {
            assert!(log.append(Event::Savings(Savings {
                module, raw_tokens: 1, presented_tokens: 1, measured: false,
            })).is_err());
        }
        assert_eq!(log.totals().unwrap(), (0, 0));
        let _ = fs::remove_file(path);
    }
}
