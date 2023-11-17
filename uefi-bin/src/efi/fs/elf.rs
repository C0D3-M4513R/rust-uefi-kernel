use elf_rs::{Elf, ElfFile, ProgramHeaderFlags};
use uefi::table::boot::{AllocateType, MemoryType};
use x86_64::{align_down, align_up};
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::PageTableFlags;
use x64::paging::PageWalker;
use x64::paging::traits::Level4;
use kernel_efi::MapElfRet;
use uefi::Result;

///maps a elf file to memory. Returns a base address, and the offset to the entry_point.
///Page table
pub fn map_elf(file:&[u8],addr:*mut u8)->Result<MapElfRet>{
	let elf=Elf::from_bytes(file).map_err(|_|uefi::Status::COMPROMISED_DATA)?;
	let mut i=0;
	let mut size=0u64;
	//this gets the totally needed size
	loop{
		if let Some(ph)=elf.program_header_nth(i){
			if ph.ph_type()==elf_rs::ProgramType::LOAD{
				let align=ph.align();
				let base_addr=x86_64::align_down(ph.vaddr(),align);//This should be the base addr
				size+=x86_64::align_up(base_addr,align);//This should be the whole memory region, that needs to be mapped.
			}
		}else{
			break;
		}
		i+=1;
	}
	let page_num=(size>>12) as usize;
	let mem=unsafe{uefi_services::system_table().as_ref()}.boot_services().allocate_pages(AllocateType::AnyPages,MemoryType::LOADER_DATA,page_num)?;
	let mem=mem as *mut u8;
	{//copy data
		let file = file as *const [u8] as *const u8;
		i=0;
		loop{
			if let Some(ph)=elf.program_header_nth(i){
				if ph.ph_type()==elf_rs::ProgramType::LOAD{
					let membase = unsafe{mem.add(ph.vaddr() as usize)};
					unsafe{core::ptr::copy(file.add(ph.offset() as usize),membase,ph.filesz() as usize)}
					let diff=ph.memsz()-ph.filesz();
					if diff>0{
						unsafe {core::ptr::write_bytes(mem.add(ph.filesz() as usize),0,diff as usize)};
					}
				}
			}else{
				break;
			}
			i+=1;
		}
	}
	
	set_pt_attr(&elf,mem,addr);
	let entry_point=elf.entry_point() as usize + addr as usize;
	Ok(MapElfRet{base:mem,pages:page_num,entry_point})
}

#[inline]
fn set_pt_attr(elf:&Elf,mem:*mut u8,addr:*mut u8){
	let mut i=0;
	loop{
		if let Some(ph)=elf.program_header_nth(i){
			if ph.ph_type()==elf_rs::ProgramType::LOAD{
				let align=ph.align();
				let membase = align_down(mem as u64 + ph.vaddr(),align);
				let memmax = align_up(mem as u64 + ph.vaddr() + ph.memsz(),align);
				let addrbase = align_down(addr as u64 +ph.vaddr(),align);
				let addrmax = align_up(addr as u64 + ph.vaddr() + ph.memsz(),align);
				
				get_phy_pg(membase,memmax,addrbase,addrmax,align,ph.flags()).unwrap();
			}
		}else{
			break;
		}
		i+=1;
	}
}

pub fn get_pte<F, O>(pw: &mut PageWalker<Level4>, addr:u64, f:F) -> core::result::Result<O, u8>
where F:FnOnce(&mut PageTableEntry)->O{
	let mut tmp=pw.create_pt(addr)?;
	let mut tmp=tmp.create_pt(addr)?;
	let mut tmp=tmp.create_pt(addr)?;
	let mut tmp=tmp.create_pt(addr)?;
	Ok(f(&mut tmp))
}

fn get_phy_pg(membase:u64,memmax:u64,addrbase:u64,addrmax:u64,_align:u64,flags:ProgramHeaderFlags)->Option<()>{
	
	//I assume here, that the kernel is not going to cover more than 512GiB (which is the size of a ptl5 entry).
	let mut pw5;
	let pw=match x64::paging::get_page_walker()?{
		Ok(v)=>{pw5=v;pw5.create_pt(addrbase)},
		Err(v)=>Ok(v),
	};
	let mut pw=pw.unwrap();
	assert_eq!(memmax-membase,addrmax-addrbase);
	
	let flag={
		let mut flag=PageTableFlags::PRESENT;
		if !flags.contains(ProgramHeaderFlags::EXECUTE){
			flag|=PageTableFlags::NO_EXECUTE;
		}
		if flags.contains(ProgramHeaderFlags::WRITE){
			flag|=PageTableFlags::WRITABLE;
		}
		flag
	};
	
	for i in 0..(memmax-membase)>>12{
		//get addr of mem region
		let im=membase+(i<<12);
		let addr = get_pte(&mut pw,im,|pe|{let addr = pe.addr(); pe.set_unused(); addr}).ok()?;
		//map that to addr region
		let ia = addrbase+(i<<12);
		get_pte(&mut pw,ia,|pe|pe.set_addr(addr,flag)).ok()?;
	}
	Some(())
}