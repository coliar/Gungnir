#![allow(dead_code)]

use core::sync::atomic::AtomicU32;

pub(crate) mod instant;
pub(crate) mod duration;
pub(crate) mod timer;

pub(crate) const TICK_HZ: u64 = 1_000;
const GCD_1K: u64 = gcd(TICK_HZ, 1_000);
const GCD_1M: u64 = gcd(TICK_HZ, 1_000_000);
const GCD_1G: u64 = gcd(TICK_HZ, 1_000_000_000);

const fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

pub(crate) static  SYS_TICKS: AtomicU32 = AtomicU32::new(0);


#[no_mangle]
pub extern "C" fn sys_tick_handler() {
    SYS_TICKS.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
}

pub(crate) fn get_sys_ticks() -> u64 {
    SYS_TICKS.load(core::sync::atomic::Ordering::Acquire) as u64
}