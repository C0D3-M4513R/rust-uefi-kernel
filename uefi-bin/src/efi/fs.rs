use core::borrow::Borrow;
use core::fmt::Debug;
use uefi::CStr16;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, RegularFile};
use uefi::table::boot::{AllocateType, MemoryType};

pub mod elf;

const EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: uefi::Guid = uefi::Guid::from_values(0x0964e5b22, 0x6459, 0x11d2, 0x8e39,0x00a0c969723b);
const KERNEL_NAME:&str="kernel";
const KiB4:usize=0xFFF;

fn get_kernel_file()->Result<RegularFile,()>{
	let st=unsafe{uefi_services::system_table().as_ref()};
	let handles=st.boot_services().find_handles::<uefi::proto::media::fs::SimpleFileSystem>().map_err(|_|())?;
	log::info!("Got {} handles.",handles.len());
	for h in handles{
		if let Ok(fs)=st.boot_services().get_image_file_system(h){
			log::info!("Got a file system");
			let sfs = unsafe{&mut *fs.interface.get()};
			
			if let Ok(mut d)=sfs.open_volume(){
				log::info!("Opened the Volume");
				
				let mut buf=[0u16;KERNEL_NAME.len()+1];
				if let Ok(kernel_name_c16) = uefi::CStr16::from_str_with_buf(KERNEL_NAME,&mut buf){
					log::info!("Encoded the filename");
					if let Ok(item) = d.open(kernel_name_c16,FileMode::Read,FileAttribute::empty()){
						log::info!("Got a Entry in the fs.");
						if let Some(file) = item.into_regular_file(){
							log::info!("The Entry in the fs was a file. We can proceed, to load it.");
							return Ok(file);
						}
					}
				}
			}
		}
	}
	Err(())
}
//The static lifetime is fine. We alloc mem, but we NEVER dealloc it here.
pub fn load_kernel_file()->Result<&'static [u8],()>{
	let mut file =get_kernel_file()?;
	//get file size
	let info=file.get_boxed_info::<FileInfo>().map_err(|_|())?;
	let file_size=info.file_size();
	//alloc the amount of pages needed.
	let pages=file_size>>12 + if file_size&(KiB4 as u64)!=0 {1} else {0};
	let mem=unsafe{uefi_services::system_table().as_ref()}.boot_services().allocate_pages(AllocateType::AnyPages,MemoryType::LOADER_DATA,pages as usize).map_err(|_|())?;
	//init contents
	unsafe{core::ptr::write_bytes(mem as *mut u8,0,(pages as usize)*KiB4)};
	//now read as many times, as we need to.
	let mut size=0;
	log::debug!("Prepared everything for reading Kernel ELF File");
	while size<file_size{//we have not yet read enough bytes. try to read more.
		//we need to adjust the buffer everytime we read, since we read everytime at a different file offset.
		let buf=unsafe{core::slice::from_raw_parts_mut((mem+size) as *mut u8,(pages*KiB4 as u64-size) as usize)};
		//actually read
		let size_n =file.read(buf).map_err(|_|())?;
		//update size, so we eventually terminate
		size+=size_n as u64;
	}
	//Construct the memory view, of the fully loaded file
	let mem_buf = unsafe{core::slice::from_raw_parts(mem as *mut u8,pages as usize*KiB4)};
	Ok(mem_buf)
}