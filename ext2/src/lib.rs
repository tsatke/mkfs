#![no_std]
#![feature(error_in_core)]

extern crate alloc;

use core::fmt::{Debug, Display, Formatter};
use core::num::NonZeroU32;
use filesystem::BlockDevice;

mod superblock;

use crate::superblock::Superblock;
pub use superblock::SuperblockDecodeError;

#[derive(Debug)]
struct Ext2InodeAddress(NonZeroU32);

impl Ext2InodeAddress {
    pub fn new(n: u32) -> Option<Self> {
        Some(Self(NonZeroU32::new(n)?))
    }
}

pub struct Ext2Fs<T> {
    block_device: T,
    superblock: Superblock,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    InvalidSuperblock(SuperblockDecodeError),
    UnableToReadSuperblock,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl core::error::Error for Error {}

impl From<SuperblockDecodeError> for Error {
    fn from(value: SuperblockDecodeError) -> Self {
        Self::InvalidSuperblock(value)
    }
}

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    pub fn try_new(block_device: T) -> Result<Self, Error> {
        let mut superblock_data = [0_u8; 1024];
        block_device
            .read_at(1024, &mut superblock_data)
            .map_err(|_| Error::UnableToReadSuperblock)?;

        let superblock = Superblock::try_from(superblock_data)?;

        Ok(Self {
            block_device,
            superblock,
        })
    }
}
