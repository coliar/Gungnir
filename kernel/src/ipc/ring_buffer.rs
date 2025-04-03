#![allow(dead_code)]

use core::ops::Range;

pub(crate) struct RingBuffer<const N: usize> {
    start: usize,
    end: usize,
    full: bool,
}

impl<const N: usize> RingBuffer<N> {
    pub(crate) const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            full: false,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        !self.full && self.start == self.end
    }

    pub(crate) fn is_full(&self) -> bool {
        self.full
    }

    pub(crate) fn len(&self) -> usize {
        if self.is_empty() {
            0
        } else if self.start < self.end {
            self.end - self.start
        } else {
            N - self.start + self.end
        }
    }

    pub(crate) fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
        self.full = false;
    }

    pub(crate) fn wrap(&self, n: usize) -> usize {
        assert!(n <= N);
        if n == N {
            0
        } else {
            n
        }
    }

    pub(crate) fn push_buf(&mut self) -> Range<usize> {
        if self.is_full() {
            return 0..0;
        }

        let n = if self.start <= self.end {
            N - self.end
        } else {
            self.start - self.end
        };
        self.end..self.end + n
    }

    pub(crate) fn push(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        self.end = self.wrap(self.end + n);
        self.full = self.end == self.start;
    }

    pub fn pop_buf(&mut self) -> Range<usize> {
        if self.is_empty() {
            return 0..0;
        }

        let n = if self.end <= self.start {
            N - self.start
        } else {
            self.end - self.start
        };
        self.start..self.start + n
    }

    pub fn pop(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        self.start = self.wrap(self.start + n);
        self.full = false;
    }

}