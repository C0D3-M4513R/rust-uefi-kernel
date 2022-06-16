use core::iter::{Copied, Filter};
use uefi::table::boot::{MemoryDescriptor, MemoryType};

pub fn get_mem() ->Option<(impl Iterator<Item=MemoryDescriptor>,u64)>{
	log::info!("Trying to get Memory-Map");
	let st=unsafe{crate::ST.as_ref()?};
	let boot=st.boot_services();
	let mem_size = boot.memory_map_size();
	log::trace!("Got Memory-Map size");
	//manually alloc space for use (since we might not yet have a allocator)
	let size=mem_size.map_size+mem_size.entry_size*128;
	let mut s = boot.allocate_pool(MemoryType::LOADER_DATA,size).ok()?;
	log::trace!("Allocated memory from uefi for Memory-Map");
	//make it accessible in a useful format
	let buf=unsafe{
		//init values (since that is a requirement of from_raw_parts_mut)
		boot.set_mem(s,size,0);
		//construct a slice over the addr
		core::slice::from_raw_parts_mut(s,size)
	};
	let (mmk,mmt) = boot.memory_map(buf).expect("Allocated way to many bytes, but still couldn't store memory map.");
	log::trace!("Got Memory-Map Iterator");
	//mm = mmt.copied().collect();
	//free the memory. we don't need it anymore, since we copied the memory map.
	boot.free_pool(s).ok()?;
	log::trace!("Free'd memory from uefi");
	let mem = mmt.copied().filter(get_type);//.expect("No regular memory exists in the system?");
	let total_mem:u64 = mem.clone().map(|x|x.page_count).sum();
	Some((mem,total_mem))
}
fn get_type(x:&MemoryDescriptor)->bool{
	x.ty==MemoryType::CONVENTIONAL
}