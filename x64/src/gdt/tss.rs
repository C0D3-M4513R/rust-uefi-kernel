use x86_64::structures::tss::TaskStateSegment;

static mut TSS_MUT:TaskStateSegment=TaskStateSegment::new();

pub static TSS:&'static TaskStateSegment=unsafe{&TSS_MUT};