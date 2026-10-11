#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget { pub tokens: u64, pub dollars_micros: u64, pub wall_ms: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage { pub tokens: u64, pub dollars_micros: u64, pub wall_ms: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision { Allow, Reject }

pub fn authorize(budget: Budget, used: Usage, requested: Usage) -> Decision {
    if used.tokens.saturating_add(requested.tokens) > budget.tokens || used.dollars_micros.saturating_add(requested.dollars_micros) > budget.dollars_micros || used.wall_ms.saturating_add(requested.wall_ms) > budget.wall_ms { Decision::Reject } else { Decision::Allow }
}

#[cfg(test)]
mod tests { use super::*; #[test] fn hard_limits_reject_overruns() { let budget = Budget { tokens: 10, dollars_micros: 10, wall_ms: 10 }; let used = Usage { tokens: 9, dollars_micros: 9, wall_ms: 9 }; assert_eq!(authorize(budget, used, Usage { tokens: 1, dollars_micros: 1, wall_ms: 1 }), Decision::Allow); assert_eq!(authorize(budget, used, Usage { tokens: 2, dollars_micros: 1, wall_ms: 1 }), Decision::Reject); } }
