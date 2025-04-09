use core::{cell::{RefCell, UnsafeCell}, future::poll_fn, sync::atomic::{AtomicBool, Ordering}, task::Poll};

use alloc::{sync::Arc, vec::Vec};
use futures_util::task::AtomicWaker;

struct State {
    locked: AtomicBool,
    waiter_list: Vec<Arc<AtomicWaker>>,
}

pub(crate) struct AsyncMutex<T: ?Sized> {
    state: RefCell<State>,
    inner: UnsafeCell<T>,
}

unsafe impl<T: ?Sized> Sync for AsyncMutex<T> {}
unsafe impl<T: ?Sized> Send for AsyncMutex<T> {}

impl<T> From<T> for AsyncMutex<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

impl<T: Default> Default for AsyncMutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> AsyncMutex<T> {
    pub(crate) const fn new(inner: T) -> Self {
        Self {
            state: RefCell::new(State {
                locked: AtomicBool::new(false),
                waiter_list: Vec::new(),
            }),
            inner: UnsafeCell::new(inner),
        }
    }
}

impl<T: ?Sized> AsyncMutex<T> {
    pub(crate) async  fn lock(&self) -> AsyncMutexGuard<'_, T> {
        poll_fn(|cx| {
            let mut state = self.state.borrow_mut();
            if state.locked.load(Ordering::Acquire) {
                let waker = Arc::new(AtomicWaker::new());
                waker.register(&cx.waker());
                state.waiter_list.push(waker);
                Poll::Pending
            } else {
                state.locked.store(true, Ordering::Release);
                Poll::Ready(AsyncMutexGuard {
                    mutex: self,
                })
            }
        }).await
    }

    pub(crate) fn try_lock(&self) -> Result<AsyncMutexGuard<'_, T>, ()> {
        let state = self.state.borrow();
        if state.locked.load(Ordering::Acquire) {
            Err(())
        } else {
            state.locked.store(true, Ordering::Release);
            Ok(AsyncMutexGuard {
                mutex: self,
            })
        }
    }
}

pub(crate) struct AsyncMutexGuard<'a, T: ?Sized> {
    mutex: &'a AsyncMutex<T>,
}

impl<'a, T: ?Sized> Drop for AsyncMutexGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.mutex.state.try_borrow_mut().unwrap();
        state.locked.store(false, Ordering::Release);
        if let Some(waker) = state.waiter_list.pop() {
            waker.wake();
        }
    }
}

impl<'a, T: ?Sized> core::ops::Deref for AsyncMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<'a, T: ?Sized> core::ops::DerefMut for AsyncMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.inner.get() }
    }
}