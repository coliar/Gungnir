//copied from https://github.com/MabezDev/embedded-fatfs

#![allow(unused_imports)]

use embedded_io_async::{Read, Write};
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


lazy_static! {
    pub(crate) static ref 
    FS: Mutex<Option<FileSystem<BufStream<SdmmcIo, 512>, DefaultTimeProvider, LossyOemCpConverter>>> = Mutex::new(None);
}

pub(crate) async fn fs_init() {
    let sdmmc = SdmmcIo::new();
    let buf_stream = BufStream::<_, 512>::new(sdmmc);

    if let Ok(fs) = FileSystem::new(buf_stream, FsOptions::new()).await {
        *FS.lock() = Some(fs);
    } else {
        info!("formatting fatfs");
        let sdmmc = SdmmcIo::new();
        let mut buf_stream = BufStream::<_, 512>::new(sdmmc);

        format_volume(&mut buf_stream, FormatVolumeOptions::default()).await.expect("format fatfs failed");

        let fs = FileSystem::new(buf_stream, FsOptions::new()).await.expect("create fatfs failed");
        *FS.lock() = Some(fs);
    }

    let fs_guard = FS.lock();
    let root = fs_guard.as_ref().unwrap().root_dir();
    let mut iter = root.iter();
    loop {
        if let Some(Ok(entry)) = iter.next().await {
            if entry.is_dir() {
                info!("Dir  name:{}", entry.file_name());
            } else if entry.is_file() {
                let mut rbuf = [0u8; 512];

                let size = entry.to_file().read(&mut rbuf).await.expect("read file failed");
                let content = core::str::from_utf8(&rbuf[..size]).expect("utf8 error");
                info!("File  name:{}  content: {}", entry.file_name(), content);
            } else {
                info!("Unknown type");
            }
        } else {
            info!("end");
            break;
        }
    }
}