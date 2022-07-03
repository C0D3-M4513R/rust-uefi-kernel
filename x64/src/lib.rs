#![no_std]
#![allow(unused)]
//
// this is required because of https://github.com/rust-lang/rust/issues/98253
#![feature(stdsimd)]
pub mod paging;
pub mod cpuid;

#[cfg(feature = "alloc")]
extern crate alloc;