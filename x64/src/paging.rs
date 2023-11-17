pub mod traits;
mod page_structs;
mod ptgetter;

use core::marker::PhantomData;
use core::ops::{Index, IndexMut};
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::{PageTable, PageTableIndex};
use crate::paging::traits::*;
use crate::palloc::PhysicalPageAllocator;

pub fn get_page_walker<'a>()->Option<Result<PageWalker<'a,Level5>,PageWalker<'a,Level4>>>{
	let (frame,_flags)=x86_64::registers::control::Cr3::read();
	let addr_p=frame.start_address().as_u64() as *mut PageTable;
	
	if addr_p.is_null(){return None};
	//The pointer is at least not null.
	//This does not necessarily mean, that this is a possible PageTable.
	let addr = unsafe{&mut *addr_p};
	if super::cpuid::pml5_avilable() {
		Some(Ok(PageWalker{addr,level:PhantomData::<Level5>}))
	} else {
		Some(Err(PageWalker{addr,level:PhantomData::<Level4>}))
	}
}
pub fn get_ptl4<'a>()->Option<PageWalker<'a,Level4>>{
	let mut p5;
	match get_page_walker(){
		None=>return None,
		Some(Ok(p))=>{
			p5=p;
			if cfg!(feature = "alloc") {
				return p5.create_pt(0).ok();
			}else{
				return p5.get_page(0).ok();
			}
		},
		Some(Err(p))=>return Some(p),
	}
}

pub struct PageWalker<'a,L:Level>{
	addr:&'a mut PageTable,
	level:PhantomData<L>,
}
impl<'a> PageWalker<'a,Level1>{
	#[cfg(feature = "alloc")]
	pub fn create_pt(&mut self,addr:u64)->Result<&mut PageTableEntry,u8>{
		Ok(&mut self.addr[((addr>>12)&512) as usize])
	}
}

impl<'a,L:LevelTable> PageWalker<'a,L>{
	pub fn get_page<'b>(&'b mut self,addr:u64)->Result<PageWalker<'a,L::Down>,(u8,&'b mut Self)>
	where 'a:'b{
		// |o:&'a PageWalker<'a,L>,addr:u64|->Result<&mut PageTableEntry,(u8,&'a PageWalker<'a,L>)>{
		self.index(((addr>>12>>(L::get_level().get_level()-1)*9)&512) as usize).ok_or((L::get_level().get_level(), self))
		// }
	}

	#[cfg(feature = "alloc")]
	pub fn create_pt(&mut self,addr:u64)->Result<PageWalker<L::Down>,u8>{
		self.get_page(addr)
			.or_else(
				|(l,s)|{
					page_structs::table::from_pt::<L>(s.addr).create_sub_table(((addr>>12>>(l-1)*9)&512) as u16);
					s.get_page(addr)
				}
			).map_err(|(l,_)|l)
	}
	
	///## Safety:
	/// L1 needs to be equal to L::Down
	fn index(&mut self, index: usize) -> Option<PageWalker<'a,L::Down>> {
		let pte=(*self.addr).index_mut(index);
		if pte.is_unused() || pte.addr().is_null(){
			None
		}else{
			//Safety:
			// Must be upheld by the one setting this value.
			// Stuff out of our control will happen, if this is not a PageTable.
			Some(PageWalker{addr:unsafe{&mut *(pte.addr().as_u64() as *mut PageTable)},level:PhantomData::<L::Down>})
		}
	}
	
	///Gets a potentially unused PageTableEntry
	fn index_unchecked(&mut self, index: PageTableIndex) -> &mut PageTableEntry {
		self.addr.index_mut(index as usize)
	}
}