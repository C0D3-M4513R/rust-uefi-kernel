use core::marker::PhantomData;
use core::ops::Index;
use x86_64::PhysAddr;
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::{PageTableFlags};
use crate::paging::{get_page_walker, get_ptl4, PageWalker};
use crate::paging::page_structs::entry::PTEntry;
use crate::paging::page_structs::table::PageTable;
use crate::paging::traits::*;
use crate::palloc::PhysicalPageAllocator;

pub struct PageTableIndex<L:Level>{
    index:x86_64::structures::paging::PageTableIndex,
    ptl:PageTable<L>
}

impl<L:Level> PageTableIndex<L> {
    pub fn new_high(pt:PageTable<L>) -> Self {
        Self{
            index:x86_64::structures::paging::PageTableIndex::new(511),
            ptl:pt
        }
    }

    pub fn new(pt:PageTable<L>,i:x86_64::structures::paging::PageTableIndex) -> Self{
        Self{
            index:i,
            ptl:pt
        }
    }

    pub fn get_ptl(&self) -> &PageTable<L>{
        &self.ptl
    }

    pub fn get_index(&self) -> x86_64::structures::paging::PageTableIndex{
        self.index
    }

    fn get_dec_index(&mut self) -> x86_64::structures::paging::PageTableIndex{
        let index = self.index;
        self.index = x86_64::structures::paging::PageTableIndex::new(index.as_usize()-1);
        index
    }

    fn dec_get_index(&mut self) -> x86_64::structures::paging::PageTableIndex{
        self.index = x86_64::structures::paging::PageTableIndex::new(self.index.as_usize()-1);
        self.index
    }

    pub fn get_new_page_table_entry(&mut self) -> &PageTableEntry{
        &self.ptl[self.dec_get_index()] as &u64 as &PageTableEntry
    }

    pub fn get_page_table_entry(&mut self) -> &PageTableEntry{
        &self.ptl[self.get_index()] as &u64 as &PageTableEntry
    }
    pub fn get_free_pages_lim_self(&self, limit:u64) -> u64{
        let mut free = 0;
        for i in 0..=self.index as u16{
            if free >= limit {return free;}
            free += self.ptl[i as usize].is_unused();
        }
        free
    }
}

impl<L:LevelTable> PageTableIndex<L> {
    pub fn get_next_level(&mut self) -> PageTableIndex<L::Down>{
        let pt_page_entry = self.get_page_table_entry();
        let pt = PageTable::new(pt_page_entry as *mut PageTableEntry as *mut x86_64::structures::paging::PageTable, PhantomData::<L::Down>);
        PageTableIndex::new_high(pt)
    }

    fn get_next_level_index(&self, index:x86_64::structures::paging::PageTableIndex)->PageTableIndex<L::Down>{
        let pt = PageTable::new(
            &self.ptl[index as usize] as &u64 as &PageTableEntry as *mut PageTableEntry as *mut x86_64::structures::paging::PageTable,
            PhantomData::<L::Down>
        );

        PageTableIndex::new_high(pt)
    }

    fn get_free_pages_lim_f(&self, limit:u64, f:impl FnOnce(PageTableIndex<L::Down>)->u64) -> u64 {
        let mut free = 0;
        for i in 0..=(index as u16) {
            if free >= limit {
                return free;
            }
            free += if self.ptl.index(i).is_unused() {
                 512u64.pow((L::get_level().get_level() - 1) as u32)
            }else{
                f(self.get_next_level_index(x86_64::structures::paging::PageTableIndex::new(i)))
            };
        }
        free
    }
}
impl PageTableIndex<Level2> {
    pub fn get_free_pages_lim(&self, limit:u64) -> u64{
        self.get_free_pages_lim_f(limit, |ptl4| ptl4.get_free_pages_lim_self(512,))
    }
}
impl PageTableIndex<Level3> {
    pub fn get_free_pages_lim(&self, limit:u64) -> u64{
        self.get_free_pages_lim_f(limit, |ptl4| ptl4.get_free_pages_lim(512,))
    }
}
impl PageTableIndex<Level4> {
    pub fn get_free_pages_lim(&self, limit:u64) -> u64{
        self.get_free_pages_lim_f(limit, |ptl4| ptl4.get_free_pages_lim(512,))
    }
}
impl PageTableIndex<Level5> {
    pub fn get_free_pages_lim(&self, limit:u64) -> u64{
        self.get_free_pages_lim_f(limit, |ptl4| ptl4.get_free_pages_lim(512,))
    }
}

