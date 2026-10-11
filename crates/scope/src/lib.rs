use std::{fs::{self, OpenOptions}, io::{self, BufRead, BufReader, Write}, path::{Path, PathBuf}, sync::Mutex};

#[derive(Debug, Clone)]
pub struct Savings { pub module: String, pub raw_tokens: u32, pub presented_tokens: u32, pub measured: bool }

pub enum Event { Savings(Savings) }

pub struct EventLog { path: PathBuf, lock: Mutex<()> }

impl EventLog {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path, lock: Mutex::new(()) })
    }
    pub fn append(&self, event: Event) -> io::Result<()> {
        let _guard = self.lock.lock().map_err(|_| io::Error::other("event log lock poisoned"))?;
        let line = match event { Event::Savings(s) => format!("{{\"type\":\"savings\",\"module\":\"{}\",\"raw_tokens\":{},\"presented_tokens\":{},\"measured\":{}}}\n", escape(&s.module), s.raw_tokens, s.presented_tokens, s.measured) };
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write_all(line.as_bytes())?; file.sync_data()
    }
    pub fn read(&self) -> io::Result<Vec<String>> { BufReader::new(fs::File::open(&self.path)?).lines().collect() }
}

fn escape(value: &str) -> String { value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n") }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appends_and_replays_events() { let path = std::env::temp_dir().join(format!("conduit-scope-{}.jsonl", std::process::id())); let log = EventLog::open(&path).unwrap(); log.append(Event::Savings(Savings { module: "lens".into(), raw_tokens: 100, presented_tokens: 40, measured: false })).unwrap(); let lines = log.read().unwrap(); assert_eq!(lines.len(), 1); assert!(lines[0].contains("\"raw_tokens\":100")); let _ = fs::remove_file(path); }
}
