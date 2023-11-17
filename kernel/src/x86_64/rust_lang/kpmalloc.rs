use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::PhysAddr;

pub struct KernelPhysicalMemoryAllocator{
    allocs:&'static mut [SplitPhysicalAllocator],
}

impl KernelPhysicalMemoryAllocator{
    pub const fn new(base:*mut u64,pages:usize) -> Self{
        return Self{
            allocs: unsafe{core::slice::from_raw_parts_mut(base as *mut SplitPhysicalAllocator,pages/64)},
        }
    }
}

impl PhysicalMemoryAllocator for KernelPhysicalMemoryAllocator{
    fn allocate(&mut self) -> Option<x86_64::PhysAddr> {
        //allocate the first page possible
        let (i,page) = self.allocs.iter_mut()
            .enumerate()
            .map(|(i,x)|(i,x.alloc()))
            .filter_map(|(i,x)|match x{
                None=>None,
                Some(p)=>Some((i,p))
            }).next()?;

        let addr = x86_64::PhysAddr::new((i*64+page) as u64 * 4096 * 64);
        return Some(addr);
    }

    fn deallocate(&mut self, addr:PhysAddr){
        let mut index = (addr.as_u64() / 4096 / 64) as usize;
        let mut page = (addr.as_u64() / 4096 % 64) as u8;
        unsafe{self.allocs.get_unchecked_mut(index)}.dealloc(page);
    }
}

#[repr(transparent)]
struct SplitPhysicalAllocator{
    pages:u64,
}

impl SplitPhysicalAllocator{
    fn new()->Self{
        return Self{
            pages:0,
        }
    }
    pub fn is_full(&self)->bool{
        return self.pages == u64::MAX;
    }
    fn get_first_free_page(&self)->Option<u8>{
        let ones = self.pages.leading_ones();
        if ones<64{
            return Some(ones as u8);
        }
        return None;
    }
    pub fn alloc(&mut self) -> Option<u8>{
        let ones = self.pages.leading_ones();
        if ones<64{
            self.pages |= 1u64<<ones;//mark as used
            return Some(ones as u8);
        }
        return None;
    }
    pub fn dealloc(&mut self, page:u8){
        self.pages &= !(1u64<<page);
    }
}