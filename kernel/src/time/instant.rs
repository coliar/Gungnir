use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::{duration::Duration, get_sys_ticks, GCD_1K, GCD_1M, TICK_HZ};


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Instant {
    ticks: u64,
}

impl Instant {
    pub(crate) const MIN: Instant = Instant { ticks: u64::MIN };
    pub(crate) const MAX: Instant = Instant { ticks: u64::MAX };

    pub(crate) fn now() -> Self {
        Self { ticks: get_sys_ticks() }
    }

    pub(crate) const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    pub(crate) const fn from_micros(micros: u64) -> Self {
        Self {
            ticks: micros * (TICK_HZ / GCD_1M) / (1_000_000 / GCD_1M),
        }
    }

    pub(crate) const fn from_millis(millis: u64) -> Self {
        Self {
            ticks: millis * (TICK_HZ / GCD_1K) / (1_000 / GCD_1K),
        }
    }

    pub(crate) const fn from_secs(secs: u64) -> Self {
        Self {
            ticks: secs * TICK_HZ,
        }
    }

    pub(crate) const fn as_ticks(&self) -> u64 {
        self.ticks
    }

    pub(crate) const fn as_secs(&self) -> u64 {
        self.ticks / TICK_HZ
    }

    pub(crate) const fn as_millis(&self) -> u64 {
        self.ticks * (1_000 / GCD_1K) / (TICK_HZ / GCD_1K)
    }

    pub(crate) const fn as_micros(&self) -> u64 {
        self.ticks * (1_000_000 / GCD_1M) / (TICK_HZ / GCD_1M)
    }

    pub(crate) fn duration_since(&self, earlier: Instant) -> Duration {
        Duration {
            ticks: self.ticks.checked_sub(earlier.ticks).expect("since error"),
        }
    }

    pub(crate) fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        if self.ticks < earlier.ticks {
            None
        } else {
            Some(Duration {
                ticks: self.ticks - earlier.ticks,
            })
        }
    }

    pub(crate) fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        Duration {
            ticks: if self.ticks < earlier.ticks {
                0
            } else {
                self.ticks - earlier.ticks
            },
        }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        Instant::now() - *self
    }

    pub(crate) fn checked_add(&self, dura: Duration) -> Option<Instant> {
        self.ticks.checked_add(dura.ticks).map(|ticks| Instant {ticks})
    }

    pub(crate) fn checked_sub(&self, dura: Duration) -> Option<Instant> {
        self.ticks.checked_sub(dura.ticks).map(|ticks| Instant {ticks})
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs).expect("Instant adding Duration is overflowed")
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;
    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs).expect("Instant subbing Duration is overflowed")
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        self.checked_duration_since(rhs).unwrap()
    }
}

impl core::fmt::Display for Instant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ticks", self.ticks)
    }
}