use core::borrow::Borrow;
use core::fmt::Debug;
use uefi::Error;
use uefi::{CStr16, Status};
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, RegularFile};
use uefi::table::boot::{AllocateType, MemoryType, SearchType};
use uefi::Identify;

pub mod elf;

pub const KERNEL_NAME:&str="kernel";
#[allow(non_upper_case_globals)]
const KiB4:usize=0xFFF;

pub fn get_file(name:&str) ->uefi::Result<RegularFile> {
	let st = unsafe { uefi_services::system_table().as_ref() };
	let handles = st.boot_services().locate_handle_buffer(SearchType::ByProtocol(&uefi::proto::media::fs::SimpleFileSystem::GUID))?;
	log::info!("Got {} handles.",handles.len());
	for h in &*handles {
		if let Ok(mut fs) = st.boot_services().get_image_file_system(*h) {
			log::info!("Got a file system");
			if let Ok(mut d) = fs.open_volume() {
				log::info!("Opened the Volume");
				
				let buf = st.boot_services().allocate_pool(MemoryType::LOADER_DATA, (name.len() + 1) * core::mem::size_of::<u16>())?;
				let buf_s = unsafe { core::slice::from_raw_parts_mut(buf as *mut u16, name.len() + 1) };
				
				if let Ok(kernel_name_c16) = uefi::CStr16::from_str_with_buf(name, buf_s) {
					log::info!("Encoded the filename");
					if let Ok(item) = d.open(kernel_name_c16, FileMode::Read, FileAttribute::empty()) {
						log::info!("Got a Entry in the fs.");
						st.boot_services().free_pool(buf)?;
						if let Some(file) = item.into_regular_file() {
							log::info!("The Entry in the fs was a file. We can proceed, to load it.");
							return Ok(file);
						}
					}
				}
				st.boot_services().free_pool(buf)?;
			}
		}
	}
	Err(Status::NO_MEDIA.into())
}
//The static lifetime is fine. We
//alloc mem, but we NEVER dealloc it here.
//the caller has to deallocate, if applicable
pub fn load_file(name:&str)->uefi::Result<&'static [u8]>{
	let mut file = get_file(name)?;
	//get file size
	let info=file.get_boxed_info::<FileInfo>()?;
	let file_size=info.file_size();
	//alloc the amount of pages needed. (maybe a little more, if the data is EXACTLY a multiple of 4KiB big)
	let pages=file_size>>12 + 1;
	let mem=unsafe{uefi_services::system_table().as_ref()}.boot_services().allocate_pages(AllocateType::AnyPages,MemoryType::LOADER_DATA,pages as usize)?;
	//init contents
	unsafe{core::ptr::write_bytes(mem as *mut u8,0,(pages as usize)*KiB4)};
	//now read as many times, as we need to.
	let mut size=0;
	log::debug!("Prepared everything for reading Kernel ELF File");
	while size<file_size{//we have not yet read enough bytes. try to read more.
		//we need to adjust the buffer everytime we read, since we read everytime at a different file offset.
		let buf=unsafe{core::slice::from_raw_parts_mut((mem+size) as *mut u8,(pages*KiB4 as u64-size) as usize)};
		//actually read
		let size_n =file.read(buf).map_err(|x|Error::new(x.status(),()))?;
		//update size, so we eventually terminate
		size+=size_n as u64;
	}
	//Construct the memory view, of the fully loaded file
	let mem_buf = unsafe{core::slice::from_raw_parts(mem as *mut u8,pages as usize*KiB4)};
	Ok(mem_buf)
}