//copied from https://github.com/MabezDev/embedded-fatfs

#![allow(unused_imports)]

use alloc::format;
use embedded_io_async::{Read, Seek, Write};
use fs::{format_volume, FileSystem, FormatVolumeOptions, FsOptions, LossyOemCpConverter};
use lazy_static::lazy_static;
use spin::Mutex;
use time::DefaultTimeProvider;

use crate::{driver::{block_device_driver::BufStream, sdmmc::SdmmcIo}, info, log, println};

pub(crate) mod io;
pub(crate) mod error;
pub(crate) mod boot_sector;
pub(crate) mod dir_entry;
pub(crate) mod dir;
pub(crate) mod file;
pub(crate) mod fs;
pub(crate) mod table;
pub(crate) mod time;

pub(crate) async fn fs_init() {
    let sdmmc_io = SdmmcIo::new();
    let buf_stream = BufStream::<_, 512>::new(sdmmc_io);

    if let Err(_err) = FileSystem::new(buf_stream, FsOptions::new()).await {
        info!("formatting fatfs");
        let sdmmc_io = SdmmcIo::new();
        let mut buf_stream = BufStream::<_, 512>::new(sdmmc_io);
        format_volume(&mut buf_stream, FormatVolumeOptions::default()).await.expect("format fatfs failed");
        let _fs = FileSystem::new(buf_stream, FsOptions::new()).await.expect("create fatfs failed");
    } else {
        info!("fatfs already existed");
    }
}