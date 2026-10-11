mod run;

use conduit_atlas::{find_streaming, peek};
use conduit_edit::{apply, preview, Operation};
use conduit_lens::distill;
use conduit_scope::{Event, EventLog, Savings, TailRepair};
use conduit_store::BlobStore;
use run::{Capture, RunOptions, RunResult};
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn usage() -> ! {
    eprintln!("usage: conduit run [--preset safe|quick|deep|compact] [--timeout-ms N] [--max-output-bytes N] [--budget N] [--compact] -- PROGRAM [ARGS...]\n       conduit <find|peek|edit|batch-edit|store|expand|doctor|record|replay|stats|repair-session|bench> ...");
    process::exit(2);
}

fn root() -> PathBuf {
    env::var_os("CONDUIT_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".conduit"))
}

fn session_log(session: &str) -> io::Result<EventLog> {
    if session.is_empty() || session.len() > 64 || !session.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "session IDs must contain 1-64 ASCII letters, digits, _ or -"));
    }
    EventLog::open(root().join("sessions").join(format!("{session}.jsonl")))
}

#[derive(Debug, Clone, Copy)]
struct RunConfig {
    timeout_ms: u64,
    max_output_bytes: usize,
    budget: usize,
    compact: bool,
}

fn preset(name: &str) -> Option<RunConfig> {
    match name {
        "safe" => Some(RunConfig { timeout_ms: 120_000, max_output_bytes: 8 * 1024 * 1024, budget: 700, compact: false }),
        "quick" => Some(RunConfig { timeout_ms: 30_000, max_output_bytes: 4 * 1024 * 1024, budget: 500, compact: false }),
        "deep" => Some(RunConfig { timeout_ms: 600_000, max_output_bytes: 32 * 1024 * 1024, budget: 1_500, compact: false }),
        "compact" => Some(RunConfig { timeout_ms: 120_000, max_output_bytes: 8 * 1024 * 1024, budget: 700, compact: true }),
        _ => None,
    }
}

fn parse_run(mut args: impl Iterator<Item = String>) -> (RunConfig, String, Vec<String>) {
    let mut chosen_preset = "safe".to_owned();
    let (mut timeout, mut max_output, mut budget) = (None, None, None);
    let mut compact = false;
    let mut program = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--" => { program = args.next(); break; }
            "--preset" => chosen_preset = args.next().unwrap_or_else(|| usage()),
            "--timeout-ms" => {
                timeout = Some(args.next().and_then(|arg| arg.parse().ok()).unwrap_or_else(|| usage()));
            }
            "--max-output-bytes" => {
                max_output = Some(args.next().and_then(|arg| arg.parse().ok()).unwrap_or_else(|| usage()));
            }
            "--budget" => {
                budget = Some(args.next().and_then(|arg| arg.parse().ok()).unwrap_or_else(|| usage()));
            }
            "--compact" => compact = true,
            flag if flag.starts_with("--") => usage(),
            command => { program = Some(command.to_owned()); break; }
        }
    }
    let mut config = preset(&chosen_preset).unwrap_or_else(|| usage());
    // Explicit flags win regardless of whether --preset comes before or after.
    if let Some(value) = timeout { config.timeout_ms = value; }
    if let Some(value) = max_output { config.max_output_bytes = value; }
    if let Some(value) = budget { config.budget = value; }
    if compact { config.compact = true; }
    if config.timeout_ms == 0 || config.timeout_ms > 3_600_000 || config.max_output_bytes == 0 || config.max_output_bytes > 64 * 1024 * 1024 || config.budget == 0 || config.budget > 100_000 {
        usage();
    }
    (config, program.unwrap_or_else(|| usage()), args.collect())
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchManifest {
    operations: Vec<Operation>,
}

fn read_manifest(path: &std::path::Path) -> io::Result<BatchManifest> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "batch manifest exceeds 1 MiB"));
    }
    serde_json::from_slice(&bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn no_extra_args(args: &mut impl Iterator<Item = String>) {
    if args.next().is_some() { usage(); }
}

fn read_store_input(input: impl Read, max_bytes: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    input.take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "store input exceeds 64 MiB"));
    }
    Ok(bytes)
}