///This struct makes VERY STRONG ASSUMPTIONS:
/// - There are no Pages allocated from the end of the address space
///Otherwise, it might run into a scenario, where it can't allocate a page table and thus cannot return a page table entry
///We however, do not check for this here!
pub struct LinearPageTableGetter<PPA: PhysicalPageAllocator>{bv
    palloc:PPA,
    ptl4:PageTableIndex<Level4>,
    ptl3:PageTableIndex<Level3>,
    ptl2:PageTableIndex<Level2>,
    ptl1:PageTableIndex<Level1>,
    ptl4_used:bool,
    ptl3_used:bool,
    ptl2_used:bool,
    ptl1_used:bool,
}

impl<PPA: PhysicalPageAllocator> LinearPageTableGetter<PPA> {
    ///Returns none, if no page table is available
    pub fn new(mut buffer:[*mut PageTableEntry;4], mut palloc:PPA) -> Option<Self> {
        let ptl5;
        let ptl4;
        match get_page_walker()? {
            Ok(ptl5w) =>{
                let ptl5_t = PageTable::new(ptl5w.addr, PhantomData::<Level5>);
                let ptl4_index = x86_64::structures::paging::PageTableIndex::new(511);
                let ptl4_table = get_pt(&ptl5_t, ptl4_index, &mut buffer, &mut palloc);
                ptl4 = PageTableIndex::new(ptl4_table, ptl4_index);
                ptl5 = Some(PageTableIndex::new_high(ptl5_t));
            },
            Err(ptl4w) => {
                ptl5 = None;
                ptl4 = PageTableIndex::new_high(PageTable::new(ptl4w.addr, PhantomData::<Level4>));
            }
        }
        let mut plt4w = get_ptl4().expect("No Page Table Level 4 available");
        let mut ptl4:PageTableIndex<Level4> = PageTableIndex::new_high(PageTable::new(plt4w.addr, PhantomData::<Level4>));
        let ptl3_index = x86_64::structures::paging::PageTableIndex::new(511);
        let ptl3_used = !ptl4.get_ptl().index(ptl3_index).is_unused();
        let mut ptl3_entry:PageTable<Level3> = get_pt(&ptl4, plt3_index, &mut buffer, &mut palloc);
        let ptl3:PageTableIndex<Level3> = PageTableIndex::new(ptl3_entry, plt3_index);
        let ptl2_index = x86_64::structures::paging::PageTableIndex::new(511);
        let ptl2_used = !ptl3.get_ptl().index(ptl2_index).is_unused();
        let mut ptl2_entry:PageTable<Level2> = get_pt(&ptl3, plt2_index, &mut buffer, &mut palloc);
        let ptl2:PageTableIndex<Level2> = PageTableIndex::new(ptl2_entry, plt2_index);
        let ptl1_index = x86_64::structures::paging::PageTableIndex::new(511);
        let ptl1_used = !ptl2.get_ptl().index(ptl1_index).is_unused();
        let mut ptl1_entry:PageTable<Level1> = get_pt(&ptl2, plt1_index, &mut buffer, &mut palloc);
        let ptl1:PageTableIndex<Level1> = PageTableIndex::new(ptl1_entry, plt1_index);
        Some(Self{
            palloc,
            ptl5,
            ptl4,
            ptl3,
            ptl2,
            ptl1,
            ptl4_used,
            ptl3_used,
            ptl2_used,
            ptl1_used,
        })
    }

