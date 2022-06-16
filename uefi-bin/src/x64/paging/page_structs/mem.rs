use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use log::trace;
use uefi::table::boot::{AllocateType, MemoryType};
use crate::x64::paging::page_structs::{MemRegion, TOTAL_MEM};

//leave space for the flink ref
const SIZE:u64=4032;
const K4:usize=0xFFF;

//#[repr(packed)]
#[repr(align(4096))]
pub(super) struct MemTracer{
	mem:[AtomicBool;SIZE as usize],//flags used 4K blocks as true, when used/mapped
	flink:AtomicPtr<MemTracer>,
}
impl MemTracer{
	///Generates a new MemTracer, at address addr.
	///# Safety
	///addr MUST be valid for 4096 bits
	///addr MUST be valid for r/w access
	///addr MUST be aligned to 4096 bits, as defined by repr
	pub(super) unsafe fn new_addr(addr:*mut u8)->*mut Self{
		const FALSE:AtomicBool=AtomicBool::new(false);
		trace!("size of Atomic Bool: {}",core::mem::size_of_val(&FALSE) as u32 );
		let addr=addr as *mut MemTracer;
		*addr=MemTracer{mem:[FALSE;SIZE as usize],flink:AtomicPtr::new(core::ptr::null_mut())};
		// log::trace!("Wrote the Array, getting MemTracer.");
		addr
	}
	// ///addr must be a pointer, that is valid for r/w access for 4Kib.
	// fn new() ->Self{
	// 	const FALSE:AtomicBool=AtomicBool::new(false);
	// 	static MEM:[AtomicBool;SIZE as usize]=[FALSE;SIZE as usize];
	// 	MemTracer{
	// 		mem:&MEM,
	// 		flink:None,
	// 	}
	// }
	///Sets self.flink to Some(flink)
	pub(super) fn set_flink(&mut self, flink:*mut MemTracer){
		self.flink.store(flink, Ordering::SeqCst)
	}
	///Sets self.flink to None
	pub(super) fn unset_flink(&mut self){
		self.flink.store(core::ptr::null_mut(),Ordering::SeqCst)
	}
	
	fn find_free_page_unchecked(&self)->Option<u64>{
		for j in 0..SIZE as usize{
			if !self.mem[j].load(Ordering::SeqCst){
				let addr=(j as u64)<<12;
				self.mem[j].store(true,Ordering::SeqCst);
				return Some(addr);
			}
		}
		//Every bit in our address is True.
		//We have exhausted this array
		let flink=self.flink.load(Ordering::SeqCst);
		if !flink.is_null(){
			let flink=unsafe{&(*flink)};
			//We have another array, that we can check.
			let addr_flink=match flink.find_free_page_unchecked() {
				None=>return None,
				
				Some(v)=>v,
			};
			let addr=addr_flink+SIZE<<12;
			return Some(addr);
		}
		None
	}
	pub(super) fn mark_as_used(&self,addr:u64)->Option<()>{
		if addr&(K4 as u64)!=0{
			log::warn!("mark_as_used called with address {:#x?}. That is not aligned to 4096, and thus likely not a page address. Continuing anyways. {}",addr,addr&(K4 as u64));
		}
		self.mark_as_used_i(addr>>12)
	}
	///Marks a particular address as used.
	///An address here is address>>12.
	fn mark_as_used_i(&self,page_addr:u64)->Option<()>{
		// log::trace!("Marking page {:#x} as used",page_addr);
		if page_addr<SIZE{
			// log::trace!("Found correct MemTracer section");
			self.mem[page_addr as usize].store(true,Ordering::SeqCst);
			Some(())
		}else{
			let mut flink=self.flink.load(Ordering::SeqCst);
			if flink.is_null() {
				if let Some(st)=unsafe{&mut crate::ST}{
					let addr=st.boot_services().allocate_pages(AllocateType::AnyPages,MemoryType::LOADER_DATA,1);
					if addr.is_err(){
						log::error!("alloc is err");
					}
					let addr=addr.ok()?;
					unsafe {
						let mut mt=MemTracer::new_addr(addr as *mut u8);
						self.flink.store(mt, Ordering::SeqCst);
						flink=mt;
					}
				}else{
					return None;
				}
			}
			unsafe{
				(*flink).mark_as_used_i(page_addr-SIZE)
			}
		}
	}
	///Returns True, if the given page_address is in the main memory
	///Returns False, if the given page_address is larger, than the calculated total memory size.
	fn check_free_page(&self,page_addr:u64)->bool{
		//This is effectively converting the "page" address to a memory address
		if (page_addr<<12)>*TOTAL_MEM{
			false
		}else{
			true
		}
	}
	///This will return an address(>>12) of a free mempage, if it exists.
	///If no mempage exists it is guaranteed, that either no more available memory exists or that the mem array cannot hold any more.
	pub fn find_free_page(&self)->Option<u64>{
		let addr=self.find_free_page_unchecked()?;//Do we even have a single page, that could be used?
		if self.check_free_page(addr){//Does that page lie in the main memory?
			Some(addr)//Yes!
		}else {
			None//No. We have exhausted memory, but we have enough memory to represent everything
		}
	}
	///returns true if marking a page as "useable" was successful, else false
	///else could be, that that memory has not been allocated before
	pub(super) fn free_page(&self,addr:u64)->bool{
		//const IVKIB:u32=0xffff_ffff_ffff;//4096-1 in hex
		self.free_page_internal(addr>>12)
	}
	fn free_page_internal(&self, addr:u64)->bool{
		if addr&SIZE==addr{
			if self.mem[addr as usize].load(Ordering::SeqCst){
				log::error!("double free occurred? I will continue, but take like nothing happened, but take nothing as granted.");
			}
			self.mem[addr as usize].store(false,Ordering::SeqCst);
			true
		}else {
			let flink = self.flink.load(Ordering::SeqCst);
			if flink.is_null(){
				false
			}else {
				unsafe{
					(*flink).free_page_internal(addr-SIZE as u64)
				}
			}
		}
	}
	///Retuns true, if the entire region was marked as being useable again.
	///See [free_page].
	pub(super) fn free_region(& self,mem:MemRegion)->bool{
		let o=(0..mem.size).into_iter().map(|x|self.free_page(mem.start+(x as u64)<<12)).reduce(|x,y|x&&y).unwrap();
		o
	}
}