fn run_command(args: impl Iterator<Item = String>) -> ! {
    let (config, program, program_args) = parse_run(args);
    let result = run::run(&program, &program_args, RunOptions {
        timeout: Duration::from_millis(config.timeout_ms),
        max_stream_bytes: config.max_output_bytes,
    }).unwrap_or_else(|error| fail(error));
    let (printed_stdout, saved) = render_stdout(&result, config, &root());
    io::stdout().write_all(&printed_stdout).unwrap_or_else(|error| fail(error));
    io::stderr().write_all(&result.stderr.bytes).unwrap_or_else(|error| fail(error));
    if let Some((handle, raw_words, shown_words)) = saved {
        eprintln!("[conduit raw_stdout={} raw_words={} effective_shown_words={} expand='conduit expand {}']", handle, raw_words, shown_words, handle);
        if let Ok(session) = env::var("CONDUIT_SESSION") {
            if let Ok(log) = session_log(&session) {
                // Whitespace counts are not tokenizer-accurate usage metrics.
                let event = Event::Savings(Savings {
                    module: "lens-words".into(),
                    raw_tokens: raw_words.min(u32::MAX as usize) as u32,
                    presented_tokens: shown_words.min(u32::MAX as usize) as u32,
                    measured: false,
                });
                if let Err(error) = log.append(event) { eprintln!("conduit: unable to record session event: {error}"); }
            }
        }
    }
    let output_overflow = result.stdout.truncated || result.stderr.truncated;
    if output_overflow {
        eprintln!("conduit: output capture limit exceeded (stdout={} stderr={} bytes); output is INCOMPLETE and no lossless expansion is available", result.stdout.total_bytes, result.stderr.total_bytes);
    }
    if result.timed_out {
        eprintln!("conduit: direct child timed out after {} ms; descendant processes may survive", config.timeout_ms);
    }
    if env::var_os("CONDUIT_DEBUG_TIMING").is_some() {
        eprintln!("conduit: child_elapsed_ms={}", result.elapsed.as_millis());
    }
    if result.timed_out { process::exit(124); }
    if output_overflow { process::exit(75); }
    process::exit(result.exit_code);
}