    pub fn get_page(&mut self) -> *const PageTableEntry {
        let mut free_pages4 = self.ptl4.get_free_pages_lim(4);
        let mut free_pages3 = self.ptl3.get_free_pages_lim(4);
        let mut free_pages2 = self.ptl2.get_free_pages_lim(4);
        let mut free_pages1 = self.ptl1.get_free_pages_lim_self(4);
        let mut alloc = false;
        if let Some(&mut ptl5) = self.ptl5 {
            if self.ptl4.get_index() == 0 && self.ptl3.get_index() == 0 && self.ptl2.get_index() == 0 && free_pages <= 4 {
                let ptl4_page = ptl5.get_new_page_table_entry();
                self.ptl4 = PageTableIndex::new_high(get_pt(ptl5.get_ptl(), ptl5.get_index(), &mut [ptl4_page as *mut PageTableEntry,  core], &mut self.palloc));
                self.ptl4_used = !ptl4_page.is_unused();
                alloc = true;
            }
        }
        if alloc {
            self.ptl3 = self.ptl4.get_next_level();
            self.ptl3_used = !self.ptl4.get_page_table_entry().is_unused();
        }
        let ptl3_page = self.ptl4.get_page_table_entry();
        if ptl3_page.is_unused() {
            self.ptl3 = PageTableIndex::new(alloc_pt::<Level3>(ptl3_page as *mut PageTableEntry, &mut [ptl3_page as *mut PageTableEntry,  core], &mut self.palloc), self.ptl4.get_index());
            self.ptl3_used = true;
        }

        if self.ptl1_index == 2 && self.ptl2_index == 0 {
            // let new_ptl3_page = self.get_ptl1();
            // let new_ptl3 = alloc_pt(new_ptl3_page as *mut PageTableEntry, &mut [] ,&mut self.palloc);
            // self.ptl1_index = PageTableIndex::new(self.ptl1.get_index() as u16 - 1);
            //alloc new ptl3
        }else if self.ptl1_index == 1 && self.ptl2_index == 0 {
            //alloc new ptl2
        }else if self.ptl1_index == 0 {
            //alloc new ptl1
        }
        todo!()
    }
}

///Gets the Page Table for the given Level at the given Index, if it exists, otherwise allocates a new one
fn get_pt<L:LevelTable>(pt:&PageTable<L>, i:x86_64::structures::paging::PageTableIndex, buffer: &mut [*mut PageTableEntry], palloc:&mut impl PhysicalPageAllocator) -> PageTable<L::Down>{
    let mut entry = pt.index(i as u16 as usize);
    return if (entry.is_unused()){
        alloc_pt::<L::Down>((entry as *mut PTEntry as *mut () as *mut u64 as *mut PageTableEntry), buffer, palloc)
    }else {
        let pt_address = entry.addr().as_u64() as *mut x86_64::structures::paging::PageTable;
        PageTable::new(pt_address, PhantomData::<L::Down>)
    };

}

///Allocates a new Page Table and sets the entry to point to it
fn alloc_pt<L:Level>(entry:*mut PageTableEntry, buffer: &mut [*mut PageTableEntry], mut palloc:impl PhysicalPageAllocator) -> PageTable<L>{
    assert!(entry.is_unused());
    unsafe{
        for index in 0..buffer.len(){
            let mut buffer_entry = buffer[index];
            if buffer_entry.is_null() || !(*buffer_entry).is_unused()  {continue;}
            (*buffer_entry).set_addr(
                palloc.allocate().expect("No Physical Page available"), //TODO: Handle the out of memory scenario
                PageTableFlags::PRESENT|PageTableFlags::Writable|PageTableFlags::NO_EXECUTE
            );
            let pt_raw = (buffer_entry << 12) as *mut x86_64::structures::paging::PageTable;
            (*pt_raw).zero();
            (*entry).set_addr(PhysAddr::new_truncate(pt_raw as u64), PageTableFlags::PRESENT|PageTableFlags::Writable|PageTableFlags::NO_EXECUTE);
            ptld = PageTable::<Level3>::new(pt_raw, PhantomData::<L>);
            buffer[index] = core::ptr::null_mut();//buffer slot was used
        }
    }
    todo!()
}

