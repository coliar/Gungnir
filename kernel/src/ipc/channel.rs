use core::{future::poll_fn, task::{Context, Poll}};

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use futures_util::task::AtomicWaker;

use super::async_mutex::AsyncMutex;

pub(crate) enum TryRecvErr {
    Empty,
}

pub(crate) enum TrySendErr<T> {
    Full(T),
}

struct ChannelState<T, const N: usize> {
    queue: VecDeque<T>,
    recv_waker: AtomicWaker,
    send_waker: AtomicWaker,
}

impl<T, const N: usize> ChannelState<T, N> {
    const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            recv_waker: AtomicWaker::new(),
            send_waker: AtomicWaker::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn is_full(&self) -> bool {
        self.queue.len() == N
    }

    fn clear(&mut self) {
        self.queue.clear();
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn rcve_with_context(&mut self, cx: &mut Context<'_>) -> Result<T, TryRecvErr> {
        if self.is_full() {
            self.send_waker.wake();
        }

        if let Some(msg) = self.queue.pop_front() {
            Ok(msg)
        } else {
            self.recv_waker.register(cx.waker());
            Err(TryRecvErr::Empty)
        }
    }

    fn send_with_context(&mut self, msg: T, cx: &mut Context<'_>) -> Result<(), TrySendErr<T>> {
        if self.is_full() {
            self.send_waker.register(cx.waker());
            Err(TrySendErr::Full(msg))
        } else {
            self.queue.push_back(msg);
            self.recv_waker.wake();
            Ok(())
        }
    }
}

struct Channel<T, const N: usize> {
    inner: AsyncMutex<ChannelState<T, N>>,
}

impl<T, const N: usize> Channel<T, N> {
    fn new() -> Self {
        Self {
            inner: AsyncMutex::new(ChannelState::new()),
        }
    }

    fn cap(&self) -> usize {
        N
    }

    async fn free_space(&self) -> usize {
        N - self.len().await
    }

    async fn is_full(&self) -> bool {
        self.inner.lock().await.is_full()
    }

    async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    async fn clear(&self) {
        self.inner.lock().await.clear();
    }

    async fn send(&self, msg: T) {
        let mut msg = Some(msg);

        poll_fn(|cx| {
            match msg.take() {
                Some(m1) => {
                    let mut guard = loop {
                        if let Ok(guard) = self.inner.try_lock() {
                            break guard;
                        }
                    };
                    match guard.send_with_context(m1, cx) {
                        Ok(..) => Poll::Ready(()),
                        Err(TrySendErr::Full(m2)) => {
                            msg = Some(m2);
                            drop(guard);
                            Poll::Pending
                        }
                    }
                },
                None => panic!("Message cannot be None"),
            }
        }).await
    }

    async fn recv(&self) -> T {
        poll_fn(|cx| {
            let mut guard = loop {
                if let Ok(guard) = self.inner.try_lock() {
                    break guard;
                }
            };
            match guard.rcve_with_context(cx) {
                Ok(msg) => Poll::Ready(msg),
                Err(TryRecvErr::Empty) => {
                    drop(guard);
                    Poll::Pending
                },
            }
        }).await
    }
}


pub(crate) struct Sender<T, const N: usize> {
    channel: Arc<Channel<T, N>>,
}

impl<T, const N: usize> Sender<T, N> {
    pub(crate) fn cap(&self) -> usize {
        self.channel.cap()
    }

    pub(crate) async fn free_space(&self) -> usize {
        self.channel.free_space().await
    }

    pub(crate) async fn clear(&self) {
        self.channel.clear().await;
    }

    pub(crate) async fn len(&self) -> usize {
        self.channel.len().await
    }

    pub(crate) async fn is_empty(&self) -> bool {
        self.channel.is_empty().await
    }

    pub(crate) async fn is_full(&self) -> bool {
        self.channel.is_full().await
    }

    pub(crate) async fn send(&self, msg: T) {
        self.channel.send(msg).await;
    }
}


pub(crate) struct Receiver<T, const N: usize> {
    channel: Arc<Channel<T, N>>,
}

impl<T, const N: usize> Receiver<T, N> {
    pub(crate) fn cap(&self) -> usize {
        self.channel.cap()
    }

    pub(crate) async fn free_space(&self) -> usize {
        self.channel.free_space().await
    }

    pub(crate) async fn clear(&self) {
        self.channel.clear().await;
    }

    pub(crate) async fn len(&self) -> usize {
        self.channel.len().await
    }

    pub(crate) async fn is_empty(&self) -> bool {
        self.channel.is_empty().await
    }

    pub(crate) async fn is_full(&self) -> bool {
        self.channel.is_full().await
    }

    pub(crate) async fn recv(&self) -> T {
        self.channel.recv().await
    }
}

pub(crate) fn channel<T, const N: usize>() -> (Sender<T, N>, Receiver<T, N>) {
    let channel = Arc::new(Channel::<T, N>::new());
    let sender = Sender { channel: channel.clone() };
    let receiver = Receiver { channel };
    (sender, receiver)
}