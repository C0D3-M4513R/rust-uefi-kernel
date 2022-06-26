use core::arch::x86_64::{__cpuid, __cpuid_count, has_cpuid};
use core::ops::{BitAnd, BitOr};
use x86_64::instructions::tables::lgdt;
use x86_64::registers::rflags;
use x86_64::registers::control::{Cr0, Cr0Flags, Efer, EferFlags};
use x86_64::registers::rflags::RFlags;
//This file copies asm code largely from there:
//https://wiki.osdev.org/Setting_Up_Long_Mode
//Thanks!

//static mut GDT:x86_64::structures::gdt::GlobalDescriptorTable = x86_64::structures::gdt::GlobalDescriptorTable::new();

unsafe fn setup_long_mode(){
	if cpuid_enabled() && extended_mode_available() && long_mode_available(){
		enable_a20();
		Cr0::write(Cr0::read().bitand(Cr0Flags::PAGING));
		enable_long_mode();
	}
}
unsafe fn cpuid_enabled()->bool{
	return has_cpuid();
	//This would be the way, I'd solve it.
	//After I wrote this I discovered, that rust already has a function for this.
	// let init=rflags::read();
	// let bi = init.bitor(RFlags::ID);
	// rflags::write(bi);
	// if rflags::read()==bi{
	// 	rflags::write(init);
	// 	return true;
	// }
	// else{
	// 	return false;
	// }
}

unsafe fn extended_mode_available()->bool{
	__cpuid(0x80000000).eax>0x80000000
}

unsafe fn long_mode_available()->bool{
	let tmp=__cpuid(0x80000001).edx;
	tmp&(1<<29)!=0
}
pub(super) unsafe fn enable_long_mode(){
	Efer::write(Efer::read().bitor(EferFlags::LONG_MODE_ENABLE))
}
unsafe fn enable_a20(){
	core::arch::asm!(
		"in al, 0x92",
		"test al, 2",
		"jnz 1f",
		"or al, 2",
		"and al, 0xFE",
		"out 0x92, al",
		"1:",
    options(nomem,nostack,preserves_flags)
	)
}
pub(super) fn pml5_avilable()->bool{
	unsafe {
		let tmp = __cpuid(0x7).ecx;
		tmp>>16&1==1
	}
}