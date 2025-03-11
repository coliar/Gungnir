use super::{GCD_1K, GCD_1M, TICK_HZ};

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

    pub(crate) const fn as_secs(&self) -> u64 {
        self.ticks / TICK_HZ
    }

    pub(crate) const fn as_millis(&self) -> u64 {
        self.ticks * (1_000 / GCD_1K) / (TICK_HZ / GCD_1K)
    }

    pub(crate) const fn as_micros(&self) -> u64 {
        self.ticks * (1_000_000 / GCD_1M) / (TICK_HZ / GCD_1M)
    }

    pub(crate) const fn from_ticks(ticks: u64) -> Self {
        Duration { ticks }
    }

    pub(crate) const fn from_secs(secs: u64) -> Self {
        Duration { ticks: secs * TICK_HZ }
    }

    pub(crate) const fn from_millis(millis: u64) -> Self {
        Duration {
            ticks: div_ceil(millis * (TICK_HZ / GCD_1K), 1000 / GCD_1K),
        }
    }

    pub(crate) const fn from_micros(micros: u64) -> Self {
        Duration {
            ticks: div_ceil(micros * (TICK_HZ / GCD_1M), 1_000_000 / GCD_1M),
        }
    }

    pub(crate) const fn from_nanos(micros: u64) -> Self {
        Duration {
            ticks: div_ceil(micros * (TICK_HZ / GCD_1G), 1_000_000_000 / GCD_1G),
        }
    }

    pub(crate) const fn from_secs_floor(secs: u64) -> Self {
        Duration { ticks: secs * TICK_HZ }
    }

    pub(crate) const fn from_millis_floor(millis: u64) -> Self {
        Duration {
            ticks: millis * (TICK_HZ / GCD_1K) / (1_000 / GCD_1K),
        }
    }

    pub(crate) const fn from_micros_floor(micros: u64) -> Self {
        Duration {
            ticks: micros * (TICK_HZ / GCD_1M) / (1_000_000 / GCD_1M),
        }
    }

    pub(crate) const fn from_hz(hz: u64) -> Self {
        let ticks = if hz >= TICK_HZ {
            1
        } else {
            (TICK_HZ + hz / 2) / hz
        };
        Duration { ticks }
    }

    pub(crate) fn checked_add(&self, rhs: Duration) -> Option<Duration> {
        self.ticks.checked_add(rhs.ticks).map(|ticks| Duration { ticks })
    }
}

#[inline]
const fn div_ceil(num: u64, den: u64) -> u64 {
    (num + den - 1) / den
}