#![no_std]
#![no_main]
#![cfg_attr(feature="core_intrinsics",core_intrinsics)]

use kernel_efi::Args;

mod rust_lang;
mod uefi_rs;
mod fb;
mod x86_64;
mod lock;

extern crate alloc;

#[no_mangle]
fn _start() {
	let args=unsafe{core::ptr::read_volatile(kernel_efi::ARGS_ADDR)};
	let fb:fb::FB<'static,'static,4>={
		//We are reading memory outside th scope of this binary.
		//No assumptions should be made about the contents of ARGS_ADDR

		let ph =args.font.height().abs() as usize;
		fb::FB{
			args,
			ph,
		}
	};
	
	loop{
		x86_64::instructions::hlt();
	}
}

//Alloc Page Table in a single ptl4 entry