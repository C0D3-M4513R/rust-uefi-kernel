#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![cfg_attr(target_arch = "x86_64",feature(stdsimd))]
#![allow(unused_imports)]
//extern crate alloc;

extern crate alloc;

use core::fmt::Write;
use core::ptr::null;
use core::sync::atomic::{AtomicU64, Ordering};
use uefi::prelude::*;
use uefi::table::boot::MemoryType;
use x86_64::structures::paging::PageTable;
//use uefi_services::system_table;

const MS_TO_NS:u64=1_000_000;

mod efi;
//mod x64;
//mod rust_lang;

#[entry]
fn main(_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
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
    
    log::info!("test");
    log::warn!("help");
    log::error!("halp");
    
   //x64::init();
    
    let map_file={
        let elf_kernel_file=match efi::fs::load_kernel_file(){
            Ok(v)=>v,
            Err(_)=>return Status::ABORTED,
        };
        let map_file=match efi::fs::elf::map_elf(elf_kernel_file,kernel_efi::KERNEL_ADDR){
            Ok(v)=>v,
            Err(_)=>return Status::ABORTED,
        };
        if let Err(e)=system_table.boot_services().free_pages(elf_kernel_file as *const [u8] as *const u8 as u64,elf_kernel_file.len()>>12){
            return e.status();
        }
        map_file
    };
    
    log::error!("cr0: {:#x}",x86_64::registers::control::Cr0::read().bits());
    log::error!("cr2: {:#x}",x86_64::registers::control::Cr2::read().as_u64());
    let (frame,cr3)=x86_64::registers::control::Cr3::read();
    let pt = frame.start_address().as_u64() as *mut u8 as *mut PageTable;
    let pt_r=unsafe{&mut *pt};
    let rpt=x86_64::structures::paging::RecursivePageTable::new(pt_r);
    
    {
        let efi_config=uefi::table::SystemTable::config_table(system_table);
        let mut acpi1=null();
        let mut acpi2=null();
        for e in efi_config{
            if e.guid==uefi::table::cfg::ACPI_GUID{
                acpi1=e.address;
                log::warn!("Found ACPIv1 at {:#x}",acpi1 as usize);
            }else if e.guid==uefi::table::cfg::ACPI2_GUID{
                acpi2=e.address;
                log::warn!("Found ACPIv2 at {:#x}",acpi2 as usize);
            }
        }
        let mut rsdp2:Option<efi::tables::rsdp::RSDP2>=None;
        let mut rsdp:Option<efi::tables::rsdp::RSDP>=None;
        //try initalising rsdp2
        if !acpi2.is_null(){
            let rsdpp = unsafe{ efi::tables::rsdp::RSDP2::from_ptr(acpi2)};
            log::debug!("try rsdp2 init");
            rsdp2=rsdpp.ok();
        }
        //let rsdp know about it too
        if let Some(rsdp2k)=rsdp2{
            log::info!("rsdp2 init success");
            rsdp=Some(rsdp2k.rsdp)
        }
        //try initalising rsdp if acpi2 is not available, or rsdp2 has failed verification
        if !rsdp2.is_some() && !rsdp.is_some() && !acpi1.is_null(){
            rsdp=unsafe{efi::tables::rsdp::RSDP::from_ptr(acpi1)}.ok();
            log::debug!("try rsdp1 init");
        }
        //panic, if nothing works
        if !rsdp.is_some() && !rsdp2.is_some() {
            panic!("Neither ACPIv2 nor ACPIv1 is available.")
        }
        log::warn!("rsdp2:{:#x?},rsdp1:{:#x?}",rsdp2,rsdp)
    }
    {
        let mut rsp:u64 =0;
        unsafe{ core::arch::asm!("mov {0},rsp",lateout(reg) rsp,options(nomem,preserves_flags,nostack))};
        log::info!("rip is:{:#x?}",rsp)
    }
    loop{
        #[cfg(target_arch="x86_64")]
        x86_64::instructions::hlt();
    }
    #[allow(unreachable_code)]
    Status::SUCCESS
}