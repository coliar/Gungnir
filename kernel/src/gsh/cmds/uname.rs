use crate::{gsh::{register_cmd, CmdEntry}, println};
use core::pin::Pin;
use alloc::{boxed::Box, collections::vec_deque::VecDeque, string::String};
use core::future::Future;

async fn uname_func() {
    println!("{}-{}", "Gungnir", env!("CARGO_PKG_VERSION"));
}

fn uname_func_wrapper(_params: VecDeque<String>) -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(uname_func())
}

pub(super) fn add_cmd() {
    register_cmd("uname", CmdEntry::new("Prints system information", uname_func_wrapper));
}