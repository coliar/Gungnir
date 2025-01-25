use alloc::{format, string::{String, ToString}};

use crate::println;

pub(crate) async fn future_test() {
    debug!("in future_test(), async message: {}", async_message().await);
    debug!("in future_test(), async num: {}", anum().await);
    debug!("in future_test(), async str: {}", astr().await);
    debug!("in future_test(), async sum: {}", async_sum(10, 20).await);
    debug!("in future_test(), async sum: {}", async_sum(60, 20).await);
}

async fn anum() -> u32 {
    32
}

async fn astr() -> String {
    String::from("hello asyncccccc")
}

async fn async_sum(a: u32, b: u32) -> u32 {
    a + b
}

async fn async_message() -> String {
    let part1 = "Hello".to_string();
    let part2 = " async".to_string();
    let part3 = String::from(" world!");
    format!("{}{}{}", part1, part2, part3)
}