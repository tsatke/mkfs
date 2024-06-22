#![no_std]
#![feature(const_option)]

extern crate alloc;

use alloc::vec;

pub use address::*;
pub use dir::*;
pub use error::*;
pub use file::*;
use filesystem::BlockDevice;
pub use inode::*;
pub use superblock::*;

use crate::block_group::{BlockGroupDescriptor, BlockGroupDescriptorTable};

mod address;
mod block_group;
mod bytefield;
mod dir;
mod error;
mod file;
mod inode;
mod superblock;

const ROOT_DIR_INODE_ADDRESS: InodeAddress = InodeAddress::new(2).unwrap();

pub struct Ext2Fs<T> {
    block_device: T,
    superblock: Superblock,
    bgdt: BlockGroupDescriptorTable,
}

const SUPERBLOCK_OFFSET: usize = 1024;
const BLOCK_GROUP_DESCRIPTOR_TABLE_OFFSET: usize = 2048;
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

        let mut bgdt_data = vec![0_u8; superblock.block_size() as usize];
        block_device
            .read_at(BLOCK_GROUP_DESCRIPTOR_TABLE_OFFSET, &mut bgdt_data)
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

    pub fn superblock(&self) -> &Superblock {
        &self.superblock
    }

    pub fn read_root_inode(&self) -> Result<Directory, Error> {
        self.read_inode(ROOT_DIR_INODE_ADDRESS)
            .and_then(|inode| Directory::try_from(inode).map_err(|_| Error::NotDirectory))
    }

    pub fn read_inode(&self, addr: InodeAddress) -> Result<(InodeAddress, Inode), Error> {
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
        let block_size = self.superblock.block_size();
        let blocks_per_group = self.superblock.blocks_per_group();
        let num_groups = self.bgdt.len();

        for group_index in 0..num_groups {
            let first_free_block_index = self.try_reserve_block_in_group(group_index)?;
            if first_free_block_index.is_none() {
                continue;
            }
            let first_free_block_index = first_free_block_index.unwrap();

            let descriptor = &mut self.bgdt[group_index];
            *descriptor.num_unallocated_blocks_mut() -= 1;

            // read the block group descriptor table
            let mut bgdt_data = vec![0_u8; block_size as usize];
            self.block_device
                .read_at(BLOCK_GROUP_DESCRIPTOR_TABLE_OFFSET, &mut bgdt_data)
                .map_err(|_| Error::UnableToReadBlockGroupDescriptorTable)?;
            // merge the changed descriptor back into the table
            let bgd_offset = group_index * BGD_SIZE;
            let bgd_end = bgd_offset + BGD_SIZE;
            let bgd_data = Into::<[u8; BGD_SIZE]>::into(&*descriptor);
            bgdt_data[bgd_offset..bgd_end].copy_from_slice(&bgd_data);
            // write the block group descriptor table back
            self.block_device
                .write_at(BLOCK_GROUP_DESCRIPTOR_TABLE_OFFSET, &bgdt_data)
                .map_err(|_| Error::UnableToWriteBlockGroupDescriptorTable)?;


            *self.superblock.num_unallocated_blocks_mut() -= 1;
            let superblock_data = Into::<SuperblockArray>::into(&self.superblock);
            self.block_device
                .write_at(SUPERBLOCK_OFFSET, superblock_data.as_slice())
                .map_err(|_| Error::UnableToWriteSuperblock)?;

            let global_block_num = group_index as u32 * blocks_per_group + first_free_block_index as u32;
            return Ok(Some(BlockAddress::new(global_block_num).unwrap()));
        }

        Ok(None)
    }

    /// Tries to allocate a block in the given block group and returns the index in the group
    /// if successful. This reads the block usage bitmap from the block group, maybe modifies
    /// it and writes it back.
    fn try_reserve_block_in_group(&mut self, group_index: usize) -> Result<Option<usize>, Error> {
        let mut block_bitmap = vec![0_u8; self.superblock.block_size() as usize];
        let bitmap_block = self.bgdt[group_index].block_usage_bitmap_block();
        let bitmap_block_address = BlockAddress::new(bitmap_block).expect("bgdt does not have valid block address for bitmap block");
        self.read_block(bitmap_block_address, &mut block_bitmap)?;

        for (i, byte) in block_bitmap.iter_mut().enumerate() {
            for bit_index in 0..8 {
                if *byte & (1_u8 << bit_index) == 0 {
                    *byte |= 1 << bit_index;
                    return Ok(Some(i * 8 + bit_index));
                }
            }
        }
        self.write_block(bitmap_block_address, &block_bitmap)?;
        Ok(None)
    }
}
