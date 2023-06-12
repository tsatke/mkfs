//! This crate provides common interfaces for the several file system
//! implementations. The interfaces can be used to interact with the
//! no_std implementations in an std environment.

#![no_std]

extern crate alloc;

mod block;

pub use block::*;
