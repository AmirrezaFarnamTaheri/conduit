use conduit_scope::{Event, EventLog, Savings};
use conduit_store::BlobStore;
use std::{env, fs, path::PathBuf, process};

fn usage() -> ! {
    eprintln!("usage: conduit <store|expand|record|replay|stats|bench> ...");
    process::exit(2);
}

fn root() -> PathBuf {
    env::var_os("CONDUIT_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".conduit"))
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("store") => {
            let text = args.next().unwrap_or_else(|| usage());
            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|e| fail(e));
            let blob = store.put(text.as_bytes()).unwrap_or_else(|e| fail(e));
            println!("{}", blob.handle);
        }
        Some("expand") => {
            let handle = args.next().unwrap_or_else(|| usage());
            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|e| fail(e));
            let bytes = store.get(&handle).unwrap_or_else(|e| fail(e));
            print!("{}", String::from_utf8_lossy(&bytes));
        }
        Some("record") => {
            let session = args.next().unwrap_or_else(|| usage());
            let module = args.next().unwrap_or_else(|| usage());
            let raw: u32 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
            let presented: u32 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
            let log = EventLog::open(root().join("sessions").join(format!("{session}.jsonl"))).unwrap_or_else(|e| fail(e));
            log.append(Event::Savings(Savings { module, raw_tokens: raw, presented_tokens: presented, measured: false })).unwrap_or_else(|e| fail(e));
        }
        Some("replay") => {
            let session = args.next().unwrap_or_else(|| usage());
            let log = EventLog::open(root().join("sessions").join(format!("{session}.jsonl"))).unwrap_or_else(|e| fail(e));
            for event in log.read().unwrap_or_else(|e| fail(e)) { println!("{}", event); }
        }
        Some("stats") => {
            let session = args.next().unwrap_or_else(|| usage());
            let log = EventLog::open(root().join("sessions").join(format!("{session}.jsonl"))).unwrap_or_else(|e| fail(e));
            let mut raw = 0u64; let mut shown = 0u64;
            for line in log.read().unwrap_or_else(|e| fail(e)) {
                let nums: Vec<u64> = line.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
                if nums.len() >= 2 { raw += nums[nums.len()-2]; shown += nums[nums.len()-1]; }
            }
            println!("raw_tokens={raw} presented_tokens={shown} saved_tokens={}", raw.saturating_sub(shown));
        }
        Some("bench") => {
            let text = fs::read_to_string("eval/corpus/sample.log").unwrap_or_else(|e| fail(e));
            let shown = text.lines().filter(|line| line.contains("ERROR") || line.contains("FAILED") || line.contains("passed") || line.contains("exit=")).count();
            println!("corpus_lines={} retained_lines={} mode=deterministic", text.lines().count(), shown);
        }
        _ => usage(),
    }
}

fn fail(error: impl std::fmt::Display) -> ! { eprintln!("conduit: {error}"); process::exit(1) }