fn render_stdout(result: &RunResult, config: RunConfig, home: &std::path::Path) -> (Vec<u8>, Option<(String, usize, usize)>) {
    if !config.compact || result.exit_code != 0 || result.timed_out || result.stdout.truncated || result.stderr.truncated {
        return (result.stdout.bytes.clone(), None);
    }
    let Ok(text) = std::str::from_utf8(&result.stdout.bytes) else {
        return (result.stdout.bytes.clone(), None);
    };
    if text.contains('\0') {
        return (result.stdout.bytes.clone(), None);
    }
    let filtered = distill(text, config.budget);
    if filtered.dropped.is_empty() || filtered.presented.len() >= text.len() {
        return (result.stdout.bytes.clone(), None);
    }
    // Compute the actual hint overhead before writing a blob. A small
    // reduction that only moves words into its recovery hint is a regression.
    let simulated_handle = "cx:o:aaaaaaaaaaaaaaaaaaaa";
    let hint = |shown_words| format!("[conduit raw_stdout={} raw_words={} effective_shown_words={} expand='conduit expand {}']", simulated_handle, filtered.raw_tokens, shown_words, simulated_handle);
    let presented_words = filtered.presented_tokens + hint(0).split_whitespace().count();
    if presented_words >= filtered.raw_tokens || filtered.presented.len() + hint(presented_words).len() >= text.len() {
        return (result.stdout.bytes.clone(), None);
    }
    let Ok(store) = BlobStore::open(home.join("blobs")) else {
        return (result.stdout.bytes.clone(), None);
    };
    let Ok(blob) = store.put(&result.stdout.bytes) else {
        return (result.stdout.bytes.clone(), None);
    };
    (filtered.presented.into_bytes(), Some((blob.handle, filtered.raw_tokens, presented_words)))
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("run") => run_command(args),
        Some("find") => {
            let query = args.next().unwrap_or_else(|| usage());
            let limit = args.next().map(|value| value.parse::<usize>().unwrap_or_else(|_| usage())).unwrap_or(20);
            if !(1..=500).contains(&limit) { usage(); }
            no_extra_args(&mut args);
            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let result = find_streaming(workspace, &query, limit).unwrap_or_else(|error| fail(error));
            for hit in &result.hits {
                let preview: String = hit.text.chars().take(240).collect();
                let cropped = hit.text.chars().count() > 240;
                println!("{}:{}:{}{} [{}]", hit.path.display(), hit.line, preview, if cropped { " [line clipped; use peek]" } else { "" }, hit.hash);
            }
            if result.skipped != 0 || result.truncated || result.limit_reached {
                eprintln!("conduit: search scanned_files={} scanned_bytes={} skipped={} truncated={} hit_limit_reached={}",
                    result.scanned_files, result.scanned_bytes, result.skipped, result.truncated, result.limit_reached);
            }
        }
        Some("peek") => {
            let relative = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let start = args.next().map(|value| value.parse::<usize>().unwrap_or_else(|_| usage())).unwrap_or(1);
            let count = args.next().map(|value| value.parse::<usize>().unwrap_or_else(|_| usage())).unwrap_or(80);
            no_extra_args(&mut args);
            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let excerpt = peek(workspace, &relative, start, count).unwrap_or_else(|error| fail(error));
            println!("file={} hash={} lines={} truncated={}", relative.display(), excerpt.hash, excerpt.total_lines, excerpt.truncated);
            for (line, text) in excerpt.lines { println!("{line}: {text}"); }
        }
        Some("edit") => {
            let path = PathBuf::from(args.next().unwrap_or_else(|| usage()));
            let expected_hash = args.next().unwrap_or_else(|| usage());
            let old = args.next().unwrap_or_else(|| usage());
            let new = args.next().unwrap_or_else(|| usage());
            no_extra_args(&mut args);
            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let applied = apply(workspace, &[Operation::SearchReplace { path, expected_hash, old, new }]).unwrap_or_else(|error| fail(error));
            for item in applied { println!("{} {}", item.path.display(), item.hash); }
        }
        Some("batch-edit") => {
            let first = args.next().unwrap_or_else(|| usage());
            let (check, manifest_path) = if first == "--check" {
                (true, PathBuf::from(args.next().unwrap_or_else(|| usage())))
            } else {
                (false, PathBuf::from(first))
            };
            no_extra_args(&mut args);
            let manifest = read_manifest(&manifest_path).unwrap_or_else(|error| fail(error));
            let workspace = env::var_os("CONDUIT_WORKSPACE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let results = if check { preview(workspace, &manifest.operations) } else { apply(workspace, &manifest.operations) }
                .unwrap_or_else(|error| fail(error));
            for item in results {
                println!("{}={} hash={}", if check { "validated" } else { "applied" }, item.path.display(), item.hash);
            }
        }
        Some("store") => {
            let argument = args.next().unwrap_or_else(|| usage());
            no_extra_args(&mut args);
            let bytes = if argument == "-" {
                read_store_input(io::stdin().lock(), 64 * 1024 * 1024)
                    .unwrap_or_else(|error| fail(error))
            } else {
                argument.into_bytes()
            };
            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
            let blob = store.put(&bytes).unwrap_or_else(|error| fail(error));
            println!("{}", blob.handle);
        }
        Some("expand") => {
            let handle = args.next().unwrap_or_else(|| usage());
            no_extra_args(&mut args);
            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
            let bytes = store.get(&handle).unwrap_or_else(|error| fail(error));
            io::stdout().write_all(&bytes).unwrap_or_else(|error| fail(error));
        }
        Some("doctor") => {
            no_extra_args(&mut args);
            let store = BlobStore::open(root().join("blobs")).unwrap_or_else(|error| fail(error));
            let report = store.audit().unwrap_or_else(|error| fail(error));
            println!("verified_blobs={} verified_bytes={} staged_blobs={} integrity_problems={}",
                report.checked_blobs, report.verified_bytes, report.staging_files, report.problem_count);
            for problem in report.problems { eprintln!("conduit: {problem}"); }
            if report.problem_count > 0 { process::exit(1); }
        }
        Some("record") => {
            let session = args.next().unwrap_or_else(|| usage());
            let module = args.next().unwrap_or_else(|| usage());
            let raw: u32 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
            let presented: u32 = args.next().unwrap_or_else(|| usage()).parse().unwrap_or_else(|_| usage());
            no_extra_args(&mut args);
            let log = session_log(&session).unwrap_or_else(|error| fail(error));
            log.append(Event::Savings(Savings { module, raw_tokens: raw, presented_tokens: presented, measured: false })).unwrap_or_else(|error| fail(error));
        }
        Some("replay") => {
            let session = args.next().unwrap_or_else(|| usage());
            no_extra_args(&mut args);
            let log = session_log(&session).unwrap_or_else(|error| fail(error));
            for event in log.events().unwrap_or_else(|error| fail(error)) { println!("{event}"); }
        }
        Some("repair-session") => {
            let session = args.next().unwrap_or_else(|| usage());
            no_extra_args(&mut args);
            let log = session_log(&session).unwrap_or_else(|error| fail(error));
            match log.repair_tail().unwrap_or_else(|error| fail(error)) {
                TailRepair::Unchanged => println!("partial_tail_removed=false action=unchanged"),
                TailRepair::CompletedRecord => println!("partial_tail_removed=false action=completed_record"),
                TailRepair::Quarantined { backup } => println!("partial_tail_removed=true action=quarantined backup={}", backup.display()),
            }
        }
        Some("stats") => {
            let session = args.next().unwrap_or_else(|| usage());
            no_extra_args(&mut args);
            let log = session_log(&session).unwrap_or_else(|error| fail(error));
            let (raw, shown) = log.totals().unwrap_or_else(|error| fail(error));
            println!("raw_units={raw} presented_units={shown} net_saved_units={}", i128::from(raw) - i128::from(shown));
            println!("note=word_based_or_caller_supplied_not_model_token_measurement");
        }
        Some("bench") => {
            let path = args.next().unwrap_or_else(|| "eval/corpus/sample.log".into());
            let budget: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(700);
            if budget == 0 || budget > 100_000 { usage(); }
            no_extra_args(&mut args);
            let raw_bytes = fs::read(&path).unwrap_or_else(|error| fail(error));
            if raw_bytes.is_empty() { fail("benchmark corpus must not be empty"); }
            let raw_text = std::str::from_utf8(&raw_bytes).unwrap_or_else(|error| fail(error));
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
            let temp_home = env::temp_dir().join(format!("conduit-bench-{}-{stamp}", process::id()));
            let config = RunConfig { compact: true, budget, ..preset("compact").expect("preset") };
            let result = RunResult {
                stdout: Capture { bytes: raw_bytes.clone(), total_bytes: raw_bytes.len() as u64, truncated: false },
                stderr: Capture { bytes: Vec::new(), total_bytes: 0, truncated: false },
                exit_code: 0,
                timed_out: false,
                elapsed: Duration::ZERO,
            };
            let (shown_bytes, savings) = render_stdout(&result, config, &temp_home);
            let (raw_words, presented_words, compact, lossless) = if let Some((handle, raw, shown)) = savings {
                let store = BlobStore::open(temp_home.join("blobs")).unwrap_or_else(|error| fail(error));
                let lossless = store.get(&handle).map(|bytes| bytes == raw_bytes).unwrap_or(false);
                (raw, shown, true, lossless)
            } else {
                let words = raw_text.split_whitespace().count();
                (words, words, false, shown_bytes == raw_bytes)
            };
            if !lossless { fail("benchmark expansion failed to recover original bytes"); }
            if let Err(error) = fs::remove_dir_all(&temp_home) {
                if error.kind() != io::ErrorKind::NotFound {
                    eprintln!("conduit: unable to clean benchmark temp state: {error}");
                }
            }
            println!("raw_words={raw_words} presented_words={presented_words} raw_lines={} presented_lines={} compact={compact} raw_recoverable={lossless} mode=rust_cli_effective_word_estimate", raw_text.lines().count(), String::from_utf8_lossy(&shown_bytes).lines().count());
        }
        _ => usage(),
    }
}

