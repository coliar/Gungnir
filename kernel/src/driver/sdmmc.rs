#![allow(dead_code)]

use core::{future::{poll_fn, Future}, sync::atomic::AtomicU32, u8};
use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use futures_util::task::AtomicWaker;
use aligned::{Aligned, A4};
use spin::Mutex;
use crate::{c_api::{get_sdcard_capacity, sdmmc_read_blocks_it, sdmmc_write_blocks_it}, ipc::async_mutex::AsyncMutex};
use super::block_device_driver::BlockDevice;
use crate::{error, log};


static IO_REQS: Mutex<BTreeMap<IoRequest, (Arc<AtomicWaker>, Arc<AtomicU32>)>> = Mutex::new(BTreeMap::new());

#[no_mangle]
pub static READ_REQUEST: u32 = 1;

#[no_mangle]
pub static WRITE_REQUEST: u32 = 2;


#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct IoRequest {
    req: u32,
    addr: usize,
}

impl IoRequest {
    fn new(req: u32, addr: usize) -> Self {
        assert!(req == READ_REQUEST || req == WRITE_REQUEST);
        Self { req, addr }
    }
}

pub(crate) struct SdmmcIo {
    waker: Arc<AtomicWaker>,
    io_status: Arc<AtomicU32>,
}

impl SdmmcIo {
    const IO_START: u32 = 0;
    const IO_WAITING: u32 = 1;
    const IO_READY: u32 = 2;

    pub(crate) fn new() -> Self {
        Self {
            waker: Arc::new(AtomicWaker::new()),
            io_status: Arc::new(AtomicU32::new(Self::IO_START)),
        }
    }

    fn get_io_status(&self) -> u32 {
        self.io_status.load(core::sync::atomic::Ordering::Acquire)
    }

    fn set_io_status(&self, status: u32) {
        assert!(status == Self::IO_START || status == Self::IO_WAITING || status == Self::IO_READY);
        self.io_status.store(status, core::sync::atomic::Ordering::Release);
    }

    fn read_blocks_it(&self, buf: *mut u8, addr: u32, num: u32) -> i32 {
        unsafe { sdmmc_read_blocks_it(buf, addr, num) }
    }

    fn write_blocks_it(&self, data: *const u8, addr: u32, num: u32) -> i32 {
        unsafe { sdmmc_write_blocks_it(data, addr, num) }
    }

    fn wait(&self) -> impl Future<Output = ()> + Send + Sync + '_ {
        poll_fn(|cx| {
            if self.get_io_status() == Self::IO_START {
                self.waker.register(&cx.waker());
                self.set_io_status(Self::IO_WAITING);
                core::task::Poll::Pending
            } else {
                core::task::Poll::Ready(())
            }
        })
    }
}

static SD_IO_LOCK: AsyncMutex<()> = AsyncMutex::new(());

impl<const SIZE: usize> BlockDevice<SIZE> for SdmmcIo {
    type Align = A4;
    type Error = ();

