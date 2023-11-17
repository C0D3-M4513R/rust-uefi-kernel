#![no_main]
#![no_std]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![allow(unused_imports)]
//extern crate alloc;
extern crate core;
extern crate alloc;

use core::ffi::c_void;
use core::fmt::Write;
use core::ops::Add;
use core::ptr::{NonNull, null};
use core::sync::atomic::{AtomicU64, compiler_fence, Ordering};
use uefi::prelude::*;
use uefi::table::boot::{AllocateType, MemoryType};
use x86_64::PhysAddr;
use x86_64::structures::paging::{PageTable, PageTableFlags};
use x86_64::structures::paging::page_table::PageTableEntry;
use kernel_efi::Args;
use crate::efi::mem::get_mem;

mod efi;
//mod rust_lang;

#[entry]
fn entry(handle: Handle, system_table: SystemTable<Boot>) -> uefi::Status{
    match main(handle,system_table) {
        Ok(_)=>Status::SUCCESS,
        Err(err)=>err.status(),
    }
}

fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> uefi::Result {
    uefi_services::init(&mut system_table).unwrap();
    let system_table=unsafe{uefi_services::system_table().as_mut()};
    //This logger impl will be taken over by uefi-services
    #[cfg(any())]
    if false{
        let mst=system_table;
        let stde = mst.stderr();
        let stdo = mst.stdout();
        
        stdo.write_str("test").ok();
        
        log_impl::Logger::set_output(stdo);
        unsafe{
            static mut LOG:log_impl::Logger=log_impl::Logger::new();
            (&mut LOG).init();
            log::set_logger(&LOG).expect("Logger could not be set");
        }
    }
    
    let _kernel_args=system_table
        .boot_services()
        .allocate_pages(
            AllocateType::Address(kernel_efi::ARGS_ADDR as u64),
            MemoryType::LOADER_DATA,
            1
        )?;
    
    log::info!("test");
    log::warn!("help");
    log::error!("halp");

    let heap_size = get_mem(None)?.1;
    let base_prt:u64;
    let base_phys_prt:u64;
    let prt_pages;
    {
        prt_pages = heap_size / 4096 / 4096; //a page holds 4096 bytes, and this is the amount of pages to allocate
        base_phys_prt = system_table.boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            prt_pages as usize
        )?;
        let base_addr = kernel_efi::ARGS_ADDR + core::mem::size_of::<kernel_efi::Args>();
        let base_addr = base_addr + if base_addr%4096 = 0 {0} else {4096 - (base_addr % 4096)};
        base_prt = base_addr;
        for i in 0..prt_pages {

            let mut pw5 ;
            let mut pw = match x64::paging::get_page_walker(){
                Ok(w) => {
                    pw5 = w;
                    pw5.create_pt(0 as u64).unwrap()
                },
                Err(w) => w,
            };
            efi::fs::elf::get_pte(
                &mut pw,
                base_addr as u64 + i as u64,
                |x|
                    x.set_addr(
                        PhysAddr::new(addr),
                        PageTableFlags::PRESENT|PageTableFlags::WRITABLE|PageTableFlags::NO_EXECUTE
                    )
            ).unwrap();
        }
        //zero the memory. There might be garbage in that memory, and we depend on that memory being initialized to 0
        for i in 0..prt_pages*4096/64{
            unsafe{core::ptr::write((base_addr as *mut u64).wrapping_offset(i as isize),0u64)};
        }
        set_bits(base_prt as *mut u64,(base_phys_prt / 4096) as usize,prt_pages as usize);
        {
            let offset = (base_phys_prt / 4096) as usize + prt_pages as usize;
            if offset%4096!=0 {set_bits(base_prt as *mut u64,offset,4096-offset%4096);}
        }
    }
    {
        let map_file={
            let elf_kernel_file=efi::fs::load_file(efi::fs::KERNEL_NAME)?;
            let map_file=efi::fs::elf::map_elf(elf_kernel_file,kernel_efi::KERNEL_ADDR)?;
            system_table.boot_services().free_pages(elf_kernel_file as *const [u8] as *const u8 as u64,(elf_kernel_file.len()>>12)+1)?;
            map_file
        };
        set_bits(base_prt as *mut u64,map_file.base/4096,map_file.pages+3);
        let ff ={
            const FONT_FILE:&str = "font.ttf";
            let f = efi::fs::load_file(FONT_FILE)?;
            if let Some(nf)=kernel_efi::ttf_parser::fonts_in_collection(f){
                if nf>1{
                    kernel_efi::ttf_parser::Face::from_slice(f,0).unwrap()
                }else {
                    panic!();
                }
            }else{
                system_table.boot_services().free_pages(f as *const [u8] as *const u8 as u64,(f.len()>>12)+1).unwrap();
                panic!();
            }
        };
        let mut pte = [core::ptr::null_mut();3];
        //Mark kernel_efi::ARGS_ADDR as r,w,nx.
        {
            let s=core::mem::size_of::<kernel_efi::Args>();
            let s= s>>12 + if s%4096!=0{1}else{0};
            let mut pw5;
            let mut pw=match x64::paging::get_page_walker().unwrap() {
                Ok(w) => {
                    pw5 = w;
                    pw5.create_pt(kernel_efi::ARGS_ADDR as u64).unwrap()
                },
                Err(w) => w,
            };
            let elf_physical_page = system_table.
                boot_services().
                allocate_pages(
                    AllocateType::AnyPages,
                    MemoryType::LOADER_DATA,
                    s+3
                )?;
            for i in 0..=s{
                efi::fs::elf::get_pte(
                    &mut pw,
                    kernel_efi::ARGS_ADDR as u64 + i as u64,
                    |x|
                        x.set_addr(
                            elf_physical_page.add(i*4096),
                            PageTableFlags::PRESENT|PageTableFlags::WRITABLE|PageTableFlags::NO_EXECUTE
                        )
                ).unwrap()
            }
            for i in s+1..=s+3{
                efi::fs::elf::get_pte(
                    &mut pw,
                    kernel_efi::ARGS_ADDR as u64 + i as u64,
                    |x|{
                        pte[i-s-1]=x as *mut PageTableEntry as *mut u64;
                        x.set_addr(
                            elf_physical_page.add(i*4096),
                            PageTableFlags::PRESENT|PageTableFlags::WRITABLE|PageTableFlags::NO_EXECUTE
                        )
                    }
                ).unwrap(
                )
            }
        }
        unsafe {
            core::ptr::write_volatile(
                kernel_efi::ARGS_ADDR,
                 Args {
                     elf: map_file,
                     font: ff,
                     gop: efi::gop::get_best_gop_fb(handle)?,
                     heap_size,
                     page_tracker_base: base_prt as *mut (),
                     page_tracker_page_size: prt_pages as usize,
                     page_table_entry: pte,
                 }
            );
        }
    }
    {
        let mut rsp:u64;
        unsafe{ core::arch::asm!("mov {0},rsp",lateout(reg) rsp,options(nomem,preserves_flags,nostack))};
        log::info!("rip is:{:#x?}",rsp)
    }
    loop{
        core::hint::spin_loop();
        #[cfg(target_arch="x86_64")]
        x86_64::instructions::hlt();
    }
    #[allow(unreachable_code)]
    Ok(())
}

