#![allow(non_snake_case)]
use core::ffi::c_void;

const RSDP_SIGNATURE_MAGIC:&[u8]="RSD PTR ".as_bytes();
#[repr(packed)]
#[derive(Debug,Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct RSDP{
	pub Signature:[u8;8],
	pub Checksum:u8,
	pub OEMID:[u8;6],
	pub Revision:u8,
	pub RsdtAddress:u32,
}
impl RSDP{
	///This will construct a RSDP from the pointer.
	///# Return
	/// A return value of Err just means, that the rsdp has the wrong signature.
	/// Similar a return value of Ok just means, that as far as the specification goes, this could be a valid rsdp.
	///# Safety
	/// This function assumes, that p is valid for at least 20 bytes.
	pub unsafe fn from_ptr(p:*const c_void)->Result<Self,()>{
		let rsdp=*(p as *const RSDP);
		if RSDP_SIGNATURE_MAGIC==rsdp.Signature{
			Ok(rsdp)
		}else{
			Err(())
		}
		//--------------------------------------
		//this would be the safe way to do this.
		//but we could speed this up, when using the layout to our advantage
		//
		// let p = p as *const u8;
		// let mut a = [0u8;20];
		// core::ptr::copy_nonoverlapping(p,a.as_mut_ptr(),a.len());
		// RSDP{
		// 	Signature:[a[0],a[1],a[2],a[3],a[4],a[5],a[6],a[7]],
		// 	Checksum:a[8],
		// 	OEMID:[a[9],a[10],a[11],a[12],a[13],a[14]],
		// 	Revision:a[15],
		// 	RsdtAddress:(a[16] as u32)<<3 | (a[17] as u32)<<2 | (a[18] as u32)<<1 | (a[19] as u32)<<0,
		// }
	}
}
#[repr(packed)]
#[derive(Debug,Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct RSDP2 {
	pub rsdp:RSDP,
	pub Length:u32,
	pub XSdtAddress:u64,
	pub ExtendedChecksum:u8,
	pub RESERVED:[u8;3]
}

impl RSDP2{
	///This will construct a RSDP from the pointer.
	///# Return
	/// A return value of Err just means, that the rsdp has the wrong signature or wrong size.
	/// Similar a return value of Ok just means, that as far as the specification goes, this could be a valid rsdp.
	///# Safety
	/// This function assumes, that p is valid for at least 36 bytes.
	pub unsafe fn from_ptr(p:*const c_void)->Result<Self,()>{
		let rsdp=*(p as *const RSDP2);
		if RSDP_SIGNATURE_MAGIC==rsdp.rsdp.Signature && rsdp.Length>=36{
			Ok(rsdp)
		}else{
			Err(())
		}
		//--------------------------------------
		//this would be the safe way to do this.
		//but we could speed this up, when using the layout to our advantage
		
		// let rsdp=RSDP::from_ptr(p);
		// let p=p as *const u32;
		// let mut a = [0;4];
		// core::ptr::copy_nonoverlapping(p,a.as_mut_ptr(),a.len());
		// let a3=a[3].to_le_bytes();
		// RSDP2{
		// 	rsdp,
		// 	Length:a[0],
		// 	XSdtAddress:(a[1] as u64)<<4 | (a[2] as u64)<<0,
		// 	ExtendedChecksum:a3[0],
		// 	RESERVED:[a3[1],a3[2],a3[3]]
		// }
		
	}
}