fn fail(error: impl std::fmt::Display) -> ! {
    eprintln!("conduit: {error}");
    process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_session_path_injection() {
        for bad in ["../sessions", ".", "../../etc/passwd", "a/b", "", "a\\b", "a\n"] {
            assert!(session_log(bad).is_err());
        }
    }

    #[test]
    fn presets_have_distinct_budgets_and_modes() {
        assert!(!preset("safe").unwrap().compact);
        assert!(preset("compact").unwrap().compact);
        assert!(preset("quick").unwrap().timeout_ms < preset("deep").unwrap().timeout_ms);
        assert!(preset("unknown").is_none());
    }

    #[test]
    fn command_flag_overrides_preset_regardless_of_order() {
        let before = vec!["--timeout-ms", "1500", "--preset", "quick", "--", "echo"];
        let after = vec!["--preset", "quick", "--timeout-ms", "1500", "--", "echo"];
        for tokens in [before, after] {
            let (config, command, remaining) = parse_run(tokens.into_iter().map(ToOwned::to_owned));
            assert_eq!(config.timeout_ms, 1500);
            assert_eq!(config.max_output_bytes, 4 * 1024 * 1024);
            assert_eq!(command, "echo");
            assert!(remaining.is_empty());
        }
    }

    #[test]
    fn store_stdin_rejects_overflow_instead_of_persisting_truncated_bytes() {
        assert_eq!(read_store_input(&b"abcd"[..], 4).unwrap(), b"abcd");
        assert_eq!(read_store_input(&b"abcde"[..], 4).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn rejects_large_batch_manifest_file() {
        let path = env::temp_dir().join(format!("conduit-manifest-{}.json", process::id()));
        fs::write(&path, vec![b'x'; 1024 * 1024 + 1]).unwrap();
        assert_eq!(read_manifest(&path).unwrap_err().kind(), io::ErrorKind::InvalidInput);
        let _ = fs::remove_file(path);
    }
}
