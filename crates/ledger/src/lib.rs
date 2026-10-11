use blake3::Hasher;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State { Live, Stubbed, Evicted }
#[derive(Debug, Clone)]
pub struct Item { pub id: String, pub content: String, pub tokens: usize, pub state: State, pub pinned: bool, pub turn: u64 }
#[derive(Clone, Default)]
pub struct Ledger { items: Arc<Mutex<Vec<Item>>> }

impl Ledger {
    pub fn insert(&self, content: impl Into<String>, turn: u64, pinned: bool) -> String {
        let content = content.into(); let id = handle(&content); let tokens = content.split_whitespace().count();
        let mut items = self.items.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(existing) = items.iter().find(|item| item.id == id) { return existing.id.clone(); }
        items.push(Item { id: id.clone(), content, tokens, state: State::Live, pinned, turn }); id
    }
    pub fn mask_to(&self, budget: usize) -> usize {
        let mut items = self.items.lock().unwrap_or_else(|poison| poison.into_inner());
        let mut total: usize = items.iter().filter(|item| item.state == State::Live).map(|item| item.tokens).sum();
        let mut changed = 0;
        let mut candidates: Vec<usize> = (0..items.len()).filter(|index| !items[*index].pinned && items[*index].state == State::Live).collect();
        candidates.sort_by_key(|index| (items[*index].turn, std::cmp::Reverse(items[*index].tokens)));
        for index in candidates {
            if total <= budget { break; }
            total = total.saturating_sub(items[index].tokens); items[index].state = State::Stubbed; changed += 1;
        }
        changed
    }
    pub fn restore(&self, id: &str) -> Option<String> {
        let mut items = self.items.lock().unwrap_or_else(|poison| poison.into_inner());
        let item = items.iter_mut().find(|item| item.id == id)?; item.state = State::Live; Some(item.content.clone())
    }
    pub fn snapshot(&self) -> Vec<Item> { self.items.lock().unwrap_or_else(|poison| poison.into_inner()).clone() }
}
fn handle(content: &str) -> String {
    let mut hasher = Hasher::new(); hasher.update(content.as_bytes());
    let digest = hasher.finalize().to_hex().to_string();
    format!("cx:l:{}", &digest[..20])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn masking_is_deterministic_and_never_masks_pinned_items() { let ledger = Ledger::default(); ledger.insert("pinned", 1, true); ledger.insert("old content", 1, false); ledger.insert("new content", 2, false); assert_eq!(ledger.mask_to(1), 2); assert!(ledger.snapshot().iter().any(|item| item.pinned && item.state == State::Live)); }
    #[test]
    fn duplicate_insert_is_idempotent() { let ledger = Ledger::default(); let a = ledger.insert("same", 1, false); let b = ledger.insert("same", 9, false); assert_eq!(a, b); assert_eq!(ledger.snapshot().len(), 1); }
}
