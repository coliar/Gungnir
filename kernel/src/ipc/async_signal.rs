use core::{cell::Cell, future::{poll_fn, Future}, task::Poll};

use alloc::vec::Vec;
use futures_util::task::AtomicWaker;

struct CellWraper<T> {
    cell: Cell<T>,
}

impl<T> CellWraper<T> {
    fn replace(&self, val: T) -> T {
        self.cell.replace(val)
    }
}

unsafe impl<T> Send for CellWraper<T> {}
unsafe impl<T> Sync for CellWraper<T> {}

enum State<T> {
    Non,
    Waiting(Vec<AtomicWaker>),
    Signaled(T),
}

pub(crate) struct AsyncSignal<T> {
    state: CellWraper<State<T>>,
}

impl<T> AsyncSignal<T> {
    pub(crate) const fn new() -> Self {
        Self { state: CellWraper { cell: Cell::new(State::Non) } }
    }

    fn set(&self, val: State<T>) {
        self.state.cell.set(val);
    }
}

impl<T> Default for AsyncSignal<T> {
    fn default() -> Self {
        Self::new()
    }
}


impl<T: Clone> AsyncSignal<T> {
    pub(crate) fn signal(&self, val: T) {
        let old_state = self.state.cell.replace(State::Signaled(val));
        if let State::Waiting(waiters) = old_state {
            for waiter in waiters {
                waiter.wake();
            }
        }
    }

    pub(crate) fn is_signaled(&self) -> bool {
        let state = self.state.replace(State::Non);
        let res = if let State::Signaled(_) = state {
            true
        } else {
            false
        };
        self.set(state);
        return res;
    }

    pub(crate) fn reset(&self) {
        self.set(State::Non);
    }

    pub(crate) fn wait(&self) -> impl Future<Output = T> + Send + Sync + '_ {
        poll_fn(|cx| {
            let old_state = self.state.replace(State::Non);
            match old_state {
                State::Signaled(val) => {
                    self.set(State::Signaled(val.clone()));
                    Poll::Ready(val)
                },
                State::Non => {
                    let waker = AtomicWaker::new();
                    waker.register(&cx.waker());
                    let mut vec = Vec::new();
                    vec.push(waker);
                    self.set(State::Waiting(vec));
                    Poll::Pending
                },
                State::Waiting(mut vec) => {
                    let waker = AtomicWaker::new();
                    waker.register(&cx.waker());
                    vec.push(waker);
                    self.set(State::Waiting(vec));
                    Poll::Pending
                }
            }
        })
    }
}


pub(crate) mod test {
    use crate::println;

    use super::*;

    static SIG: AsyncSignal<u8> = AsyncSignal::new();

    pub(crate) async fn wait1() {
        println!("in wait1");
        let sig = SIG.wait().await;
        println!("wait1 --- sig: {}", sig);
    }

    pub(crate) async fn wait2() {
        println!("in wait2");
        let sig = SIG.wait().await;
        println!("wait2 --- sig: {}", sig);
    }

    pub(crate) async fn wait3() {
        println!("in wait3");
        let sig = SIG.wait().await;
        println!("wait3 --- sig: {}", sig);
    }

    pub(crate) async fn signal() {
        println!("in signal");
        SIG.signal(9);
        println!("signal done");
    }
}