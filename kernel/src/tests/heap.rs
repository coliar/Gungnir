use alloc::{boxed::Box, format, string::{String, ToString}};

use crate::debug;

pub(crate) fn heap_test(heap_start: *mut u8, heap_size: usize) -> Result<(), usize> {
    let mut heap_value = Box::new(34);
    *heap_value = 1234;
    assert_eq!(*heap_value, 1234);

    let addr: String = format!("{:p}", heap_value);
    let addr: usize = usize::from_str_radix(&addr[2..], 16).expect("parse addr");
    
    if addr >= (heap_start as usize) && addr < (heap_start as usize) + heap_size {
        debug!("heap string: {}", message());
        Ok(())
    } else {
        Err(addr)
    }
}

fn message() -> String {
    let part1 = "rust".to_string();
    let part2 = " heap".to_string();
    let part3 = String::from(" allocator!");
    format!("{}{}{}", part1, part2, part3)
}