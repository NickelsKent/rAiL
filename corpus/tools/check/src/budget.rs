//! The pull-request time budget (NFR9.19).

use std::time::{Duration, Instant};

/// A budget that started when it was created; `None` means unlimited.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    start: Instant,
    limit: Option<Duration>,
}

impl Budget {
    pub fn new(limit: Option<Duration>) -> Budget {
        Budget {
            start: Instant::now(),
            limit,
        }
    }

    /// Time left, or `None` when unlimited.
    pub fn remaining(&self) -> Option<Duration> {
        self.limit
            .map(|limit| limit.saturating_sub(self.start.elapsed()))
    }

    pub fn exhausted(&self) -> bool {
        self.remaining().is_some_and(|left| left.is_zero())
    }
}
