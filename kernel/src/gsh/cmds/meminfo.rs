use alloc::{collections::vec_deque::VecDeque, string::String, boxed::Box};
use core::pin::Pin;
use core::future::Future;
use crate::{gsh::{register_cmd, CmdEntry}, println, ALLOCATOR};

async fn meminfo_func() {
    let mem_size = ALLOCATOR.lock().size();
    let mem_used = ALLOCATOR.lock().used();
    let mem_free = ALLOCATOR.lock().free();
    println!("heap info : (KB)");
    println!("{:<8} {:<8} {:<8}", "total", "used", "free");
    println!("{:<8} {:<8} {:<8}", mem_size / 1024, mem_used / 1024, mem_free / 1024);
}

fn meminfo_func_wrapper(_params: VecDeque<String>) -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(meminfo_func())
}

pub(super) fn add_cmd() {
    register_cmd("meminfo", CmdEntry::new("get memory info", meminfo_func_wrapper));
}