    async fn read(
            &mut self,
            block_address: u32,
            data: &mut [Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error> {
        let num_block = SIZE / 512;

        if data.len() == 0 {
            return Ok(());
        }

        let guard = SD_IO_LOCK.lock().await;
        for (i, buf) in data.iter_mut().enumerate() {
            let buf_ptr = buf[..].as_mut_ptr();

            let res = self.read_blocks_it(buf_ptr, block_address + (i * num_block) as u32, num_block as u32);
            if res != 0 {
                error!("read_blocks_it return [{}]", res);
                return Err(());
            }
            self.set_io_status(Self::IO_START);

            IO_REQS.lock().insert(
                IoRequest::new(READ_REQUEST, buf_ptr as usize + SIZE),
                (self.waker.clone(), self.io_status.clone())
            );

            self.wait().await;
        }
        drop(guard);

        Ok(())
    }

    async fn write(
            &mut self,
            block_address: u32,
            data: &[Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error> {
        let num_block = SIZE / 512;

        if data.len() == 0 {
            return Ok(());
        }

        let guard = SD_IO_LOCK.lock().await;
        for (i, buf) in data.iter().enumerate() {
            let buf_ptr = buf[..].as_ptr();

            let res = self.write_blocks_it(buf_ptr, block_address + (i * num_block) as u32, num_block as u32);
            if res != 0 {
                error!("write_blocks_it return [{}]", res);
                return Err(())
            }
            self.set_io_status(Self::IO_START);

            IO_REQS.lock().insert(
                IoRequest::new(WRITE_REQUEST, buf_ptr as usize + SIZE),
                (self.waker.clone(), self.io_status.clone())
            );

            self.wait().await;
        }
        drop(guard);

        Ok(())
    }

    async fn size(&mut self) -> Result<u64, Self::Error> {
        let cap = unsafe { get_sdcard_capacity() };
        Ok(cap)
    }
}



#[no_mangle]
pub extern "C" fn io_req_cplt_callback(req: u32, addr: usize, size: u32) {
    assert!(size == 0);
    assert!(req == READ_REQUEST || req == WRITE_REQUEST);

    let io_req = IoRequest::new(req, addr);
    let val = IO_REQS.lock().remove_entry(&io_req);
    if let Some((_io_req, (waker, io_status))) = val {
        if io_status.load(core::sync::atomic::Ordering::Acquire) == SdmmcIo::IO_WAITING {
            waker.wake();
        }
        io_status.store(SdmmcIo::IO_READY, core::sync::atomic::Ordering::Release);
    } else {
        panic!("invalid io request in sdmmc request complete callback");
    }
}

// pub(crate) async fn test_sdmmc_io() {
//     use crate::{debug, info, log};
//     use crate::driver::block_device_driver::BufStream;
//     use embedded_io_async::{Read, Write, Seek};

//     let mut sdmmc_io = SdmmcIo::new();

//     const TEST_ADDR: u32 = 0x00000000;
//     {
//         const LEN: usize = 512;
//         let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
//         let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

//         let write_result = sdmmc_io.write(TEST_ADDR, &write_data).await;
//         assert!(write_result.is_ok(), "Failed to write to SD card");

//         let read_result = sdmmc_io.read(TEST_ADDR, &mut read_data).await;
//         assert!(read_result.is_ok(), "Failed to read from SD card");

//         assert_eq!(read_data, write_data, "Data read does not match data written");
//         debug!("SD card read/write {} bytes block ........ Ok", LEN);
//     }
    

//     {
//         const LEN: usize = 1024;
//         let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
//         let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

//         let write_result = sdmmc_io.write(TEST_ADDR, &write_data).await;
//         assert!(write_result.is_ok(), "Failed to write to SD card");

//         let read_result = sdmmc_io.read(TEST_ADDR, &mut read_data).await;
//         assert!(read_result.is_ok(), "Failed to read from SD card");

//         assert_eq!(read_data, write_data, "Data read does not match data written");
//         debug!("SD card read/write {} bytes block ........ Ok", LEN);
//     }

//     {
//         const LEN: usize = 2048;
//         let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
//         let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

//         let write_result = sdmmc_io.write(TEST_ADDR, &write_data).await;
//         assert!(write_result.is_ok(), "Failed to write to SD card");

//         let read_result = sdmmc_io.read(TEST_ADDR, &mut read_data).await;
//         assert!(read_result.is_ok(), "Failed to read from SD card");

//         assert_eq!(read_data, write_data, "Data read does not match data written");
//         debug!("SD card read/write {} bytes block ........ Ok", LEN);
//     }

//     {
//         const LEN: usize = 4096;
//         let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
//         let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

//         let write_result = sdmmc_io.write(TEST_ADDR, &write_data).await;
//         assert!(write_result.is_ok(), "Failed to write to SD card");

//         let read_result = sdmmc_io.read(TEST_ADDR, &mut read_data).await;
//         assert!(read_result.is_ok(), "Failed to read from SD card");

//         assert_eq!(read_data, write_data, "Data read does not match data written");
//         debug!("SD card read/write {} bytes block ........ Ok", LEN);
//     }

//     info!("SD card read/write test passed");



//     let mut buf_stream = BufStream::<_, 512>::new(sdmmc_io);

//     {
//         const LEN: usize = 512;
//         let write_data = [0x51; LEN];
//         let mut read_data = [0u8; LEN];

//         let write_res = buf_stream.write(&write_data).await;
//         assert!(write_res.is_ok(), "Failed to write to BufStream");

//         assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

//         let read_res = buf_stream.read(&mut read_data).await;
//         assert!(read_res.is_ok(), "Failed to read from BufStream");

//         assert_eq!(write_data, read_data, "Data read does not match data written");
//         debug!("BufStream read/write {} bytes block ........ Ok", LEN);
//     }

//     {
//         const LEN: usize = 1024;
//         let write_data = [0x51; LEN];
//         let mut read_data = [0u8; LEN];

//         let write_res = buf_stream.write(&write_data).await;
//         assert!(write_res.is_ok(), "Failed to write to BufStream");

//         assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

//         let read_res = buf_stream.read(&mut read_data).await;
//         assert!(read_res.is_ok(), "Failed to read from BufStream");

//         assert_eq!(write_data, read_data, "Data read does not match data written");
//         debug!("BufStream read/write {} bytes block ........ Ok", LEN);
//     }

//     {
//         const LEN: usize = 2048;
//         let write_data = [0x51; LEN];
//         let mut read_data = [0u8; LEN];

//         let write_res = buf_stream.write(&write_data).await;
//         assert!(write_res.is_ok(), "Failed to write to BufStream");

//         assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

//         let read_res = buf_stream.read(&mut read_data).await;
//         assert!(read_res.is_ok(), "Failed to read from BufStream");

//         assert_eq!(write_data, read_data, "Data read does not match data written");
//         debug!("BufStream read/write {} bytes block ........ Ok", LEN);
//     }

//     {
//         const LEN: usize = 4096;
//         let write_data = [0x51; LEN];
//         let mut read_data = [0u8; LEN];

//         let write_res = buf_stream.write(&write_data).await;
//         assert!(write_res.is_ok(), "Failed to write to BufStream");

//         assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

//         let read_res = buf_stream.read(&mut read_data).await;
//         assert!(read_res.is_ok(), "Failed to read from BufStream");

//         assert_eq!(write_data, read_data, "Data read does not match data written");
//         debug!("BufStream read/write {} bytes block ........ Ok", LEN);
//     }

//     info!("BufStream read/write test passed");
// }