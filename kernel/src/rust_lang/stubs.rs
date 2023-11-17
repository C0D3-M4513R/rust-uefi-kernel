use core::panic::PanicInfo;

// #[lang = "eh_personality"]
// fn eh_personality() {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    for _ in 0..u32::MAX {
        x86_64::instructions::nop();
    }
    #[cfg(not(feature = "core_intrinsics"))]
    loop {
        #[cfg(target_arch="x86_64")]
        x86_64::instructions::hlt();
    }
    #[cfg(feature="core_intrinsics")]
    core::intrinsics::abort();
}
