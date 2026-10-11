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
        if seen.insert((*line).to_owned(), ()).is_none() { selected.push(*line); }
    }
    let mut presented = selected.join("\n");
    if !presented.is_empty() { presented.push('\n'); }
    let presented_tokens = token_count(&presented);
    if presented_tokens > budget {
        let mut bounded = String::new();
        for line in presented.lines() {
            let addition = token_count(line) + usize::from(!bounded.is_empty());
            if token_count(&bounded) + addition > budget { break; }
            if !bounded.is_empty() { bounded.push('\n'); }
            bounded.push_str(line);
        }
        presented = bounded;
    }
    let presented_tokens = token_count(&presented);
    let dropped = vec![format!("{} lines", lines.len().saturating_sub(presented.lines().count()))];
    Distilled { presented, dropped, raw_tokens, presented_tokens }
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
        assert!(result.presented_tokens <= 8);
        assert!(!result.dropped.is_empty());
    }
    #[test]
    fn passthrough_is_byte_exact_under_budget() {
        let raw = "ok\n";
        assert_eq!(distill(raw, 10).presented, raw);
        assert!(distill(raw, 10).dropped.is_empty());
    }
}
