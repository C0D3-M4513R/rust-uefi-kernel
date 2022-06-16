/*
Large parts of this File were Copied from there:
https://github.com/rust-osdev/x86_64/blob/master/src/structures/paging/page_table.rs

The Files in that repo are licenced under MIT or APACHE-2.0
A copy of the MIT License of the repo can be found below:

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
use core::iter::{Chain, Cloned, Copied, Cycle, Enumerate, Filter, FilterMap, FlatMap, Flatten, Fuse, FusedIterator, Inspect, Map, MapWhile, Peekable, Product, Rev, Scan, Skip, SkipWhile, StepBy, Sum, Take, TakeWhile, Zip};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};
use core::sync::atomic::{AtomicPtr, compiler_fence, Ordering};
use uefi::table::boot::{AllocateType, MemoryType};
use x86_64::PhysAddr;
use x86_64::structures::paging::{PageTableFlags, PageTableIndex};
use crate::x64::paging::traits::{Level, LevelTable};
use super::entry::PTEntry;

const ENTRY_COUNT:usize=512;
/// Represents a page table.
///
/// Always page-sized.
///
/// This struct implements the `Index` and `IndexMut` traits, so the entries can be accessed
/// through index operations. For example, `page_table[15]` returns the 15th page table entry.
///
/// Note that while this type implements [`Clone`], the users must be careful not to introduce
/// mutable aliasing by using the cloned page tables.
#[repr(align(4096))]
#[repr(C)]
//#[derive(Clone)]
pub struct PageTable<L:Level> {
	entries: AtomicPtr<[PTEntry; ENTRY_COUNT]>,
	phantom: PhantomData<L>
}

impl<L:LevelTable> PageTable<L>{
	#[inline]
	pub(super) fn get_addr(&self)->*const (){
		self.entries.load(Ordering::SeqCst) as *const [PTEntry;ENTRY_COUNT] as *const ()
	}
	pub(super) fn create_sub_table(&self, index:u16) ->Result<(),()>{
		log::trace!("Creating sub table from Level:{} and index:{:x}",L::get_level().get_Level(),index);
		if let Some(sts)=unsafe{&crate::ST}{
			log::trace!("We are still in a UEFI environment. We can proceed, to simply use the UEFI allocator.");
			let a = sts.boot_services().allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1);
			if a.is_err(){
				log::error!("Could not create subtable. The UEFI-allocator said no.");
				return Err(())
			}
			log::trace!("We got an allocation. Proceeding to create a PageTable at that address.");
			let a = a.unwrap();
			unsafe {
				PageTable::<L>::new_addr(a as *mut u8);
			}
			{
				self[index as usize]
					.set_addr(
						PhysAddr::new(a),
						PageTableFlags::PRESENT|PageTableFlags::WRITABLE
					);
			}
		}else{
			assert!(false,"No allocator present, to populate page-table. I have yet to write code for this case.")
		}
		Ok(())
	}
}

impl<L:Level> PageTable<L> {
	///Generates a new MemTracer, at address addr.
	///# Safety
	///addr MUST be valid for 4096 bits
	///addr MUST be valid for r/w access
	///addr MUST be aligned to 4096 bits, as defined by repr
	pub(super) unsafe fn new_addr(addr:*mut u8)->Self{
		const ENTRY:PTEntry=PTEntry::new();
		let mut addr=addr as *mut PTEntry as *mut [PTEntry;ENTRY_COUNT];
		core::ptr::write(addr,[ENTRY;ENTRY_COUNT]);
		let mem=AtomicPtr::new(addr);
		log::trace!("Wrote the Array, returning PageTable Struct");
		PageTable{
			entries:mem,
			phantom:PhantomData::<L>,
		}
	}
	
	/// Clears all entries.
	#[inline]
	pub fn zero(&mut self) {
		for entry in 0..ENTRY_COUNT {
			unsafe{(*(self.entries.load(Ordering::SeqCst) as *const PTEntry).add(entry)).set_unused()};
		}
	}
	
	// /// Returns an iterator over the entries of the page table.
	// #[inline]
	// pub fn iter(&self) -> impl Iterator<Item = &PTEntry> {
	// 	self.entries.iter()
	// }
}

impl<L:Level> Index<usize> for PageTable<L> {
	type Output = PTEntry;
	
	#[inline]
	fn index(&self, index: usize) -> &Self::Output {
		assert!(index<512,"Array bounds not satisfied. Requested index {}, but max index is 512",index);
		unsafe{&*(self.entries.load(Ordering::SeqCst) as *const PTEntry).add(index)}
	}
}

impl<L:Level> Index<PageTableIndex> for PageTable<L> {
	type Output = PTEntry;
	
	#[inline]
	fn index(&self, index: PageTableIndex) -> &Self::Output {
		unsafe{&*(self.entries.load(Ordering::SeqCst)as *const PTEntry).add(index.into())}
	}
}

impl<L:Level> core::fmt::Debug for PageTable<L> {
	#[inline]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		for i in 0..ENTRY_COUNT{
			unsafe{(*(self.entries.load(Ordering::SeqCst) as *const PTEntry).add(i)).fmt(f)?}
		}
		Ok(())
	}
}
impl<L:Level> IntoIterator for PageTable<L>{
	type Item = *const PTEntry;
	type IntoIter = PageTableIter;
	
	fn into_iter(self) -> Self::IntoIter {
		PageTableIter{
			ptr:self.entries.load(Ordering::SeqCst) as *const PTEntry,
			count:0,
			count_rev:0,
		}
	}
}

pub struct PageTableIter{
	ptr:*const PTEntry,
	count:usize,
	count_rev:usize,
}
impl Iterator for PageTableIter{
	type Item = *const PTEntry;
	
	fn next(&mut self) -> Option<Self::Item> {
		if self.count>=ENTRY_COUNT-self.count_rev {
			None
		}else {
			self.count+=1;
			Some(unsafe{ self.ptr.add(self.count-1)})
		}
	}
	
	fn size_hint(&self) -> (usize, Option<usize>) {
		(ENTRY_COUNT-self.count-self.count_rev, Some(ENTRY_COUNT-self.count-self.count_rev))
	}
}
impl ExactSizeIterator for PageTableIter{}
impl FusedIterator for PageTableIter{}
impl DoubleEndedIterator for PageTableIter{
	fn next_back(&mut self) -> Option<Self::Item> {
		if self.count_rev >=ENTRY_COUNT-self.count{
			None
		}else{
			self.count_rev +=1;
			Some(unsafe{self.ptr.add(ENTRY_COUNT-self.count_rev -1)})
		}
	}
}
