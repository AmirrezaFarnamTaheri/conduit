use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test { pub id: String, pub files: BTreeSet<String> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan { pub tests: Vec<String>, pub widened: bool }
pub fn impacted(tests: &[Test], changed: &BTreeSet<String>) -> Plan {
    if changed.is_empty() { return Plan { tests: Vec::new(), widened: false }; }
    let mut selected: Vec<String> = tests.iter().filter(|test| test.files.iter().any(|file| changed.contains(file))).map(|test| test.id.clone()).collect();
    let covered: BTreeSet<&String> = tests.iter().filter(|test| test.files.iter().any(|file| changed.contains(file))).flat_map(|test| test.files.iter()).collect();
    let widened = changed.iter().any(|file| !covered.contains(file));
    if widened { selected = tests.iter().map(|test| test.id.clone()).collect(); }
    selected.sort(); Plan { tests: selected, widened }
}
#[cfg(test)]
mod tests { use super::*; fn test(id: &str, file: &str) -> Test { Test { id: id.into(), files: [file.into()].into_iter().collect() } } #[test] fn selects_impacted_tests() { let plan = impacted(&[test("a", "src/a"), test("b", "src/b")], &["src/b".into()].into_iter().collect()); assert_eq!(plan.tests, vec!["b"]); assert!(!plan.widened); } #[test] fn widens_when_coverage_is_missing() { let plan = impacted(&[test("a", "src/a")], &["src/missing".into()].into_iter().collect()); assert_eq!(plan.tests, vec!["a"]); assert!(plan.widened); } }
