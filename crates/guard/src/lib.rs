use blake3::Hasher;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision { Continue, Halt }
#[derive(Debug, Clone)]
pub struct Guard { fingerprints: VecDeque<String>, max_repeats: usize, consecutive_failures: usize, failure_limit: usize }
impl Guard {
    pub fn new(max_repeats: usize, failure_limit: usize) -> Self { Self { fingerprints: VecDeque::new(), max_repeats, consecutive_failures: 0, failure_limit } }
    pub fn observe(&mut self, action: &str, succeeded: bool) -> Decision {
        if succeeded { self.consecutive_failures = 0; } else { self.consecutive_failures += 1; }
        let mut hasher = Hasher::new(); hasher.update(action.as_bytes()); let fingerprint = hasher.finalize().to_hex().to_string();
        self.fingerprints.push_back(fingerprint.clone()); if self.fingerprints.len() > self.max_repeats + 1 { self.fingerprints.pop_front(); }
        let repeats = self.fingerprints.iter().filter(|item| item.as_str() == fingerprint).count();
        if repeats > self.max_repeats || self.consecutive_failures >= self.failure_limit { Decision::Halt } else { Decision::Continue }
    }
}
#[cfg(test)]
mod tests { use super::*; #[test] fn detects_repeated_actions() { let mut guard = Guard::new(2, 5); assert_eq!(guard.observe("same", false), Decision::Continue); assert_eq!(guard.observe("same", false), Decision::Continue); assert_eq!(guard.observe("same", false), Decision::Halt); } #[test] fn resets_failures_on_progress() { let mut guard = Guard::new(10, 2); assert_eq!(guard.observe("a", false), Decision::Continue); assert_eq!(guard.observe("b", true), Decision::Continue); assert_eq!(guard.observe("c", false), Decision::Continue); } }
