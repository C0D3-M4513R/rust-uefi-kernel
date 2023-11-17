use core::alloc::{GlobalAlloc, Layout};
use crate::lock::Lock;
use super::kpmalloc::KernelPhysicalMemoryAllocator;

struct GlobalAllocator{
    palloc:KernelPhysicalMemoryAllocator,
}

impl GlobalAllocator{
    pub fn new(base:*mut u64, pages:usize)->Self{
        Self{
            palloc:KernelPhysicalMemoryAllocator::new(base,pages),
        }
    }
}

unsafe impl GlobalAlloc for Lock<GlobalAllocator>{

    //allocates a non-initialized memory block of the given size and alignment.
    //This implementation only allocates pages, and stitches them together via the page table.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        //We do not need to worry about null-size layouts. It is undefined behavior to allocate them.
        if layout.size() == 0 {return core::ptr::null_mut();}
        let lock = self.lock();
        let alloc_phys_page = match lock.palloc.allocate(){
            None=>return core::ptr::null_mut(),
            Some(p)=>p,
        };
        let mut pw5;
        let pw_o = match match x64::paging::get_page_walker(){
            None=>alloc::alloc::handle_alloc_error(layout),
            Some(p)=>p,
        }{
            Ok(p)=>{
                pw5 = p;
                pw5.get_page(0)
            },Err(p)=>{
                p
            }
        };

    }


    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        todo!()
    }
}