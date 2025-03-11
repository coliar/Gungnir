mod instant;
mod duration;
mod timer;

pub(crate) const TICK_HZ: u64 = 1_000_000;
const GCD_1K: u64 = gcd(TICK_HZ, 1_000);
const GCD_1M: u64 = gcd(TICK_HZ, 1_000_000);
const GCD_1G: u64 = gcd(TICK_HZ, 1_000_000_000);