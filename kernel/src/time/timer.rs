use core::{future::Future, pin::pin};
use alloc::{sync::Arc, vec::Vec};
use futures_core::{FusedStream, Stream};
use futures_util::{future::{select, Either}, task::AtomicWaker};
use spin::Mutex;
use super::{duration::Duration, instant::Instant};

pub(crate) static WAITING_TIMERS: Mutex<Vec<(Instant, Arc<AtomicWaker>)>> = Mutex::new(Vec::new());

pub(crate) struct Timer {
    expires_at: Instant,
    waker: Arc<AtomicWaker>,
}

impl Timer {
    pub(crate) fn at(expires_at: Instant) -> Self {
        Self { expires_at, waker: Arc::new(AtomicWaker::new()) }
    }

    pub(crate) fn after(duration: Duration) -> Self {
        Self {
            expires_at: Instant::now() + duration,
            waker: Arc::new(AtomicWaker::new()),
        }
    }

    pub(crate) fn after_ticks(ticks: u64) -> Self {
        Self::after(Duration::from_ticks(ticks))
    }

    pub(crate) fn after_nanos(nanos: u64) -> Self {
        Self::after(Duration::from_nanos(nanos))
    }

    pub(crate) fn after_micros(micros: u64) -> Self {
        Self::after(Duration::from_micros(micros))
    }

    pub(crate) fn after_millis(millis: u64) -> Self {
        Self::after(Duration::from_millis(millis))
    }

    pub(crate) fn after_secs(secs: u64) -> Self {
        Self::after(Duration::from_secs(secs))
    }
}

impl Unpin for Timer {}

impl Future for Timer {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        if self.expires_at <= Instant::now() {
            core::task::Poll::Ready(())
        } else {
            self.waker.register(cx.waker());
            WAITING_TIMERS.lock().push((self.expires_at, self.waker.clone()));
            core::task::Poll::Pending
        }
    }
}

pub(crate) struct Ticker {
    expires_at: Instant,
    duration: Duration,
    waker: Arc<AtomicWaker>,
}

impl Ticker {
    pub(crate) fn every(duration: Duration) -> Self {
        Self {
            expires_at: Instant::now() + duration,
            duration, 
            waker: Arc::new(AtomicWaker::new()),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.expires_at = Instant::now() + self.duration;
    }

    pub(crate) fn reset_at(&mut self, deadline: Instant) {
        self.expires_at = deadline + self.duration;
    }

    pub(crate) fn reset_after(&mut self, after: Duration) {
        self.expires_at = Instant::now() + after + self.duration;
    }
}

impl Unpin for Ticker {}

impl Stream for Ticker {
    type Item = ();
    fn poll_next(mut self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<Self::Item>> {
        if self.expires_at <= Instant::now() {
            let dur = self.duration;
            self.expires_at += dur;
            core::task::Poll::Ready(Some(()))
        } else {
            self.waker.register(cx.waker());
            WAITING_TIMERS.lock().push((self.expires_at, self.waker.clone()));
            core::task::Poll::Pending
        }
    }
}

impl FusedStream for Ticker {
    fn is_terminated(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimeoutError;

pub(crate) async fn with_timeout<F: Future>(timeout: Duration, fut: F) -> Result<F::Output, TimeoutError> {
    let timer = Timer::after(timeout);
    match select(pin!(fut), timer).await {
        Either::Left((r, _)) => Ok(r),
        Either::Right(_) => Err(TimeoutError),
    }
}

pub(crate) async fn with_deadline<F: Future>(deadline: Instant, fut: F) -> Result<F::Output, TimeoutError> {
    let timer = Timer::at(deadline);
    match select(pin!(fut), timer).await {
        Either::Left((r, _)) => Ok(r),
        Either::Right(_) => Err(TimeoutError),
    }
}

pub(crate) trait WithTimeout {
    type Output;

    async fn with_timeout(self, timeout: Duration) -> Result<Self::Output, TimeoutError>;

    async fn with_deadline(self, deadline: Instant) -> Result<Self::Output, TimeoutError>;
}

impl<F: Future> WithTimeout for F {
    type Output = F::Output;

    async fn with_timeout(self, timeout: Duration) -> Result<Self::Output, TimeoutError> {
        with_timeout(timeout, self).await
    }

    async fn with_deadline(self, deadline: Instant) -> Result<Self::Output, TimeoutError> {
        with_deadline(deadline, self).await
    }
}