#![allow(unused_imports)]

use core::future::Future;

use embedded_io_async::{Read, Seek, Write};
use futures_util::task::AtomicWaker;
use aligned::{Aligned, A4};

use crate::{
    c_api::{get_RxCplt, get_TxCplt, get_sdcard_capacity, sdmmc_read_blocks_it, sdmmc_write_blocks_it, set_RxCplt, set_TxCplt},
    println
};

use super::block_device_driver::{BlockDevice, BufStream};

static SDMMC_WAKER: AtomicWaker = AtomicWaker::new();

struct SdmmcReader {
    _private: (),
}

impl Future for SdmmcReader {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let rx_cplt = unsafe { get_RxCplt() };
        if rx_cplt == 0 {
            SDMMC_WAKER.register(&cx.waker());
            unsafe { set_RxCplt(2); }
            core::task::Poll::Pending
        } else {
            core::task::Poll::Ready(())
        }
    }
}

#[no_mangle]
pub extern "C" fn wake_sdmmc_reader() {
    SDMMC_WAKER.wake();
}

struct SdmmcWriter {
    _private: (),
}

impl Future for SdmmcWriter {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let tx_cplt = unsafe { get_TxCplt() };
        if tx_cplt == 0 {
            SDMMC_WAKER.register(&cx.waker());
            unsafe { set_TxCplt(2); }
            core::task::Poll::Pending
        } else {
            core::task::Poll::Ready(())
        }
    }
}

#[no_mangle]
pub extern "C" fn wake_sdmmc_writer() {
    SDMMC_WAKER.wake();
}

pub(crate) struct Sdmmc {
    reader: SdmmcReader,
    writer: SdmmcWriter,
}

impl Sdmmc {
    pub(crate) fn new() -> Self {
        Sdmmc {
            reader: SdmmcReader { _private: () },
            writer: SdmmcWriter { _private: () },
        }
    }
}

impl<const SIZE: usize> BlockDevice<SIZE> for Sdmmc {
    type Align = A4;
    type Error = ();

    async fn read(
            &mut self,
            block_address: u32,
            data: &mut [Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error> {
        if data.len() == 0 {
            return Ok(());
        }
        let num_block = SIZE / 512;
        for (i, buf) in data.iter_mut().enumerate() {
            let read_res = unsafe {
                set_RxCplt(0);
                sdmmc_read_blocks_it(buf[..].as_mut_ptr(), block_address + (i * num_block) as u32, num_block as u32)
            };
            if read_res != 0 {
                return Err(());
            }
            (&mut self.reader).await;
        }

        Ok(())
    }

    async fn write(
            &mut self,
            block_address: u32,
            data: &[Aligned<Self::Align, [u8; SIZE]>],
    ) -> Result<(), Self::Error> {
        if data.len() == 0 {
            return Ok(());
        }
        let num_block = SIZE / 512;
        for (i, buf) in data.iter().enumerate() {
            let write_res = unsafe {
                set_TxCplt(0);
                sdmmc_write_blocks_it(buf[..].as_ptr(), block_address + (i * num_block) as u32, num_block as u32)
            };
            if write_res != 0 {
                return Err(());
            }
            (&mut self.writer).await;
        }

        Ok(())
    }

    async fn size(&mut self) -> Result<u64, Self::Error> {
        let cap = unsafe { get_sdcard_capacity() };
        Ok(cap)
    }
}

#[cfg(feature = "sdmmc_test")]
pub(crate) async fn test_sdmmc_read_write() {
    use crate::{debug, info, log};

    let mut sdmmc = Sdmmc {
        reader: SdmmcReader { _private: () },
        writer: SdmmcWriter { _private: () },
    };

    const TEST_ADDR: u32 = 0x00000000;
    {
        const LEN: usize = 512;
        let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
        let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

        let write_result = sdmmc.write(TEST_ADDR, &write_data).await;
        assert!(write_result.is_ok(), "Failed to write to SD card");

        let read_result = sdmmc.read(TEST_ADDR, &mut read_data).await;
        assert!(read_result.is_ok(), "Failed to read from SD card");

        assert_eq!(read_data, write_data, "Data read does not match data written");
        debug!("SD card read/write {} bytes block ........ Ok", LEN);
    }
    

    {
        const LEN: usize = 1024;
        let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
        let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

        let write_result = sdmmc.write(TEST_ADDR, &write_data).await;
        assert!(write_result.is_ok(), "Failed to write to SD card");

        let read_result = sdmmc.read(TEST_ADDR, &mut read_data).await;
        assert!(read_result.is_ok(), "Failed to read from SD card");

        assert_eq!(read_data, write_data, "Data read does not match data written");
        debug!("SD card read/write {} bytes block ........ Ok", LEN);
    }

    {
        const LEN: usize = 2048;
        let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
        let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

        let write_result = sdmmc.write(TEST_ADDR, &write_data).await;
        assert!(write_result.is_ok(), "Failed to write to SD card");

        let read_result = sdmmc.read(TEST_ADDR, &mut read_data).await;
        assert!(read_result.is_ok(), "Failed to read from SD card");

        assert_eq!(read_data, write_data, "Data read does not match data written");
        debug!("SD card read/write {} bytes block ........ Ok", LEN);
    }

    {
        const LEN: usize = 4096;
        let mut read_data = [Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN]), Aligned([0; LEN])];
        let write_data = [Aligned([0x42; LEN]), Aligned([0x43; LEN]), Aligned([0x44; LEN]), Aligned([0x45; LEN])];

        let write_result = sdmmc.write(TEST_ADDR, &write_data).await;
        assert!(write_result.is_ok(), "Failed to write to SD card");

        let read_result = sdmmc.read(TEST_ADDR, &mut read_data).await;
        assert!(read_result.is_ok(), "Failed to read from SD card");

        assert_eq!(read_data, write_data, "Data read does not match data written");
        debug!("SD card read/write {} bytes block ........ Ok", LEN);
    }

