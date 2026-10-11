use blake3::Hasher;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact { pub key: String, pub value: String, pub source_hash: String, pub valid: bool }
#[derive(Clone, Default)]
pub struct Memory { facts: Arc<Mutex<Vec<Fact>>> }
impl Memory {
    pub fn learn(&self, key: impl Into<String>, value: impl Into<String>, source: &[u8]) {
        let key = key.into(); let value = value.into(); let source_hash = digest(source);
        let mut facts = self.facts.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(fact) = facts.iter_mut().find(|fact| fact.key == key) { fact.value = value; fact.source_hash = source_hash; fact.valid = true; } else { facts.push(Fact { key, value, source_hash, valid: true }); }
    }
    pub fn recall(&self, key: &str, current_source: &[u8]) -> Option<Fact> {
        let source_hash = digest(current_source);
        let mut facts = self.facts.lock().unwrap_or_else(|poison| poison.into_inner());
        let fact = facts.iter_mut().find(|fact| fact.key == key)?;
        fact.valid = fact.source_hash == source_hash;
        fact.valid.then(|| fact.clone())
    }
    pub fn invalidate_changed(&self, current_source: &[u8]) -> usize {
        let hash = digest(current_source); let mut facts = self.facts.lock().unwrap_or_else(|poison| poison.into_inner());
        let mut count = 0; for fact in facts.iter_mut().filter(|fact| fact.valid && fact.source_hash != hash) { fact.valid = false; count += 1; } count
    }
}
fn digest(data: &[u8]) -> String { let mut hasher = Hasher::new(); hasher.update(data); hasher.finalize().to_hex().to_string() }
#[cfg(test)]
mod tests { use super::*; #[test] fn stale_facts_are_never_returned() { let memory = Memory::default(); memory.learn("build", "cargo test", b"v1"); assert!(memory.recall("build", b"v1").is_some()); assert!(memory.recall("build", b"v2").is_none()); assert_eq!(memory.invalidate_changed(b"v3"), 0); } #[test] fn learning_replaces_same_key() { let memory = Memory::default(); memory.learn("x", "one", b"a"); memory.learn("x", "two", b"b"); assert_eq!(memory.recall("x", b"b").unwrap().value, "two"); } }
