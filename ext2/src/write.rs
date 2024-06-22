use alloc::vec;

use filesystem::BlockDevice;

use crate::{Error, Ext2Fs, RegularFile};

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    #[allow(unused_variables)]
    pub fn write_to_file(
        &mut self,
        file: &mut RegularFile,
        offset: usize,
        buf: &[u8],
    ) -> Result<usize, Error> {
        let block_size = self.superblock.block_size();
        let offset = offset as u32;

        let start_block = offset / block_size;
        let end_block = (offset + buf.len() as u32 - 1) / block_size;
        let relative_offset = (offset % block_size) as usize;
        let block_count = (end_block - start_block + 1) as usize;

        // This is the data that we want to write. We pad the data with data from the disk
        // to align it with the block boundaries (start and length). This data can be written
        // back to disk (block aligned) as is.
        let data = {
            let mut data = vec![0_u8; block_count * block_size as usize];
            self.read_blocks_from_inode(&file, start_block as usize, end_block as usize, &mut data)?; // TODO: we don't need to read what will be overwritten anyways
            // overwrite the part that should be written
            data[relative_offset..relative_offset + buf.len()].copy_from_slice(buf);
            data
        };

        let mut chunks = data.chunks_exact(block_size as usize);
        for ((i, block), chunk) in (start_block..=end_block).enumerate().zip(&mut chunks) {
            let block_address =
                if let Some(block_address) = self.resolve_block_index(file, block)? {
                    block_address
                } else {
                    // TODO: we don't need to allocate if the full content of this block would be zero if the fs allows sparse files
                    let free_block_address = self.allocate_block()?;
                    if free_block_address.is_none() {
                        return Err(Error::NoSpace);
                    }
                    let free_block_address = free_block_address.unwrap();

                    let inode = file.inode_mut();
                    let free_slot = inode.direct_ptrs()
                        .enumerate()
                        .find(|(_, ptr)| ptr.is_none())
                        .map(|(i, _)| i);
                    if let Some(free_slot) = free_slot {
                        inode.set_direct_ptr(free_slot, Some(free_block_address));
                        self.write_inode(file.inode_address(), file)?;
                    } else {
                        todo!("allocate indirect block");
                    }

                    free_block_address
                };

            self.write_block(block_address, chunk)?;
        }
        debug_assert_eq!(chunks.remainder().len(), 0, "data to write was not block aligned");

        if file.len() < offset as usize + buf.len() {
            let new_size = offset as usize + buf.len();
            let new_size_lower = new_size as u32;
            let new_size_upper = (new_size >> 32) as u32;

            let inode = file.inode_mut();
            inode.set_file_size_lower(new_size_lower);
            inode.set_file_size_upper(new_size_upper);

            self.write_inode(file.inode_address(), file)?;
        }

        Ok(buf.len())
    }
}
