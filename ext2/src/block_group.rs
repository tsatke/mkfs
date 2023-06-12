use crate::{bytefield, bytefield_field_read, bytefield_field_write, check_is_implemented};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

bytefield! {
    pub struct BlockGroupDescriptor ([u8; 32]) {
        block_usage_bitmap_block: u32 = 0,
        inode_usage_bitmap_block: u32 = 4,
        inode_table_starting_block: u32 = 8,
        num_unallocated_blocks: u16 = 12,
        num_unallocated_inodes: u16 = 14,
        num_directories: u16 = 16,
    }
}

impl BlockGroupDescriptor {
    pub fn block_usage_bitmap_block(&self) -> u32 {
        self.block_usage_bitmap_block
    }

    pub fn inode_usage_bitmap_block(&self) -> u32 {
        self.inode_usage_bitmap_block
    }

    pub fn inode_table_starting_block(&self) -> u32 {
        self.inode_table_starting_block
    }

    pub fn num_unallocated_blocks(&self) -> u16 {
        self.num_unallocated_blocks
    }

    pub fn num_unallocated_inodes(&self) -> u16 {
        self.num_unallocated_inodes
    }

    pub fn num_directories(&self) -> u16 {
        self.num_directories
    }
}

pub type Inner = Vec<BlockGroupDescriptor>;

pub struct BlockGroupDescriptorTable(Inner);

impl BlockGroupDescriptorTable {
    pub fn new() -> Self {
        Self(Inner::new())
    }
}

impl Deref for BlockGroupDescriptorTable {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BlockGroupDescriptorTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
