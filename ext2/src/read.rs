use alloc::vec;
use alloc::vec::Vec;

use filesystem::BlockDevice;

use crate::{BlockAddress, Error, Ext2Fs, Inode, RegularFile};

const SZ: usize = size_of::<BlockAddress>();

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    pub fn read_from_file(
        &self,
        file: &RegularFile,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, Error> {
        let file_size = file.len();
        if offset >= file_size {
            return Ok(0);
        }

        let block_size = self.superblock.block_size();
        let offset = offset as u32;

        let start_block = offset / block_size;
        let end_block = (offset + buf.len() as u32 - 1) / block_size;
        let relative_offset = (offset % block_size) as usize;
        let block_count = (end_block - start_block + 1) as usize;

        // read blocks
        let mut data: Vec<u8> = vec![0_u8; block_count * block_size as usize]; // TODO: avoid allocation - maybe try to only allocate the first and last block if the read is not aligned, but read the rest directly into the buffer
        let res = self.read_blocks_from_inode(file, start_block as usize, end_block as usize, &mut data)?;
        // copy the data into buf, but only the requested part and only up to the file size
        let total_read = res.min(file_size - offset as usize).min(buf.len());
        buf[..total_read].copy_from_slice(&data[relative_offset..relative_offset + total_read]);


        Ok(total_read)
    }

    pub(crate) fn read_blocks_from_inode(&self, inode: &Inode, start_block: usize, end_block: usize, buf: &mut [u8]) -> Result<usize, Error> {
        let block_size = self.superblock.block_size() as usize;
        assert_eq!(buf.len(), (end_block - start_block + 1) * block_size, "buf.len() must be equal to the number of blocks you want to read");

        let (direct_limit, indirect_limit, double_indirect_limit) = self.indirect_pointer_limits();

        let mut total_read = 0;

        for (i, block) in (start_block..=end_block).enumerate() {
            let block_data = &mut buf[i * block_size..(i + 1) * block_size];
            let block_pointer = if block < direct_limit as usize {
                inode.direct_ptr(block)
            } else if block < indirect_limit as usize {
                self.resolve_indirect_ptr(inode.single_indirect_ptr(), block as u32 - direct_limit)?
            } else if block < double_indirect_limit as usize {
                self.resolve_double_indirect_ptr(inode.double_indirect_ptr(), block as u32 - indirect_limit)?
            } else {
                self.resolve_triple_indirect_ptr(inode.triple_indirect_ptr(), block as u32 - double_indirect_limit)?
            };
            if let Some(block_pointer) = block_pointer {
                total_read += self.read_block(block_pointer, block_data)?;
            } else {
                block_data.fill(0);
                total_read += block_size; // FIXME: what if the last block is sparse?
            }
        }

        Ok(total_read)
    }

    pub fn indirect_pointer_limits(&self) -> (u32, u32, u32) {
        let direct_limit = 12;
        let indirect_limit = direct_limit + self.superblock.block_size() / 4;
        let double_indirect_limit = indirect_limit + indirect_limit * indirect_limit;
        (direct_limit, indirect_limit, double_indirect_limit)
    }

    pub fn is_block_allocated(&self, inode: &Inode, block_index: u32) -> Result<bool, Error> {
        self.resolve_block_index(inode, block_index)
            .map(|block| block.is_some())
    }

    pub fn resolve_block_index(&self, inode: &Inode, block_index: u32) -> Result<Option<BlockAddress>, Error> {
        let (direct_limit, indirect_limit, double_indirect_limit) = self.indirect_pointer_limits();

        Ok(
            if block_index < direct_limit {
                inode.direct_ptr(block_index as usize)
            } else if block_index < indirect_limit {
                self.resolve_indirect_ptr(inode.single_indirect_ptr(), block_index - direct_limit)?
            } else if block_index < double_indirect_limit {
                self.resolve_double_indirect_ptr(inode.double_indirect_ptr(), block_index - indirect_limit)?
            } else {
                self.resolve_triple_indirect_ptr(inode.triple_indirect_ptr(), block_index - double_indirect_limit)?
            }
        )
    }

    pub fn resolve_indirect_ptr(&self, indirect_ptr: Option<BlockAddress>, block_index: u32) -> Result<Option<BlockAddress>, Error> {
        if indirect_ptr.is_none() {
            return Ok(None);
        }
        let indirect_ptr = indirect_ptr.unwrap();

        let mut indirect_block_data = vec![0_u8; self.superblock.block_size() as usize];
        self.read_block(indirect_ptr, &mut indirect_block_data)?;
        Ok(
            indirect_block_data
                .iter()
                .copied()
                .array_chunks::<SZ>()
                .map(u32::from_le_bytes)
                .map(BlockAddress::new)
                .nth(block_index as usize)
                .unwrap() // the amount of pointers is fixed, so this is fine
        )
    }

    pub fn resolve_double_indirect_ptr(&self, double_indirect_block: Option<BlockAddress>, block_index: u32) -> Result<Option<BlockAddress>, Error> {
        let block_size = self.superblock.block_size();

        let single_indirect_block_size = block_size / 4;
        let single_indirect_index = block_index / single_indirect_block_size;

        self.resolve_indirect_ptr(double_indirect_block, single_indirect_index)
            .and_then(|single_indirect_block_ptr| self.resolve_indirect_ptr(single_indirect_block_ptr, block_index % single_indirect_block_size))
    }

    pub fn resolve_triple_indirect_ptr(&self, triple_indirect_block: Option<BlockAddress>, block_index: u32) -> Result<Option<BlockAddress>, Error> {
        let block_size = self.superblock.block_size();

        let double_indirect_block_size = block_size / 4;
        let double_indirect_index = block_index / double_indirect_block_size;

        self.resolve_indirect_ptr(triple_indirect_block, double_indirect_index)
            .and_then(|double_indirect_block_ptr| self.resolve_double_indirect_ptr(double_indirect_block_ptr, block_index % double_indirect_block_size))
    }
}
