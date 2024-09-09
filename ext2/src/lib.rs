#![no_std]
#![feature(const_option)]
#![feature(iter_array_chunks)]

extern crate alloc;

use alloc::vec;

pub use address::*;
pub use dir::*;
pub use error::*;
use filesystem::BlockDevice;
pub use inode::*;
pub use superblock::*;

use crate::block_group::{BlockGroupDescriptor, BlockGroupDescriptorTable};

mod address;
mod block_group;
mod bytefield;
mod create;
mod dir;
mod error;
mod inode;
mod read;
mod superblock;
mod write;

const ROOT_DIR_INODE_ADDRESS: InodeAddress = InodeAddress::new(2).unwrap();

/// An ext2 filesystem over a block device.
pub struct Ext2Fs<T> {
    block_device: T,
    superblock: Superblock,
    bgdt: BlockGroupDescriptorTable,
}

const SUPERBLOCK_OFFSET: usize = 1024;
const BGD_SIZE: usize = 32; // 32 bytes per block group descriptor

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    pub fn try_new(block_device: T) -> Result<Self, Error> {
        let mut superblock_data = [0_u8; 1024];
        block_device
            .read_at(SUPERBLOCK_OFFSET, &mut superblock_data)
            .map_err(|_| Error::UnableToReadSuperblock)?;

        let superblock = Superblock::try_from(SuperblockArray::from(superblock_data)).unwrap();
        let number_of_block_groups = (superblock.num_blocks() + superblock.blocks_per_group() - 1)
            / superblock.blocks_per_group();

        let bgdt_offset = if superblock.block_size() == 1024 { 2048 } else { superblock.block_size() } as usize;

        let mut bgdt_data = vec![0_u8; superblock.block_size() as usize];
        block_device
            .read_at(bgdt_offset, &mut bgdt_data)
            .map_err(|_| Error::UnableToReadBlockGroupDescriptorTable)?;
        let mut bgdt = BlockGroupDescriptorTable::new();
        for i in 0..number_of_block_groups as usize {
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

    fn bgdt_offset(&self) -> usize {
        let block_size = self.superblock.block_size() as usize;
        if block_size == 1024 {
            2048
        } else {
            block_size
        }
    }

    pub fn superblock(&self) -> &Superblock {
        &self.superblock
    }

    pub fn read_root_inode(&self) -> Result<Directory, Error> {
        self.read_inode(ROOT_DIR_INODE_ADDRESS)
            .and_then(|inode| Directory::try_from(inode).map_err(|_| Error::NotDirectory))
    }

    pub fn read_inode(&self, addr: InodeAddress) -> Result<(InodeAddress, Inode), Error> {
        // FIXME: reading the inode will create multiple copies that alias the data in the block device, there needs to be some kind of cache that centralizes the inodes (and their data)
        let inodes_per_group = self.superblock.inodes_per_group();
        let block_group_index = (addr.get() - 1) / inodes_per_group;
        let block_group = &self.bgdt[block_group_index as usize];
        let itable_start_block =
            BlockAddress::new(block_group.inode_table_starting_block()).unwrap();

        let index = (addr.get() - 1) % inodes_per_group;
        let inode_size = self.superblock.inode_size();
        let address =
            self.resolve_block_offset(itable_start_block) + (index * inode_size as u32) as usize;

        let mut inode_buffer = [0_u8; 128]; // inode size can vary, but the specified fields are always between 0 and 128, and we don't need more
        self.block_device
            .read_at(address, &mut inode_buffer)
            .map_err(|_| Error::DeviceRead)?;

        Ok((addr, Inode::try_from(InodeRawArray::from(inode_buffer)).expect("inode conversion can't fail. if it does, the logic has changed and this should propagate the error")))
    }

    pub fn write_inode(&mut self, addr: InodeAddress, inode: &Inode) -> Result<(), Error> {
        let inodes_per_group = self.superblock.inodes_per_group();
        let block_group_index = (addr.get() - 1) / inodes_per_group;
        let block_group = &self.bgdt[block_group_index as usize];
        let itable_start_block =
            BlockAddress::new(block_group.inode_table_starting_block()).unwrap();

        let index = (addr.get() - 1) % inodes_per_group;
        let inode_size = self.superblock.inode_size();
        let address =
            self.resolve_block_offset(itable_start_block) + (index * inode_size as u32) as usize;

        let inode_raw = InodeRawArray::from(inode);
        self.block_device
            .write_at(address, &inode_raw.as_slice())
            .map_err(|_| Error::DeviceWrite)
            .map(|_| ())
    }

    pub fn read_block(&self, addr: BlockAddress, buf: &mut [u8]) -> Result<usize, Error> {
        let offset = self.resolve_block_offset(addr);
        self.block_device
            .read_at(offset, buf)
            .map_err(|_| Error::DeviceRead)
    }

    pub fn write_block(&mut self, addr: BlockAddress, buf: &[u8]) -> Result<usize, Error> {
        let offset = self.resolve_block_offset(addr);
        self.block_device
            .write_at(offset, buf)
            .map_err(|_| Error::DeviceWrite)
    }

    fn resolve_block_offset(&self, addr: BlockAddress) -> usize {
        (1024 + (addr.get() - 1) * self.superblock.block_size()) as usize
    }

    pub fn allocate_block(&mut self) -> Result<Option<BlockAddress>, Error> {
        let blocks_per_group = self.superblock.blocks_per_group();
        self.allocate_resource(blocks_per_group, Self::try_reserve_block_in_group)
            .map(|block| block.map(BlockAddress::new).flatten())
    }

    pub fn allocate_inode(&mut self) -> Result<Option<InodeAddress>, Error> {
        let inodes_per_group = self.superblock.inodes_per_group();
        self.allocate_resource(inodes_per_group, Self::try_reserve_inode_in_group)
            .map(|inode| inode.map(InodeAddress::new).flatten())
    }

    fn allocate_resource<F>(&mut self, resource_per_group: u32, try_reserve_in_group: F) -> Result<Option<u32>, Error>
    where
        F: Fn(&mut Self, usize) -> Result<Option<usize>, Error>,
    {
        let block_size = self.superblock.block_size();
        let num_groups = self.bgdt.len();
        let bgdt_offset = self.bgdt_offset();

        for group_index in 0..num_groups {
            let first_free_resource_index = try_reserve_in_group(self, group_index)?;
            if first_free_resource_index.is_none() {
                continue;
            }
            let first_free_resource_index = first_free_resource_index.unwrap();

            let descriptor = &mut self.bgdt[group_index];
            *descriptor.num_unallocated_blocks_mut() -= 1;

            // read the block group descriptor table
            let mut bgdt_data = vec![0_u8; block_size as usize];
            self.block_device
                .read_at(bgdt_offset, &mut bgdt_data)
                .map_err(|_| Error::UnableToReadBlockGroupDescriptorTable)?;
            // merge the changed descriptor back into the table
            let bgd_offset = group_index * BGD_SIZE;
            let bgd_end = bgd_offset + BGD_SIZE;
            let bgd_data = Into::<[u8; BGD_SIZE]>::into(&*descriptor);
            bgdt_data[bgd_offset..bgd_end].copy_from_slice(&bgd_data);
            // write the block group descriptor table back
            self.block_device
                .write_at(bgdt_offset, &bgdt_data)
                .map_err(|_| Error::UnableToWriteBlockGroupDescriptorTable)?;

            *self.superblock.num_unallocated_blocks_mut() -= 1;
            let superblock_data = Into::<SuperblockArray>::into(&self.superblock);
            self.block_device
                .write_at(SUPERBLOCK_OFFSET, superblock_data.as_slice())
                .map_err(|_| Error::UnableToWriteSuperblock)?;

            let global_resource_num = group_index as u32 * resource_per_group + first_free_resource_index as u32;
            return Ok(Some(global_resource_num));
        }

        Ok(None)
    }

    fn try_reserve_block_in_group(&mut self, group_index: usize) -> Result<Option<usize>, Error> {
        let bitmap_block = self.bgdt[group_index].block_usage_bitmap_block();
        let bitmap_block_address = BlockAddress::new(bitmap_block).expect("bgdt does not have valid block address for bitmap block");
        self.try_reserve_in_group_with_bitmap(bitmap_block_address)
    }

    fn try_reserve_inode_in_group(&mut self, group_index: usize) -> Result<Option<usize>, Error> {
        let bitmap_block = self.bgdt[group_index].inode_usage_bitmap_block();
        let bitmap_block_address = BlockAddress::new(bitmap_block).expect("bgdt does not have valid block address for bitmap block");
        self.try_reserve_in_group_with_bitmap(bitmap_block_address)
    }

    fn try_reserve_in_group_with_bitmap(&mut self, bitmap_block: BlockAddress) -> Result<Option<usize>, Error> {
        let mut inode_bitmap = vec![0_u8; self.superblock.block_size() as usize];
        self.read_block(bitmap_block, &mut inode_bitmap)?;

        for (i, byte) in inode_bitmap.iter_mut().enumerate() {
            for bit_index in 0..8 {
                if *byte & (1_u8 << bit_index) == 0 {
                    *byte |= 1 << bit_index;
                    return Ok(Some(i * 8 + bit_index));
                }
            }
        }
        self.write_block(bitmap_block, &inode_bitmap)?;
        Ok(None)
    }
}
