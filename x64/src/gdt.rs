use x86_64::structures::gdt::{GlobalDescriptorTable,DescriptorFlags,Descriptor};

mod tss;

lazy_static::lazy_static!(
	static ref GDT:GlobalDescriptorTable={
		let mut gdt=GlobalDescriptorTable::new();
		gdt.add_entry(Descriptor::UserSegment(0));
		gdt.add_entry(Descriptor::kernel_code_segment());
		gdt.add_entry(Descriptor::kernel_data_segment());
		gdt.add_entry(Descriptor::user_code_segment());
		gdt.add_entry(Descriptor::user_data_segment());
		gdt.add_entry(Descriptor::tss_segment(tss::TSS));
		gdt
	};
);

pub fn protected_mode(){
	log::trace!("Disabling Interrupts");
	x86_64::instructions::interrupts::disable();
	log::trace!("Loading GDT");
	(*GDT).load();
	log::trace!("Changing PME in cr0");
	unsafe{
		x86_64::registers::control::Cr0::update(|f| *f|=x86_64::registers::control::Cr0Flags::PROTECTED_MODE_ENABLE);
		core::arch::asm!("jmp 8:2f","2:",options(nomem,nostack));
	}
	
}