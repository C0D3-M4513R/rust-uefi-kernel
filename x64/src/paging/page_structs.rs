use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
use core::cell::{Cell,RefCell};
use core::cmp::min;
use core::marker::PhantomData;
use core::ops::{Deref, Index, IndexMut};
use core::pin::Pin;
use core::ptr::{NonNull, null_mut};
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use log::log;
use x86_64::PhysAddr;
use x86_64::registers::control::Cr3Flags;
use x86_64::structures::paging::{FrameAllocator, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::structures::paging::page_table::PageTableEntry;
use crate::cpuid::pml5_avilable;
use crate::paging::page_structs::entry::PTEntry;
use crate::paging::page_structs::table::PageTable;
use crate::paging::traits::{Level, Level1, Level2, Level3, Level4, Level5};
use super::traits::{LevelTable,LevelEnum};

pub mod entry;
pub mod table;
pub mod mem;


struct MemRegion{
	start:u64,
	size:usize,
}
static mut TOTAL_MEM_M:u64 = 0;
static TOTAL_MEM:&u64=unsafe{&TOTAL_MEM_M };
/*
lazy_static::lazy_static!(
	static ref TOTAL_MEM:u64 = crate::efi::mem::get_mem().unwrap().1;
	static ref MEM_TRACER:AtomicPtr<mem::MemTracer>=unsafe{
		log::debug!("Init for MemTracer.");
		let mt=crate::ST.as_ref().unwrap().boot_services().allocate_pages(AllocateType::AnyPages,MemoryType::LOADER_DATA,1).unwrap();
		log::debug!("Got Page For MemTracer.");
		AtomicPtr::new(mem::MemTracer::new_addr(mt as *mut u8))
	};
	static ref PAGE_TABLES:table::PageTable<Level4>=unsafe{
		log::debug!("Init for PAGE_TABLES.");
		let pt=crate::ST.as_ref().unwrap().boot_services().allocate_pages(AllocateType::AnyPages,MemoryType::LOADER_DATA,1).unwrap();
		log::debug!("Got Page For PAGE_TABLES.");
		table::PageTable::<Level4>::new_addr(pt as *mut u8)
	};
	// static ref MEM_TRACER:MemTracer=MemTracer::new();
);
#[global_allocator]
static mut ALLOCATOR:Allocate<MEM_TRACER,PAGE_TABLES>=Allocate{mem:&MEM_TRACER,pt:&PAGE_TABLES };

pub fn id_map()->Result<(),()>{
	log::trace!("Identity Mapping the first 4G of Memory");
	let p4e = &(*PAGE_TABLES)[0];
	if p4e.is_unused(){
		(*PAGE_TABLES).create_sub_table(0)?;
	}
	log::trace!("P4");
	// let p4=unsafe{& *(p5e.addr().as_u64() as *mut table::PageTable<Level4>)};
	// log::trace!("P4");
	// let p4e=&p4[0];
	// if p4e.is_unused(){
	// 	log::trace!("Creating P3 table 0");
	// 	p4.create_sub_table(0)?;
	// }
	let p3=unsafe{& *(p4e.addr().as_u64() as *mut table::PageTable<Level3>)};
	log::trace!("P3");
	for i in 0..3{
		let p3e=&p3[i];
		if p3e.is_unused(){
			log::trace!("Creating P2 table {}",i);
			p3.create_sub_table(i as u16)?;
		}
		let p2=unsafe{& *(p3e.addr().as_u64() as *mut table::PageTable<Level2>)};
		log::warn!("P2 {}",i);
		for j in 0..512{
			let p2e=&p2[j];
			if p2e.is_unused(){
				log::trace!("Creating P1 Table");
				p2.create_sub_table(j as u16)?;
			}
			let p1=unsafe{& *(p2e.addr().as_u64() as *mut table::PageTable<Level1>)};
			log::trace!("P1 {},{}",i,j);
			//unsafe{asm!("int 3",options(nomem,nostack,preserves_flags))};
			for k in 0..512{
				let p1e=&p1[k];
				let addr=(i as u64)<<12<<2*9|(j as u64)<<12<<9|(k as u64)<<12;
				// log::trace!("Marking as used");
				let mau=unsafe{(*(*MEM_TRACER).load(Ordering::Acquire)).mark_as_used(addr)}.ok_or(());
				match mau {
					Ok(_)=>{},
					Err(_)=>{log::error!("mark as used error!");return Err(())},
				}
				// log::trace!("Marking as used success");
				p1e.set_addr(PhysAddr::new(addr),PageTableFlags::PRESENT|PageTableFlags::WRITABLE);
			}
		}
	}
	log::trace!("ID Map successful.");
	Ok(())
}

pub fn load_page_table(){
	log::trace!("Loading Page Map");
	unsafe{
		x86_64::registers::control::Cr3::write(
			PhysFrame::containing_address(
				PhysAddr::new_truncate((*PAGE_TABLES).get_addr() as u64)
			),
			Cr3Flags::empty()
		)
		
	}
	log::trace!("Loaded Page Map")
}

struct Allocate<MT:Deref<Target=AtomicPtr<mem::MemTracer>>+'static,PT:Deref<Target=PageTable<Level4>>+'static>{
	mem:&'static MT,
	pt:&'static PT,
}
impl<MT:Deref<Target=AtomicPtr<mem::MemTracer>>+'static,PT:Deref<Target=PageTable<Level4>>+'static> Allocate<MT,PT>{
	const fn new(mem:&'static MT,pt:&'static PT)->Self{
		Allocate{ mem,pt }
	}
	
	fn find_mem_region(&self, layout:Layout) ->Result<MemRegion,()>{
		let align=layout.align().trailing_zeros() as u16;
		let mut size=layout.size();
		log::debug!("Finding free memory-region with alignment 2**{}, and {} pages",align,size>>12);
		let mut step = [0u16;5];
		if align<12{
			//This case means, that the allocator just requires some alignment within a page.
			//We don't really care, since we will only be giving out full pages.
			//We can give out full pages in that case, since a page is always aligned at 4Kib.
		}else if align<(12+9) {
			//we need to give out some page in the page table layer 1
			let align=align-12;
			step[0]=1<<align as u16;
		}else if align<(12+2*9){
			//we need to give out some page in the page table layer 2
			let align=align-12-1*9;
			step[1]=1<<align as u16;
		}else if align<(12+3*9){
			//we need to give out some page in the page table layer 3
			let align=align-12-2*9;
			step[2]=1<<align as u16;
		}else if align<(12+4*9){
			//we need to give out some page in the page table layer 4
			let align=align-12-3*9;
			step[3]=1<<align as u16;
		}/*else if align<(12+5*9){
			//we need to give out some page in the page table layer 5
			let align=align-12-4*9;
			step[4]=1<<align as u16;
		}*/ else{
			return Err(());//the align address is past the virtual address space
		}
		//counter
		let mut i = [0u16;4];
		const MAX_INDEX:u16=0x1ff;
		log::trace!("Set step values, to get a memory at the right alingment: {:#x?}",step);
		let orig_step=step;
		let mut addr = u64::MAX;
		//should we change the addr?
		//also indicates, if we are just searching for a memory location(true),
		//or just for a free, big enough chunk of memory (false) and already have an address
		let mut addr_write=true;
		while /*i[4]<512 &&*/ i[3]<512 && i[2]<512 && i[1]<512 && i[0]<512 && size>0{
			/*
			let p5e =&(*self.pt)[i[4] as usize];
			if p5e.is_unused(){
				(*self.pt).create_sub_table(i[4])?;
				addr=(i[4] as u64)<<12<<4*9;
			}else if step[3]==0 && step[2]==0 && step[1]==0 && step[0]==0 {
				continue;
				//there is no way, that we can reach alignment needs here.
				//find another memory location.
			}
			let p4=unsafe{& *(p5e.addr().as_u64() as *mut table::PageTable<Level4>)};
			*/
			let p4e = &(*self.pt)[i[3] as usize];
			// let p4e=&p4[i[3] as usize];
			if p4e.is_unused(){
				(*self.pt).create_sub_table(i[3])?;
				//if we require some alignment in the p4, we still keep it here, because then step[3] would be 0
				addr|=(i[3] as u64)<<12<<3*9;
			}else if step[2]==0 && step[1]==0 && step[0]==0{
				continue;
				//same reason as above
			}
			let p3=unsafe{& *(p4e.addr().as_u64() as *mut table::PageTable<Level3>)};
			let p3e=&p3[i[2] as usize];
			if p3e.is_unused(){
				p3.create_sub_table(i[2])?;
				addr|=(i[2] as u64)<<12<<2*9;
			}else if step[1]==0 && step[0]==0 {
				continue;
				//same reason as above
			}
			let p2 = unsafe{& *(p3e.addr().as_u64() as *mut table::PageTable<Level2>)};
			let p2e=&p2[i[1] as usize];
			if p2e.is_unused(){
				p2.create_sub_table(i[1])?;
				addr|=(i[1] as u64)<<12<<1*9;
			}else if step[0]==0 {
				continue;
				//same reason as above
			}
			let p1 = unsafe{& *(p2e.addr().as_u64() as *mut table::PageTable<Level1>)};
			let p1e = &p1[i[0] as usize];
			//we want to now look, if we find a space big enough.
			//we have accounted for memory alignment above.
			//If we find a stretch of unused memory pages, that is good.
			//If we don't, we go back to finding another memory location, that fits the alignment needs.
			if p1e.is_unused(){
				//we want to look for continuous free/unused pages of size `size` (since t)
				step=[1,0,0,0,0];
				//set the address, and proceed, to go into searching, if we can alloc size bytes here.
				if addr_write{
					addr|=i[0] as u64;
					addr_write=false;
				}
				log::trace!("Found empty p1 page at the location {:#x?}. Need {} more consecutive pages",addr,size);
				let phy_addr=match unsafe{(*(**self.mem).load(Ordering::Acquire)).find_free_page()}{
					None=>return Err(()),
					Some(v)=>v,
				};
				p1e.set_addr(
					PhysAddr::new_truncate(phy_addr),
					PageTableFlags::PRESENT|PageTableFlags::WRITABLE
				);//we do not really care about which memory gets put in the virtual address.
				addr|=(i[0] as u64)<<12;
				size=size.checked_sub(4096).unwrap_or(0);
			}else{
				log::info!("Would need {} more pages",size>>12);
				//we did not find a region of acceptable size.
				//reset, and search for an acceptable region again.
				step=orig_step;//make sure the alignment is right.
				addr_write=true;//we want to search for another address again
				addr=u64::MAX;//we do not currently have an address
				size=layout.size();//the size needs to be reset again.
			}
			//increment counters
			{
				//search for the next page
				//i[4]+=step[4];
				i[3]+=step[3];
				i[2]+=step[2];
				i[1]+=step[1];
				i[0]+=step[0];
				//Make sure everything (except i[4], which will be our termination cause) is in between 0 and 511.
				//i[4]+=i[3]>>9;
				i[3]+=i[3]>>9+i[1]>>9;
				//i[3]=i[3]&MAX_INDEX+i[2]>>9;
				i[2]=i[2]&MAX_INDEX+i[1]>>9;
				i[1]=i[1]&MAX_INDEX+i[0]>>9;
				i[0]=i[0]&MAX_INDEX;
			}
		}
		if !addr_write || addr==u64::MAX{
			Err(())
			//the while loop did not find any memory suitable of the required alignment or size.
			//todo: we should do something against memory fragmentation sometime.
		}else {
			Ok(MemRegion{start:addr,size:layout.size()})
		}
	}
}
unsafe impl<MT:Deref<Target=AtomicPtr<mem::MemTracer>>+'static,PT:Deref<Target=PageTable<Level4>>+'static> GlobalAlloc for Allocate<MT,PT>{
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		return match self.find_mem_region(layout) {
			Err(_) => core::ptr::null_mut(),
			Ok(v) => v.start as *mut u8,
		}
	}
	
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		log::debug!("Request to dealloc memory at addr {:#x?}",ptr);
		let mut size=layout.size();
		if size>4096{
			size=size>>12;
		}else{
			size=1;
		}
		let mut addr=ptr as u64>>12;
		const MOD:usize=0x1ff;
		while size>0{
			let p5e=(*self.pt).index((addr>>4*9) as usize&MOD);
			if p5e.is_unused(){
				//todo: is this useless?
				handle_alloc_error(layout);//we cannot dealloc something here?
			}
			let old_addr=addr;
			while (addr>>4*9) as usize &MOD == (old_addr>>4*9)as usize &MOD && size>0{
				let p4 = unsafe{&mut *(p5e.addr().as_u64() as *mut table::PageTable<Level4>)};
				let p4e=p4.index((addr>>3*9) as usize&MOD);
				if p4e.is_unused(){
					//todo: is this useless?
					handle_alloc_error(layout);//we cannot dealloc something here?
				}
				let old_addr=addr;
				while (addr>>3*9) as usize&MOD == (old_addr>>3*9) as usize&MOD && size>0{
					let p3 = unsafe{&mut *(p4e.addr().as_u64() as *mut table::PageTable<Level3>)};
					let p3e=p3.index((addr>>2*9) as usize&MOD);
					if p3e.is_unused(){
						//todo: is this useless?
						handle_alloc_error(layout);//we cannot dealloc something here?
					}
					let old_addr=addr;
					while (addr>>2*9) as usize&MOD == (old_addr>>2*9) as usize&MOD && size>0{
						let p2 = unsafe{&mut *(p3e.addr().as_u64() as *mut table::PageTable<Level2>)};
						let p2e=p2.index((addr>>9) as usize);
						if p2e.is_unused(){
							//todo: is this useless?
							handle_alloc_error(layout);//we cannot dealloc something here?
						}
						let old_addr=addr;
						while (addr>>9) as usize&MOD==(old_addr>>9) as usize&MOD && size>0{//if we stumble into a new page, this should catch it.
							let p1 = unsafe{&mut *(p2e.addr().as_u64() as *mut table::PageTable<Level1>)};
							let p1e=p1.index(addr as usize&MOD);
							if p1e.is_unused(){
								//todo: is this useless?
								log::error!("Duplicate dealloc found at addr {:#x?}",ptr);
								handle_alloc_error(layout);//we cannot dealloc something here?
							}else{
								p1e.set_unused();
							}
							size-=1;
							addr+=1;
						}
					}
				}
			}
		}
		
		(*(**self.mem).load(Ordering::Acquire)).free_region(MemRegion{start:addr,size});
	}
}
*/