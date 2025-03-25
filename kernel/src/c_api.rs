#[allow(dead_code)]
unsafe extern "C" {
    pub unsafe fn led_twinkle(ms: u32);

    pub unsafe fn _putchar(ch: u8);

    pub unsafe fn enable_irq();

    pub unsafe fn disable_irq();

    pub unsafe fn led_toggle();

    pub unsafe fn enter_sleep_mode();

    pub unsafe fn sdmmc_read_blocks_it(buf: *mut u8, addr: u32, num: u32) -> i32;

    pub unsafe fn sdmmc_write_blocks_it(data: *const u8, addr: u32, num: u32) -> i32;

    pub unsafe fn get_sdcard_capacity() -> u64;
}