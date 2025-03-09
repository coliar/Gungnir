#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use allocator::LockedHeap;
use task::executor::Executor;

#[allow(unused_imports)]
use alloc::{boxed::Box, sync::Arc};


mod c_api;
mod allocator;
mod task;
mod gsh;
mod fatfs;

#[macro_use]
mod driver;

#[macro_use]
mod log;

#[cfg(feature = "test_features")]
mod tests;



#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[no_mangle]
pub extern "Rust" fn __rust_alloc_error_handler(_size: usize, _align: usize) -> ! {
    error!("alloc error");
    loop {
        unsafe {
            crate::c_api::led_twinkle(1000);
        }
    }
}

#[no_mangle]
static __rust_no_alloc_shim_is_unstable: u8 = 0;


// panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {
        unsafe {
            crate::c_api::led_twinkle(500);
        }
    }
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn kernel_main(sdram_start: *mut u8, sdram_size: usize) -> ! {
    println!("kernel is powered by Rust");

    unsafe {
        ALLOCATOR.lock().init(sdram_start, sdram_size);
    }
    #[cfg(feature = "heap_test")]
    {
        tests::heap_test(sdram_start, sdram_size).expect("kernle heap test");
        info!("kernel heap was inited");
    }

    let executor = Arc::new(Executor::new());
    
    executor.spawn(fatfs::fs_init());

    executor.spawn(gsh::gshell(executor.clone()));

    executor.run();
}
