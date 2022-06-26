pub mod traits;
mod page_structs;

use core::any::{Any, TypeId};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::PageTable;
use crate::paging::traits::*;

pub fn get_page_walker<'a>()->Option<Result<PageWalker<'a,Level5>,PageWalker<'a,Level4>>>{
	let (frame,flags)=x86_64::registers::control::Cr3::read();
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
		self.index::<L::Down>(((addr>>12>>(L::get_level().get_Level()-1)*9)&512) as usize).ok_or((L::get_level().get_Level(),self))
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
			).map_err(|(l,s)|l)
	}
	
	///## Safety:
	/// L1 needs to be equal to L::Down
	fn index<L1:Level>(&mut self, index: usize) -> Option<PageWalker<'a,L1>> {
		let pte=unsafe{(*self.addr).index_mut(index)};
		if pte.is_unused() || pte.addr().is_null(){
			None
		}else{
			Some(PageWalker{addr:unsafe{&mut *(pte.addr().as_u64() as *mut PageTable)},level:PhantomData::<L1>})
		}
	}
}