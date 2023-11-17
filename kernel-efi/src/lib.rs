#![no_std]
pub use ttf_parser;
pub use uefi::proto::console::gop;
pub const KERNEL_ADDR:*mut u8=0x01FF_FFFF_0000_0000 as *mut u8;
pub const ARGS_ADDR:*mut Args=0x8000_0000 as *mut Args;


#[repr(C)]
pub struct Args<'b> {
	pub elf: MapElfRet,
	pub font: ttf_parser::Face<'b>,
	pub gop:GOP,
	pub heap_size:u64,
	pub page_tracker_base: *mut (),
	pub page_tracker_page_size: usize,
	pub page_table_entry: [*mut u64;3],
}

#[derive(Debug)]
#[repr(C)]
pub struct MapElfRet{
	pub base: *mut u8,
	pub pages: usize,
	pub entry_point: usize,
}

#[repr(C)]
//Invariant: Only RGB and BGR allowed
pub struct GOP{
	pub fb:FB,
	pub mode:uefi::proto::console::gop::Mode,
}

//todo: write own fb driver
pub struct FB{
	pub base: *mut u8,
	pub size: usize,
}