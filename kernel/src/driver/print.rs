use core::fmt;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::c_api::_putchar;

pub struct Writer;

impl Writer {
    #[allow(dead_code)]
    unsafe fn write_byte(&mut self, byte: u8) {
        _putchar(byte);
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            unsafe { _putchar(byte); }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    static ref WRITER: Mutex<Writer> = Mutex::new(Writer);
}


#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::driver::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub(crate) fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}