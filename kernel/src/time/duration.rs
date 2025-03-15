use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use super::{GCD_1G, GCD_1K, GCD_1M, TICK_HZ};

#[inline]
const fn div_ceil(num: u64, den: u64) -> u64 {
    (num + den - 1) / den
}

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
        Self { ticks }
    }

    pub(crate) const fn from_secs(secs: u64) -> Self {
        Self { ticks: secs * TICK_HZ }
    }

    pub(crate) const fn from_millis(millis: u64) -> Self {
        Self {
            ticks: div_ceil(millis * (TICK_HZ / GCD_1K), 1000 / GCD_1K),
        }
    }

    pub(crate) const fn from_micros(micros: u64) -> Self {
        Self {
            ticks: div_ceil(micros * (TICK_HZ / GCD_1M), 1_000_000 / GCD_1M),
        }
    }

    pub(crate) const fn from_nanos(nanos: u64) -> Self {
        Self {
            ticks: div_ceil(nanos * (TICK_HZ / GCD_1G), 1_000_000_000 / GCD_1G),
        }
    }

    pub(crate) const fn from_secs_floor(secs: u64) -> Self {
        Self { ticks: secs * TICK_HZ }
    }

    pub(crate) const fn from_millis_floor(millis: u64) -> Self {
        Self {
            ticks: millis * (TICK_HZ / GCD_1K) / (1_000 / GCD_1K),
        }
    }

    pub(crate) const fn from_micros_floor(micros: u64) -> Self {
        Self {
            ticks: micros * (TICK_HZ / GCD_1M) / (1_000_000 / GCD_1M),
        }
    }

    pub(crate) const fn from_hz(hz: u64) -> Self {
        let ticks = if hz >= TICK_HZ {
            1
        } else {
            (TICK_HZ + hz / 2) / hz
        };
        Self { ticks }
    }

    pub(crate) fn checked_add(&self, rhs: Duration) -> Option<Duration> {
        self.ticks.checked_add(rhs.ticks).map(|ticks| Duration { ticks })
    }

    pub(crate) fn checked_sub(&self, rhs: Duration) -> Option<Duration> {
        self.ticks.checked_sub(rhs.ticks).map(|ticks| Duration { ticks })
    }

    pub(crate) fn checked_mul(&self, rhs: u64) -> Option<Duration> {
        self.ticks.checked_mul(rhs).map(|ticks| Duration { ticks })
    }

    pub(crate) fn checked_div(&self, rhs: u64) -> Option<Duration> {
        self.ticks.checked_div(rhs).map(|ticks| Duration { ticks })
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("overflow when adding durations")
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("overflow when subtracting durations")
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<u64> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u64) -> Self::Output {
        self.checked_mul(rhs).expect("overflow when multiplying duration by scale")
    }
}

impl Mul<Duration> for u64 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<u64> for Duration {
    fn mul_assign(&mut self, rhs: u64) {
        *self = *self * rhs;
    }
}

impl Div<u64> for Duration {
    type Output = Duration;

    fn div(self, rhs: u64) -> Self::Output {
        self.checked_div(rhs).expect("divide by zero error when dividing duration by scalar")
    }
}

impl DivAssign<u64> for Duration {
    fn div_assign(&mut self, rhs: u64) {
        *self = *self / rhs;
    }
}

impl core::fmt::Display for Duration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ticks", self.ticks)
    }
}

impl TryFrom<core::time::Duration> for Duration {
    type Error = <u64 as TryFrom<u128>>::Error;

    fn try_from(value: core::time::Duration) -> Result<Self, Self::Error> {
        Ok(Self::from_micros(value.as_micros().try_into()?))
    }
}

impl From<Duration> for core::time::Duration {
    fn from(value: Duration) -> Self {
        core::time::Duration::from_micros(value.as_micros())
    }
}