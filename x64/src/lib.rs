#![no_std]
#![allow(unused)]
//
// this is required because of https://github.com/rust-lang/rust/issues/98253
#![feature(stdsimd)]
extern crate alloc;

pub mod paging;
pub mod cpuid;
pub mod palloc;