pub struct PageTableGetter<L:LevelTable>{
    ptt:PageTable<L>,
    buffer:*mut [*mut u64;4]
}

impl<L:LevelTable> PageTableGetter<L> {
    pub fn new(ptt:PageTable<L>, buffer:*mut [*mut u64;4]) -> Self {
        Self{ptt, buffer}
    }
    pub fn get_entry_indexed(ptl:PageTable<L>, i:x86_64::structures::paging::PageTableIndex, mut palloc:impl PhysicalPageAllocator) -> Option<PageTableGetter<L::Down>> {
        let pte = ptl.index_unchecked(i);
        let mut ptld:PageTable<L::Down>;
        if pte.is_unused() {
            unsafe{
                for index in 0..buffer.len(){
                    let buffer_entry = buffer[index];
                    if buffer_entry.is_null() {continue;}
                    let page = buffer as *mut PageTableEntry;
                    if !(*page).is_unused() {continue;}
                    (*page).set_addr(
                        palloc.allocate().expect("No Physical Page available"),
                        PageTableFlags::PRESENT|PageTableFlags::Writable|PageTableFlags::NO_EXECUTE
                    );
                    let pt_raw = (buffer[0] << 12) as *mut x86_64::structures::paging::PageTable;
                    (*pt_raw).zero();
                    ptld = PageTable::<Level3>::new(pt_raw, PhantomData::<Level3>);
                    buffer[index] = core::ptr::null_mut();//buffer slot was used
                }
            }
        }else{
            let pt_raw = pte.addr().as_u64() as *mut u64 as *mut x86_64::structures::paging::PageTable;
            ptld = PageTable::<L::Down>::new(pt_raw, PhantomData::<L::Down>);
        }

        let mut ptl2:PageTable<Level2>;
        let pte = ptld.index_unchecked(x86_64::structures::paging::PageTableIndex::new(511));
        if pte.is_unused() {
            unsafe{
                for buffer_entry in buffer{
                    if buffer_entry.is_null() {continue;}
                    let page = buffer as *mut PageTableEntry;
                    if !(*page).is_unused() {continue;}
                    (*page).set_addr(
                        palloc.allocate().expect("No Physical Page available"),
                        PageTableFlags::PRESENT|PageTableFlags::Writable|PageTableFlags::NO_EXECUTE
                    );
                    let pt_raw = (buffer[0] << 12) as *mut x86_64::structures::paging::PageTable;
                    (*pt_raw).zero();
                    ptl2 = PageTable::<Level2>::new(pt_raw, PhantomData::<Level2>);
                }
            }
        }else{
            let pt_raw = pte.addr().as_u64() as *mut u64 as *mut x86_64::structures::paging::PageTable;
            ptl2 = PageTable::<Level2>::new(pt_raw, PhantomData::<Level2>);
        }
        todo!();
    }
}

pub fn get_or_create<L:LevelTable>(table:PageTable<L>,buffer:&[*mut u64;4], mut palloc:impl PhysicalPageAllocator) -> Option<PageTable<L::Down>>{
    todo!()
}

impl<L:LevelTable> PageTableGetter<L>{
    pub fn new(ptt:PageTable<L>,index:u16)->Self{
        let ptd = ptt.index(index as usize);
        let addr = ptt.addr();

        todo!()
    }

    pub fn get_page(&mut self) -> PTEntry {
        if self.p11_free<self.p12_free{
            self.p11_free-=1;
            self.p11.get_free_entry()
        }else{
            self.p12_free-=1;
            self.p12.get_free_entry()
        }
    }

}