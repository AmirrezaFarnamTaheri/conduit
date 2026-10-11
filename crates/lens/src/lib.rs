use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distilled {
    pub presented: String,
    pub dropped: Vec<String>,
    pub raw_tokens: usize,
    pub presented_tokens: usize,
}

pub fn distill(raw: &str, budget: usize) -> Distilled {
    let raw_tokens = token_count(raw);
    let lines: Vec<&str> = raw.lines().collect();
    if raw_tokens <= budget || lines.is_empty() {
        return Distilled { presented: raw.to_owned(), dropped: Vec::new(), raw_tokens, presented_tokens: raw_tokens };
    }

    let mut selected = Vec::new();
    let mut seen = BTreeMap::new();
    for line in &lines {
        let trimmed = line.trim();
        if is_signal(trimmed) && seen.insert(trimmed.to_owned(), ()).is_none() { selected.push(*line); }
    }

    let tail_start = lines.len().saturating_sub(20);
    for line in &lines[tail_start..] {
        if seen.contains_key(line) { continue; }
        let candidate = line.to_string();
        let with_candidate = join_lines(&selected, Some(line));
        if token_count(&with_candidate) <= budget {
            seen.insert(candidate, ());
            selected.push(*line);
        }
    }

    let presented = join_lines(&selected, None);
    let retained = selected.len();
    let dropped = if retained < lines.len() { vec![format!("{} lines", lines.len() - retained)] } else { Vec::new() };
    Distilled { presented: presented.clone(), dropped, raw_tokens, presented_tokens: token_count(&presented) }
}

fn join_lines(lines: &[&str], extra: Option<&str>) -> String {
    let mut result = lines.join("\n");
    if let Some(line) = extra {
        if !result.is_empty() { result.push('\n'); }
        result.push_str(line);
    }
    if !result.is_empty() { result.push('\n'); }
    result
}

fn is_signal(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ["error", "failed", "failure", "panic", "fatal", "traceback", "exception", "assertion", "warning", "passed", "exit code", "exit="]
        .iter().any(|needle| lower.contains(needle)) || line.contains(":/") || line.contains(".rs:") || line.contains(".py:") || line.contains(".ts:")
}

fn token_count(text: &str) -> usize { text.split_whitespace().count() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_failure_location_and_exit_state() {
        let raw = "noise one\nnoise two\nerror src/lib.rs:42 failed\nnoise three\nexit=1\n";
        let result = distill(raw, 5);
        assert!(result.presented.contains("src/lib.rs:42"));
        assert!(result.presented.contains("exit=1"));
        assert!(result.presented_tokens <= 5);
        assert!(!result.dropped.is_empty());
    }
    #[test]
    fn never_drops_a_signal_when_budget_is_too_small() {
        let raw = "error this diagnostic has a long explanation src/main.rs:9\nordinary noise\n";
        let result = distill(raw, 1);
        assert!(result.presented.contains("src/main.rs:9"));
        assert!(result.presented_tokens > 1);
    }

    #[test]
    fn does_not_report_a_drop_when_all_lines_are_retained() {
        let raw = "ordinary\nerror src/main.rs:1\n";
        let result = distill(raw, 3);
        assert!(result.dropped.is_empty());
    }

    #[test]
    fn passthrough_is_byte_exact_under_budget() {
        let raw = "ok\n";
        assert_eq!(distill(raw, 10).presented, raw);
        assert!(distill(raw, 10).dropped.is_empty());
    }

}
