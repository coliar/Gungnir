use crate::{error, println, warn, log};
use conquer_once::spin::OnceCell;
use core::{pin::Pin, task::{Context, Poll}};
use crossbeam_queue::ArrayQueue;
use futures_util::{stream::Stream, task::AtomicWaker};

static CODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static USART_WAKER: AtomicWaker = AtomicWaker::new();

#[no_mangle]
pub extern "C" fn usart_add_code(code: u8) {
    if let Ok(queue) = CODE_QUEUE.try_get() {
        if let Err(_) = queue.push(code) {
            warn!("code queue full; dropping uart input");
        } else {
            USART_WAKER.wake();
        }
    } else {
        error!("code queue uninitialized");
    }
}

pub(crate) struct UsartCodeStream {
    _private: (),
}

impl UsartCodeStream {
    pub(crate) fn new() -> Self {
        CODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("UsartCodeStream::new should only be called once");
        UsartCodeStream { _private: () }
    }
}

impl Stream for UsartCodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = CODE_QUEUE
            .try_get()
            .expect("inputcode queue not initialized");

        if let Some(code) = queue.pop() {
            return Poll::Ready(Some(code));
        }

        USART_WAKER.register(&cx.waker());
        match queue.pop() {
            Some(code) => {
                USART_WAKER.take();
                Poll::Ready(Some(code))
            }
            None => Poll::Pending,
        }
    }
}