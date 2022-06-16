/*
For this file:
https://github.com/rust-osdev/x86_64/blob/master/src/structures/paging/page_table.rs
https://github.com/rust-osdev/x86_64/blob/master/LICENSE-MIT
The MIT License (MIT)

Copyright (c) 2018 Philipp Oppermann
Copyright (c) 2015 Gerd Zellweger
Copyright (c) 2015 The libcpu Developers

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
 */
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::PhysAddr;
use x86_64::structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::structures::paging::page_table::FrameError;

//#[derive(Clone)]
#[repr(transparent)]
pub struct PTEntry{
	entry:AtomicU64
}
impl PTEntry{
	/// Creates an unused page table entry.
	#[inline]
	pub const fn new() -> Self {
		PTEntry { entry: AtomicU64::new(0) }
	}
	
	/// Returns whether this entry is zero.
	#[inline]
	pub fn is_unused(&self) -> bool {
		self.entry.load(Ordering::SeqCst)==0
	}
	
	/// Sets this entry to zero.
	#[inline]
	pub fn set_unused(&self) {
		self.entry.store(0,Ordering::SeqCst);
	}
	
	/// Returns the flags of this entry.
	#[inline]
	pub fn flags(&self) -> PageTableFlags {
		PageTableFlags::from_bits_truncate(self.entry.load(Ordering::SeqCst))
	}
	
	/// Returns the physical address mapped by this entry, might be zero.
	#[inline]
	pub fn addr(&self) -> PhysAddr {
		PhysAddr::new(self.entry.load(Ordering::SeqCst) & 0x000f_ffff_ffff_f000)
	}
	
	/// Returns the physical frame mapped by this entry.
	///
	/// Returns the following errors:
	///
	/// - `FrameError::FrameNotPresent` if the entry doesn't have the `PRESENT` flag set.
	/// - `FrameError::HugeFrame` if the entry has the `HUGE_PAGE` flag set (for huge pages the
	///    `addr` function must be used)
	#[inline]
	pub fn frame(&self) -> Result<PhysFrame, FrameError> {
		if !self.flags().contains(PageTableFlags::PRESENT) {
			Err(FrameError::FrameNotPresent)
		} else if self.flags().contains(PageTableFlags::HUGE_PAGE) {
			Err(FrameError::HugeFrame)
		} else {
			Ok(PhysFrame::containing_address(self.addr()))
		}
	}
	
	/// Map the entry to the specified physical address with the specified flags.
	#[inline]
	pub fn set_addr(&self, addr: PhysAddr, flags: PageTableFlags) {
		assert!(addr.is_aligned(Size4KiB::SIZE));
		self.entry.store(addr.as_u64() | flags.bits(),Ordering::SeqCst);
	}
	
	/// Map the entry to the specified physical frame with the specified flags.
	#[inline]
	pub fn set_frame(&self, frame: PhysFrame, flags: PageTableFlags) {
		assert!(!flags.contains(PageTableFlags::HUGE_PAGE));
		self.set_addr(frame.start_address(), flags)
	}
	
	/// Sets the flags of this entry.
	#[inline]
	pub fn set_flags(&self, flags: PageTableFlags) {
		self.entry.store(self.addr().as_u64() | flags.bits(),Ordering::SeqCst);
	}
}
impl core::fmt::Debug for PTEntry {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		let mut f = f.debug_struct("PageTableEntry");
		f.field("addr", &self.addr());
		f.field("flags", &self.flags());
		f.finish()
	}
}