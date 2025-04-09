use core::{future::{poll_fn, Future}, sync::atomic::AtomicBool, task::Poll};

use alloc::{sync::Arc, vec::Vec};
use futures_util::task::AtomicWaker;
use spin::Mutex;

pub(super) static YIELD_LIST: Mutex<Vec<(Arc<AtomicWaker>, Arc<AtomicBool>)>> = Mutex::new(Vec::new());

#[allow(dead_code)]
pub(crate) fn yield_now() -> impl Future<Output = ()> + Send + Sync + 'static {
    let waked = Arc::new(AtomicBool::new(false));
    poll_fn(move |cx| {
        if !waked.load(core::sync::atomic::Ordering::Acquire) {
            let waker = Arc::new(AtomicWaker::new());
            waker.register(&cx.waker());
            YIELD_LIST.lock().push((waker, waked.clone()));
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
}