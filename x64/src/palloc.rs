pub trait PhysicalPageAllocator {
    fn allocate(&mut self) -> Option<x86_64::PhysAddr>;
    fn deallocate(&mut self, page: x86_64::PhysAddr);
}