use core::panic::PanicInfo;

// #[lang = "eh_personality"]
// fn eh_personality() {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        #[cfg(target_arch="x86_64")]
        x86_64::instructions::hlt();
    }
}
