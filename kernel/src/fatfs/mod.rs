//copied from https://github.com/MabezDev/embedded-fatfs

#![allow(unused_imports, dead_code)]

use alloc::format;
use embedded_io_async::{Read, Seek, Write};
use fs::{format_volume, FileSystem, FormatVolumeOptions, FsOptions, LossyOemCpConverter};
use lazy_static::lazy_static;
use spin::Mutex;
use time::DefaultTimeProvider;

use crate::{driver::{block_device_driver::BufStream, sdmmc::SdmmcIo}, info, debug, log, println};

pub(crate) mod io;
pub(crate) mod error;
pub(crate) mod boot_sector;
pub(crate) mod dir_entry;
pub(crate) mod dir;
pub(crate) mod file;
pub(crate) mod fs;
pub(crate) mod table;
pub(crate) mod time;

pub(crate) async fn fs_init(tx1: futures_channel::oneshot::Sender<()>, tx2: futures_channel::oneshot::Sender<()>) {
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
    tx1.send(()).unwrap();
    tx2.send(()).unwrap();
}

pub(crate) async fn fs_test1(rx1: futures_channel::oneshot::Receiver<()>) {
    rx1.await.expect("rx1 failed");

    let sdmmc_io = SdmmcIo::new();
    let buf_stream = BufStream::<_, 512>::new(sdmmc_io);
    let fs = FileSystem::new(buf_stream, FsOptions::new()).await.expect("create fatfs failed");
    let root = fs.root_dir();

    for i in 1..=5 {
        let file_name = format!("a{}.txt", i);
        let exist = root.exists(&file_name).await.expect("exists failed");

        if exist {
            let mut file = root.open_file(&file_name).await.expect("open file failed");
            let mut rbuf = [0u8; 512];
            let size = file.read(&mut rbuf).await.expect("read file failed");
            let content = core::str::from_utf8(&rbuf[..size]).expect("utf8 error");
            debug!("File  name:{}  content: {}", file_name, content);
        } else {
            let mut file = root.create_file(&file_name).await.expect("create file failed");
            let content = format!("This is (a) test file {}", i);
            file.write(content.as_bytes()).await.expect("write file failed");
            file.flush().await.expect("flush file failed");
            debug!("File  name:{}  content: {}", file_name, content);
        }
    }
    info!("fs test 1 done");
}

pub(crate) async fn fs_test2(rx2: futures_channel::oneshot::Receiver<()>) {
    rx2.await.expect("rx2 failed");

    let sdmmc_io = SdmmcIo::new();
    let buf_stream = BufStream::<_, 512>::new(sdmmc_io);
    let fs = FileSystem::new(buf_stream, FsOptions::new()).await.expect("create fatfs failed");
    let root = fs.root_dir();

    for i in 1..=5 {
        let file_name = format!("b{}.txt", i);
        let exist = root.exists(&file_name).await.expect("exists failed");

        if exist {
            let mut file = root.open_file(&file_name).await.expect("open file failed");
            let mut rbuf = [0u8; 512];
            let size = file.read(&mut rbuf).await.expect("read file failed");
            let content = core::str::from_utf8(&rbuf[..size]).expect("utf8 error");
            debug!("File  name:{}  content: {}", file_name, content);
        } else {
            let mut file = root.create_file(&file_name).await.expect("create file failed");
            let content = format!("This is (b) test file {}", i);
            file.write(content.as_bytes()).await.expect("write file failed");
            file.flush().await.expect("flush file failed");
            debug!("File  name:{}  content: {}", file_name, content);
        }
    }
    info!("fs test 2 done");
}