fn set_bits(base_ptr:*mut u64, offset:usize, size:usize){
    {
        let mut prt_offset = offset; //map_file.base as u64 / 4096;
        let mut size = size;//map_file.pages;
        if prt_offset%64 != 0 {
            let offset = prt_offset % 64;
            let bits_available = 64 - offset;
            let bits_to_set =
                //mask all bits before the first bit to set
                (!(u64::MAX<<bits_available))&
                    //mask all bits after the potentially last bit to set
                    (!(u64::MAX<< if size > bits_available as usize {0} else {size}));
            //this should set size or bits_available bits to 1 (whichever is less), starting at offset
            prt_offset/=64;
            unsafe {
                core::ptr::write_volatile((base_ptr as *mut u64).wrapping_offset(prt_offset as isize), bits_to_set);
            }
            prt_offset+=1;
            size-=bits_to_set.count_ones() as usize;
        };
        while size >= 64 {
            unsafe{core::ptr::write_volatile((base_ptr as *mut u64).wrapping_offset(prt_offset as isize), u64::MAX)}
            prt_offset+=1;
            size-=64;
        }
        unsafe {
            core::ptr::write_volatile((base_ptr as *mut u64).wrapping_offset(prt_offset as isize), !(u64::MAX << 64-size));
        }
    }
}