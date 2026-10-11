use conduit_scope::{Event, EventLog, Savings};
use conduit_store::BlobStore;
use conduit_lens::distill;
use conduit_atlas::Snapshot;
use conduit_edit::{apply, Operation};
use std::{env, fs, path::PathBuf, process};

fn usage() -> ! {
    eprintln!("usage: conduit <run|find|edit|store|expand|record|replay|stats|bench> ...");
    process::exit(2);
}

fn root() -> PathBuf {
    env::var_os("CONDUIT_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".conduit"))
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("run") => {
            let mut command = args.next().unwrap_or_else(|| usage());
            if command == "--" { command = args.next().unwrap_or_else(|| usage()); }
            let command_args: Vec<String> = args.collect();
            let output = std::process::Command::new(&command).args(&command_args).output().unwrap_or_else(|e| fail(e));
            let mut raw = output.stdout;
            raw.extend_from_slice(&output.stderr);
            let raw_text = String::from_utf8_lossy(&raw);
            let blob = BlobStore::open(root().join("blobs")).ok().and_then(|store| store.put(&raw).ok());
            let result = std::panic::catch_unwind(|| distill(&raw_text, 700));
            match result {
                Ok(filtered) => {
                    if filtered.dropped.is_empty() {
                        print!("{}", filtered.presented);
                    } else if let Some(blob) = blob {
                        print!("{}", filtered.presented);
                        println!("[conduit h={} shown={}/{}tok exit={} dropped='{}' expand='conduit expand {}']", blob.handle, filtered.presented_tokens, filtered.raw_tokens, output.status.code().unwrap_or(1), filtered.dropped.join(", "), blob.handle);
                    } else {
                        print!("{}", raw_text);
                    }
                }
                Err(_) => print!("{}", raw_text),
            }
            std::process::exit(output.status.code().unwrap_or(1));
        }
        Some("find") => {
            let query = args.next().unwrap_or_else(|| usage());
            let limit = args.next().and_then(|value| value.parse().ok()).unwrap_or(20);
            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let snapshot = Snapshot::build(workspace).unwrap_or_else(|e| fail(e));
            for hit in snapshot.search(&query, limit) { println!("{}:{}:{} [{}]", hit.path.display(), hit.line, hit.text, hit.hash); }
        }
        Some("edit") => {
            let path = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let expected_hash = args.next().unwrap_or_else(|| usage());
            let old = args.next().unwrap_or_else(|| usage());
            let new = args.next().unwrap_or_else(|| usage());
            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let applied = apply(workspace, &[Operation::SearchReplace { path, expected_hash, old, new }]).unwrap_or_else(|e| fail(e));
            for item in applied { println!("{} {}", item.path.display(), item.hash); }
        }
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
            let (raw, shown) = log.totals().unwrap_or_else(|e| fail(e));
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
