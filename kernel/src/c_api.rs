#[allow(dead_code)]
extern "C" {
    pub fn led_twinkle(ms: u32);

    pub fn _putchar(ch: u8);

    pub fn enable_irq();

    pub fn disable_irq();

    pub fn led_toggle();

    pub fn enter_sleep_mode();

    pub fn sdmmc_read_blocks_it(buf: *mut u8, addr: u32, num: u32) -> i32;

    pub fn sdmmc_write_blocks_it(data: *const u8, addr: u32, num: u32) -> i32;

    pub fn get_sdcard_capacity() -> u64;

    pub fn get_RxCplt() -> u8;

    pub fn set_RxCplt(val: u8);

    pub fn get_TxCplt() -> u8;

    pub fn set_TxCplt(val: u8);

    pub fn get_ticks() -> u64;
}