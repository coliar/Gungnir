#![allow(dead_code)]

#![allow(async_fn_in_trait)]

use core::{cmp, fmt::Debug};

use aligned::Aligned;
use embedded_io_async::{ErrorKind, Read, Seek, SeekFrom, Write};

use crate::{debug, println, log};

pub(crate) trait BlockDevice<const SIZE: usize> {
    type Error: core::fmt::Debug;

    type Align: aligned::Alignment;

    async fn read(
        &mut self,
        block_address: u32,
        data: &mut [Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error>;

    async fn write(
        &mut self,
        block_address: u32,
        data: &[Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error>;

    async fn size(&mut self) -> Result<u64, Self::Error>;
}

impl<T: BlockDevice<SIZE>, const SIZE: usize> BlockDevice<SIZE> for &mut T {
    type Error = T::Error;
    type Align = T::Align;

    async fn read(
        &mut self,
        block_address: u32,
        data: &mut [Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error> {
        (*self).read(block_address, data).await
    }

    async fn write(
        &mut self,
        block_address: u32,
        data: &[Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error> {
        (*self).write(block_address, data).await
    }

    async fn size(&mut self) -> Result<u64, Self::Error> {
        (*self).size().await
    }
}

fn slice_to_blocks<ALIGN, const SIZE: usize>(slice: &[u8]) -> &[Aligned<ALIGN, [u8; SIZE]>]
where
    ALIGN: aligned::Alignment,
{
    let align: usize = core::mem::align_of::<Aligned<ALIGN, ()>>();
    assert!(slice.len() % SIZE == 0);
    assert!(slice.len() % align == 0);
    assert!(slice.as_ptr().cast::<u8>() as usize % align == 0);
    unsafe {
        core::slice::from_raw_parts(
            slice.as_ptr() as *const Aligned<ALIGN, [u8; SIZE]>,
            slice.len() / SIZE,
        )
    }
}

fn slice_to_blocks_mut<ALIGN, const SIZE: usize>(slice: &mut [u8]) -> &mut [Aligned<ALIGN, [u8; SIZE]>]
where
    ALIGN: aligned::Alignment,
{
    let align: usize = core::mem::align_of::<Aligned<ALIGN, [u8; SIZE]>>();
    assert!(slice.len() % SIZE == 0);
    assert!(slice.len() % align == 0);
    assert!(slice.as_ptr().cast::<u8>() as usize % align == 0);
    unsafe {
        core::slice::from_raw_parts_mut(
            slice.as_mut_ptr() as *mut Aligned<ALIGN, [u8; SIZE]>,
            slice.len() / SIZE,
        )
    }
}

fn blocks_to_slice<ALIGN, const SIZE: usize>(buf: &[Aligned<ALIGN, [u8; SIZE]>]) -> &[u8]
where
    ALIGN: aligned::Alignment,
{
    let align: usize = core::mem::align_of::<Aligned<ALIGN, ()>>();
    assert!(SIZE % align == 0);
    unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * SIZE) }
}

fn blocks_to_slice_mut<ALIGN, const SIZE: usize>(buf: &mut [Aligned<ALIGN, [u8; SIZE]>]) -> &mut [u8]
where
    ALIGN: aligned::Alignment,
{
    let align: usize = core::mem::align_of::<Aligned<ALIGN, ()>>();
    assert!(SIZE % align == 0);
    unsafe { core::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len() * SIZE) }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum BufStreamError<T> {
    Io(T),
}

impl<T> From<T> for BufStreamError<T> {
    fn from(t: T) -> Self {
        BufStreamError::Io(t)
    }
}

impl<T: core::fmt::Debug> embedded_io_async::Error for BufStreamError<T> {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

pub(crate) struct BufStream<T: BlockDevice<SIZE>, const SIZE: usize> {
    inner: T,
    buffer: Aligned<T::Align, [u8; SIZE]>,
    current_block: u32,
    current_offset: u64,
    dirty: bool,
}

impl<T: BlockDevice<SIZE>, const SIZE: usize> BufStream<T, SIZE> {

    const ALIGN: usize = core::mem::align_of::<Aligned<T::Align, [u8; SIZE]>>();

    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            current_block: u32::MAX,
            current_offset: 0,
            buffer: Aligned([0; SIZE]),
            dirty: false,
        }
    }

    pub(crate) fn into_inner(self) -> T {
        self.inner
    }

    #[inline]
    fn pointer_block_start_addr(&self) -> u64 {
        self.pointer_block_start() as u64 * SIZE as u64
    }

    #[inline]
    fn pointer_block_start(&self) -> u32 {
        (self.current_offset / SIZE as u64)
            .try_into()
            .expect("Block larger than 2TB")
    }

    async fn flush(&mut self) -> Result<(), T::Error> {
        if self.dirty {
            self.dirty = false;
            self.inner
                .write(self.current_block, slice_to_blocks(&self.buffer[..]))
                .await?;
        }
        Ok(())
    }

    async fn check_cache(&mut self) -> Result<(), T::Error> {
        let block_start = self.pointer_block_start();
        if block_start != self.current_block {
            // we may have modified data in old block, flush it to disk
            self.flush().await?;
            // We have seeked to a new block, read it
            let buf = &mut self.buffer[..];
            self.inner
                .read(block_start, slice_to_blocks_mut(buf))
                .await?;
            self.current_block = block_start;
        }
        Ok(())
    }
}

impl<T: BlockDevice<SIZE>, const SIZE: usize> embedded_io_async::ErrorType for BufStream<T, SIZE> {
    type Error = BufStreamError<T::Error>;
}

impl<T: BlockDevice<SIZE>, const SIZE: usize> Read for BufStream<T, SIZE> {
    async fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut total = 0;
        let target = buf.len();
        loop {
            let bytes_read = if buf.len() % SIZE == 0
                && buf.as_ptr().cast::<u8>() as usize % Self::ALIGN == 0
                && self.current_offset % SIZE as u64 == 0
            {
                let block = self.pointer_block_start();
                self.inner.read(block, slice_to_blocks_mut(buf)).await?;

                buf.len()
            } else {
                let block_start = self.pointer_block_start_addr();
                let block_end = block_start + SIZE as u64;
                debug!(
                    "offset {}, block_start {}, block_end {}",
                    self.current_offset,
                    block_start,
                    block_end
                );

                self.check_cache().await?;

                // copy as much as possible, up to the block boundary
                let buffer_offset = (self.current_offset - block_start) as usize;
                let bytes_to_read = buf.len();

                let end = core::cmp::min(buffer_offset + bytes_to_read, SIZE);
                debug!("buffer_offset {}, end {}", buffer_offset, end);
                let bytes_read = end - buffer_offset;
                buf[..bytes_read].copy_from_slice(&self.buffer[buffer_offset..end]);
                buf = &mut buf[bytes_read..];

                bytes_read
            };

            self.current_offset += bytes_read as u64;
            total += bytes_read;

            if total == target {
                return Ok(total);
            }
        }
    }
}


impl<T: BlockDevice<SIZE>, const SIZE: usize> Write for BufStream<T, SIZE> {
    async fn write(&mut self, mut buf: &[u8]) -> Result<usize, Self::Error> {
        let mut total = 0;
        let target = buf.len();
        loop {
            let bytes_written = if buf.len() % SIZE == 0
                && buf.as_ptr().cast::<u8>() as usize % Self::ALIGN == 0
                && self.current_offset % SIZE as u64 == 0
            {
                let block = self.pointer_block_start();
                self.inner.write(block, slice_to_blocks(buf)).await?;

                buf.len()
            } else {
                let block_start = self.pointer_block_start_addr();
                let block_end = block_start + SIZE as u64;
                debug!(
                    "offset {}, block_start {}, block_end {}",
                    self.current_offset,
                    block_start,
                    block_end
                );

                // reload the cache if we need to
                self.check_cache().await?;

                // copy as much as possible, up to the block boundary
                let buffer_offset = (self.current_offset - block_start) as usize;
                let bytes_to_write = buf.len();

                let end = core::cmp::min(buffer_offset + bytes_to_write, SIZE);
                debug!("buffer_offset {}, end {}", buffer_offset, end);
                let bytes_written = end - buffer_offset;
                self.buffer[buffer_offset..buffer_offset + bytes_written]
                    .copy_from_slice(&buf[..bytes_written]);
                buf = &buf[bytes_written..];

                self.dirty = true;

                // write out the whole block with the modified data
                if block_start + end as u64 == block_end {
                    self.flush().await?;
                }

                bytes_written
            };

            self.current_offset += bytes_written as u64;
            total += bytes_written;

            if total == target {
                return Ok(total);
            }
        }
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush().await?;
        Ok(())
    }
}

impl<T: BlockDevice<SIZE>, const SIZE: usize> Seek for BufStream<T, SIZE> {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.current_offset = match pos {
            SeekFrom::Start(x) => x,
            SeekFrom::End(x) => (self.inner.size().await? as i64 - x) as u64,
            SeekFrom::Current(x) => (self.current_offset as i64 + x) as u64,
        };
        Ok(self.current_offset)
    }
}

#[derive(Debug)]
#[non_exhaustive]
enum StreamSliceError<T: Debug> {
    InvalidSeek(i64),
    WriteZero,
    Other(T),
}

impl<E: Debug> From<E> for StreamSliceError<E> {
    fn from(e: E) -> Self {
        Self::Other(e)
    }
}

struct StreamSlice<T: Read + Write + Seek> {
    inner: T,
    start_offset: u64,
    current_offset: u64,
    size: u64,
}

impl<E: Debug> embedded_io_async::Error for StreamSliceError<E> {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            StreamSliceError::InvalidSeek(_) => embedded_io_async::ErrorKind::InvalidInput,
            StreamSliceError::Other(_) | StreamSliceError::WriteZero => {
                embedded_io_async::ErrorKind::Other
            }
        }
    }
}

impl<T: Read + Write + Seek> embedded_io_async::ErrorType for StreamSlice<T> {
    type Error = StreamSliceError<T::Error>;
}

impl<T: Read + Write + Seek> StreamSlice<T> {
    pub async fn new(
        mut inner: T,
        start_offset: u64,
        end_offset: u64,
    ) -> Result<Self, StreamSliceError<T::Error>> {
        debug_assert!(end_offset >= start_offset);
        inner.seek(SeekFrom::Start(start_offset)).await?;
        let size = end_offset - start_offset;
        Ok(StreamSlice {
            start_offset,
            size,
            inner,
            current_offset: 0,
        })
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Read + Write + Seek> Read for StreamSlice<T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, StreamSliceError<T::Error>> {
        let max_read_size = cmp::min((self.size - self.current_offset) as usize, buf.len());
        let bytes_read = self.inner.read(&mut buf[..max_read_size]).await?;
        self.current_offset += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl<T: Read + Write + Seek> Write for StreamSlice<T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, StreamSliceError<T::Error>> {
        let max_write_size = cmp::min((self.size - self.current_offset) as usize, buf.len());
        let bytes_written = self.inner.write(&buf[..max_write_size]).await?;
        if bytes_written == 0 {
            return Err(StreamSliceError::WriteZero);
        }
        self.current_offset += bytes_written as u64;
        Ok(bytes_written)
    }

    async fn flush(&mut self) -> Result<(), StreamSliceError<T::Error>> {
        self.inner.flush().await?;
        Ok(())
    }
}

impl<T: Read + Write + Seek> Seek for StreamSlice<T> {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, StreamSliceError<T::Error>> {
        let new_offset = match pos {
            SeekFrom::Current(x) => self.current_offset as i64 + x,
            SeekFrom::Start(x) => x as i64,
            SeekFrom::End(x) => self.size as i64 + x,
        };
        if new_offset < 0 || new_offset as u64 > self.size {
            Err(StreamSliceError::InvalidSeek(new_offset))
        } else {
            self.inner
                .seek(SeekFrom::Start(self.start_offset + new_offset as u64))
                .await?;
            self.current_offset = new_offset as u64;
            Ok(self.current_offset)
        }
    }
}