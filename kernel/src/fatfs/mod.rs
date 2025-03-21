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


// lazy_static! {
//     pub(crate) static ref 
//     FS: Mutex<Option<FileSystem<BufStream<SdmmcIo, 512>, DefaultTimeProvider, LossyOemCpConverter>>> = Mutex::new(None);
// }

// pub(crate) async fn fs_init() {
//     let sdmmc_io = SdmmcIo::new();
//     let buf_stream = BufStream::<_, 512>::new(sdmmc_io);

//     if let Ok(fs) = FileSystem::new(buf_stream, FsOptions::new()).await {
//         *FS.lock() = Some(fs);
//     } else {
//         info!("formatting fatfs");
//         let sdmmc = SdmmcIo::new();
//         let mut buf_stream = BufStream::<_, 512>::new(sdmmc);

//         format_volume(&mut buf_stream, FormatVolumeOptions::default()).await.expect("format fatfs failed");

//         let fs = FileSystem::new(buf_stream, FsOptions::new()).await.expect("create fatfs failed");
//         *FS.lock() = Some(fs);
//     }

//     let fs_guard = FS.lock();
//     //info!("fs info: {:?}", fs_guard.as_ref().unwrap().stats().await);
//     let root = fs_guard.as_ref().unwrap().root_dir();
//     let mut iter = root.iter();
//     loop {
//         if let Some(Ok(entry)) = iter.next().await {
//             if entry.is_dir() {
//                 info!("Dir  name:{}", entry.file_name());
//             } else if entry.is_file() {
//                 let mut rbuf = [0u8; 512];

//                 let size = entry.to_file().read(&mut rbuf).await.expect("read file failed");
//                 let content = core::str::from_utf8(&rbuf[..size]).expect("utf8 error");
//                 info!("File  name:{}  content: {}", entry.file_name(), content);
//             } else {
//                 info!("Unknown type");
//             }
//         } else {
//             let ex = root.exists("Gungnir.txt").await.expect("exists failed");
//             if ex {
//                 let mut file = root.open_file("Gungnir.txt").await.expect("open file failed");
//                 if file.seek(embedded_io_async::SeekFrom::End(0)).await.expect("seek file failed") == 0 {
//                     info!("File  name:{}  content: empty", "Gungnir.txt");
//                     root.remove("Gungnir.txt").await.expect("remove file failed");
//                     info!("File  name:{}  removed", "Gungnir.txt");
//                 } else {
//                     let mut rbuf = [0u8; 512];
//                     let size = file.read(&mut rbuf).await.expect("read file failed");
//                     let content = core::str::from_utf8(&rbuf[..size]).expect("utf8 error");
//                     info!("File  name:{}  content: {}", "Gungnir.txt", content);
//                 }
//             } else {
//                 let mut file = root.create_file("Gungnir.txt").await.expect("create file failed");
//                 let content = "Gungnir is a legendary weapon in Norse mythology that is attested in the Poetic Edda, \
//                                      a collection of Old Norse poems from the Viking age. The weapon is a sword or spear, \
//                                      originally belonging to the god Odin. It is said to be so powerful that it can destroy \
//                                      entire armies and even kill gods.";
//                 info!("*****&&&&&");
//                 file.write(content.as_bytes()).await.expect("write file failed");
//                 file.flush().await.expect("flush file failed");
//                 info!("File  name:{}  content: {}", "Gungnir.txt", content);
//             }
//             info!("end");
//             break;
//         }
//     }
// }

pub(crate) async fn fs_init(tx1: futures_channel::oneshot::Sender<()>) {
    let sdmmc_io = SdmmcIo::new();
    let buf_stream = BufStream::<_, 512>::new(sdmmc_io);

    if let Err(_err) = FileSystem::new(buf_stream, FsOptions::new()).await {
        info!("formatting fatfs");
        let sdmmc_io = SdmmcIo::new();
        let mut buf_stream = BufStream::<_, 512>::new(sdmmc_io);
        format_volume(&mut buf_stream, FormatVolumeOptions::default()).await.expect("format fatfs failed");
        let _fs = FileSystem::new(buf_stream, FsOptions::new()).await.expect("create fatfs failed");
    } else {
        info!("fatfs already exists");
    }
    tx1.send(()).unwrap();
}

pub(crate) async fn fs_test1(rx1: futures_channel::oneshot::Receiver<()>, tx2: futures_channel::oneshot::Sender<()>) {
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
            info!("File  name:{}  content: {}", file_name, content);
        } else {
            let mut file = root.create_file(&file_name).await.expect("create file failed");
            let content = format!("This is (a) test file {}", i);
            file.write(content.as_bytes()).await.expect("write file failed");
            file.flush().await.expect("flush file failed");
            info!("File  name:{}  content: {}", file_name, content);
        }
    }
    tx2.send(()).unwrap();
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
            info!("File  name:{}  content: {}", file_name, content);
        } else {
            let mut file = root.create_file(&file_name).await.expect("create file failed");
            let content = format!("This is (b) test file {}", i);
            file.write(content.as_bytes()).await.expect("write file failed");
            file.flush().await.expect("flush file failed");
            info!("File  name:{}  content: {}", file_name, content);
        }
    } 
}