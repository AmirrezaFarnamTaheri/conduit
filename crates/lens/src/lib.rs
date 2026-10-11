//! Conservative, reversible-presentation log reduction.
//!
//! `distill` never stores raw data. Callers MUST preserve recoverable raw bytes
//! before displaying a reduced result. An exceeded budget is possible when
//! retaining diagnostics is safer than omitting them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distilled {
    pub presented: String,
    pub dropped: Vec<String>,
    /// Whitespace-delimited word estimates, NOT provider token measurements.
    pub raw_tokens: usize,
    pub presented_tokens: usize,
}

pub fn distill(raw: &str, budget: usize) -> Distilled {
    let raw_tokens = token_count(raw);
    let lines: Vec<&str> = raw.lines().collect();
    if budget == 0 || raw_tokens <= budget || lines.is_empty() {
        return Distilled {
            presented: raw.to_owned(),
            dropped: Vec::new(),
            raw_tokens,
            presented_tokens: raw_tokens,
        };
    }

    // Diagnostics win over a soft presentation budget. In particular, never
    // suppress the first error just because a later summary is shorter.
    let mut keep = vec![false; lines.len()];
    let mut retained_words = 0usize;
    for (index, line) in lines.iter().enumerate() {
        if is_signal(line.trim()) {
            keep[index] = true;
            retained_words += token_count(line);
        }
    }

    // Add recent surrounding context within the remaining budget. Selection
    // happens newest-first, but output is emitted in original line order.
    for index in (lines.len().saturating_sub(20)..lines.len()).rev() {
        if keep[index] {
            continue;
        }
        let words = token_count(lines[index]);
        if retained_words.saturating_add(words) <= budget {
            keep[index] = true;
            retained_words += words;
        }
    }

    let selected: Vec<&str> = lines
        .iter()
        .zip(keep.iter())
        .filter_map(|(line, included)| included.then_some(*line))
        .collect();
    let removed = lines.len() - selected.len();
    if removed == 0 {
        return Distilled {
            presented: raw.to_owned(),
            dropped: Vec::new(),
            raw_tokens,
            presented_tokens: raw_tokens,
        };
    }
    let presented = format!("{}\n", selected.join("\n"));
    Distilled {
        presented_tokens: token_count(&presented),
        presented,
        dropped: vec![format!("{removed} lines")],
        raw_tokens,
    }
}

fn is_signal(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ["error", "failed", "failure", "panic", "fatal", "traceback", "exception", "assertion", "warning", "passed", "exit code", "exit="]
        .iter()
        .any(|needle| lower.contains(needle))
        || line.contains(".rs:")
        || line.contains(".py:")
        || line.contains(".ts:")
}

fn token_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_diagnostics_and_exit_state_chronologically() {
        let raw = "noise one\nerror src/lib.rs:42 failed\nnoise two\nexit=1\n";
        let result = distill(raw, 5);
        assert!(result.presented.contains("src/lib.rs:42"));
        assert!(result.presented.contains("exit=1"));
        assert!(result.presented.find("src/lib.rs:42") < result.presented.find("exit=1"));
        assert!(!result.dropped.is_empty());
    }

    #[test]
    fn preserves_critical_signal_even_over_soft_budget() {
        let raw = "error this diagnostic has a long explanation src/main.rs:9\nordinary noise\n";
        let result = distill(raw, 1);
        assert!(result.presented.contains("src/main.rs:9"));
        assert!(result.presented_tokens > 1);
    }

    #[test]
    fn does_not_report_a_drop_when_all_lines_are_retained() {
        let raw = "ordinary\nerror src/main.rs:1\n";
        let result = distill(raw, 3);
        assert_eq!(result.presented, raw);
        assert!(result.dropped.is_empty());
    }

    #[test]
    fn passthrough_is_byte_exact_under_budget() {
        let raw = "ok\r\n";
        assert_eq!(distill(raw, 10).presented, raw);
    }

    #[test]
    fn does_not_reorder_signal_lines_with_tail_context() {
        let raw = "error:first a b\nnoise a b\nwarning:second c d\nnoise c d\nexit=1\n";
        let shown = distill(raw, 7).presented;
        let ordered: Vec<_> = shown.lines().collect();
        let a = ordered.iter().position(|line| line.contains("error:first")).unwrap();
        let b = ordered.iter().position(|line| line.contains("warning:second")).unwrap();
        let c = ordered.iter().position(|line| line.contains("exit=1")).unwrap();
        assert!(a < b && b < c);
    }
}
