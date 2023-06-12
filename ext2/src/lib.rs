#![no_std]
#![feature(const_option)]
#![feature(error_in_core)]

extern crate alloc;

use alloc::vec;
use core::fmt::Debug;
use core::num::NonZeroU32;
use core::ops::Deref;

use crate::block_group::{BlockGroupDescriptor, BlockGroupDescriptorTable};

mod block_group;
mod bytefield;
mod dir;
mod error;
mod inode;
mod superblock;

use filesystem::BlockDevice;

pub use dir::*;
pub use error::*;
pub use inode::*;
pub use superblock::*;

const ROOT_DIR_INODE_ADDRESS: InodeAddress = InodeAddress::new(2).unwrap();

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct InodeAddress(NonZeroU32);

impl Deref for InodeAddress {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl InodeAddress {
    pub const fn new(n: u32) -> Option<Self> {
        let nzu32 = NonZeroU32::new(n);
        match nzu32 {
            None => None,
            Some(v) => Some(Self(v)),
        }
    }
}

pub struct Ext2Fs<T> {
    block_device: T,
    superblock: Superblock,
    bgdt: BlockGroupDescriptorTable,
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

        let superblock = Superblock::try_from(SuperblockArray::from(superblock_data)).unwrap();
        let number_of_block_groups = (superblock.num_blocks() + superblock.blocks_per_group() - 1)
            / superblock.blocks_per_group();

        let mut bgdt_data = vec![0_u8; superblock.block_size() as usize];
        block_device
            .read_at(2048, &mut bgdt_data)
            .map_err(|_| Error::UnableToReadBlockGroupDescriptorTable)?;
        let mut bgdt = BlockGroupDescriptorTable::new();
        for i in 0..number_of_block_groups as usize {
            const BGD_SIZE: usize = 32; // 32 bytes per block group descriptor
            let offset = i * BGD_SIZE;
            let end = offset + BGD_SIZE;
            let bgd_data = TryInto::<[u8; BGD_SIZE]>::try_into(&bgdt_data[offset..end]).unwrap();
            let bgd = BlockGroupDescriptor::try_from(bgd_data).unwrap();
            bgdt.push(bgd);
        }

        Ok(Self {
            block_device,
            superblock,
            bgdt,
        })
    }

    pub fn read_root_inode(&self) -> Result<Inode, Error> {
        self.read_inode(ROOT_DIR_INODE_ADDRESS)
    }

    fn read_inode(&self, addr: InodeAddress) -> Result<Inode, Error> {
        let inodes_per_group = self.superblock.inodes_per_group();
        let block_group_index = (addr.get() - 1) / inodes_per_group;
        let block_group = &self.bgdt[block_group_index as usize];
        let itable_start_block = block_group.inode_table_starting_block();

        let index = (addr.get() - 1) % inodes_per_group;
        let inode_size = self.superblock.inode_size();
        let address =
            self.get_block_address(itable_start_block) + (index * inode_size as u32) as usize;

        let mut inode_buffer = [0_u8; 128]; // inode size can vary, but the specified fields are always between 0 and 128, and we don't need more
        self.block_device
            .read_at(address, &mut inode_buffer)
            .map_err(|_| Error::DeviceRead)?;

        Ok(
            Inode::try_from(InodeRawArray::from(inode_buffer))
                .expect("inode conversion can't fail. if it does, the logic has changed and this should propagate the error")
        )
    }

    pub(crate) fn read_block(&self, block: u32, buf: &mut [u8]) -> Result<usize, T::Error> {
        let addr = self.get_block_address(block);
        self.block_device.read_at(addr, buf)
    }

    fn get_block_address(&self, block: u32) -> usize {
        assert_ne!(
            block, 0,
            "a block address of 0 means the address is invalid"
        );
        (1024 + (block - 1) * self.superblock.block_size()) as usize
    }
}
