use crate::{gsh::{register_cmd, CmdEntry}, println};
use core::pin::Pin;
use alloc::{boxed::Box, collections::vec_deque::VecDeque, string::String};
use core::future::Future;

const POEM: &str = "桃李春风一杯酒，江湖夜雨十年灯。";

#[allow(dead_code)]
async fn poem_func() {
    println!("{}", POEM);
}

fn poem_func_wrapper(_params: VecDeque<String>) -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(poem_func())
}

pub(super) fn add_cmd() {
    register_cmd("poem", CmdEntry::new("Prints a poem", poem_func_wrapper));
}