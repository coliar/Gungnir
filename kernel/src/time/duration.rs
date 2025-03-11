#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Duration {
    pub(super) ticks: u64,
}

impl Duration {
    pub(crate) const MIN: Duration = Duration { ticks: u64::MIN };
    pub(crate) const MAX: Duration = Duration { ticks: u64::MAX };

    pub(crate) const fn as_ticks(&self) -> u64 {
        self.ticks
    }

    
}