//#![feature(lang_items)]
#![no_std]
#![no_main]

mod rust_lang;

#[no_mangle]
fn _start() {
	x86_64::instructions::nop();
	loop{
		x86_64::instructions::hlt();
	}
}