    info!("SD card read/write test passed");



    let mut buf_stream = BufStream::<_, 512>::new(sdmmc);

    {
        const LEN: usize = 512;
        let write_data = [0x51; LEN];
        let mut read_data = [0u8; LEN];

        let write_res = buf_stream.write(&write_data).await;
        assert!(write_res.is_ok(), "Failed to write to BufStream");

        assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

        let read_res = buf_stream.read(&mut read_data).await;
        assert!(read_res.is_ok(), "Failed to read from BufStream");

        assert_eq!(write_data, read_data, "Data read does not match data written");
        debug!("BufStream read/write {} bytes block ........ Ok", LEN);
    }

    {
        const LEN: usize = 1024;
        let write_data = [0x51; LEN];
        let mut read_data = [0u8; LEN];

        let write_res = buf_stream.write(&write_data).await;
        assert!(write_res.is_ok(), "Failed to write to BufStream");

        assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

        let read_res = buf_stream.read(&mut read_data).await;
        assert!(read_res.is_ok(), "Failed to read from BufStream");

        assert_eq!(write_data, read_data, "Data read does not match data written");
        debug!("BufStream read/write {} bytes block ........ Ok", LEN);
    }

    {
        const LEN: usize = 2048;
        let write_data = [0x51; LEN];
        let mut read_data = [0u8; LEN];

        let write_res = buf_stream.write(&write_data).await;
        assert!(write_res.is_ok(), "Failed to write to BufStream");

        assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

        let read_res = buf_stream.read(&mut read_data).await;
        assert!(read_res.is_ok(), "Failed to read from BufStream");

        assert_eq!(write_data, read_data, "Data read does not match data written");
        debug!("BufStream read/write {} bytes block ........ Ok", LEN);
    }

    {
        const LEN: usize = 4096;
        let write_data = [0x51; LEN];
        let mut read_data = [0u8; LEN];

        let write_res = buf_stream.write(&write_data).await;
        assert!(write_res.is_ok(), "Failed to write to BufStream");

        assert!(buf_stream.seek(embedded_io_async::SeekFrom::Start(0)).await.is_ok(), "BufStream faile to seek");

        let read_res = buf_stream.read(&mut read_data).await;
        assert!(read_res.is_ok(), "Failed to read from BufStream");

        assert_eq!(write_data, read_data, "Data read does not match data written");
        debug!("BufStream read/write {} bytes block ........ Ok", LEN);
    }

    info!("BufStream read/write test passed");
}