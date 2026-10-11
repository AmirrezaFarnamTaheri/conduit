//! Pure accounting feasibility primitive. This is NOT a concurrent reservation
//! service and is not currently used to authorize `conduit run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub tokens: u64,
    pub dollars_micros: u64,
    pub wall_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub tokens: u64,
    pub dollars_micros: u64,
    pub wall_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Reject,
}

fn within_budget(used: u64, request: u64, maximum: u64) -> bool {
    // Saturating addition would incorrectly permit overflow if maximum=MAX.
    used.checked_add(request).is_some_and(|total| total <= maximum)
}

pub fn authorize(budget: Budget, used: Usage, requested: Usage) -> Decision {
    if within_budget(used.tokens, requested.tokens, budget.tokens)
        && within_budget(used.dollars_micros, requested.dollars_micros, budget.dollars_micros)
        && within_budget(used.wall_ms, requested.wall_ms, budget.wall_ms)
    {
        Decision::Allow
    } else {
        Decision::Reject
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_the_exact_limit_and_rejects_excess() {
        let budget = Budget { tokens: 10, dollars_micros: 10, wall_ms: 10 };
        let used = Usage { tokens: 9, dollars_micros: 9, wall_ms: 9 };
        assert_eq!(authorize(budget, used, Usage { tokens: 1, dollars_micros: 1, wall_ms: 1 }), Decision::Allow);
        assert_eq!(authorize(budget, used, Usage { tokens: 2, dollars_micros: 1, wall_ms: 1 }), Decision::Reject);
    }

    #[test]
    fn overflow_never_authorizes_a_request() {
        let budget = Budget { tokens: u64::MAX, dollars_micros: u64::MAX, wall_ms: u64::MAX };
        let zero = Usage { tokens: 0, dollars_micros: 0, wall_ms: 0 };
        let at_max = Usage { tokens: u64::MAX, dollars_micros: 0, wall_ms: 0 };
        assert_eq!(authorize(budget, at_max, Usage { tokens: 1, ..zero }), Decision::Reject);
        assert_eq!(authorize(budget, zero, Usage { tokens: u64::MAX, ..zero }), Decision::Allow);
    }
}
