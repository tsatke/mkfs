use filesystem::BlockDevice;

use crate::{Error, Ext2Fs, RegularFile};

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    #[allow(unused_variables)]
    pub fn write_to_file(
        &mut self,
        file: &RegularFile,
        offset: usize,
        buf: &[u8],
    ) -> Result<usize, Error> {
        let block_size = self.superblock.block_size();
        let offset = offset as u32;

        let start_block = offset / block_size;
        let end_block = (offset + buf.len() as u32 - 1) / block_size;
        let relative_offset = (offset % block_size) as usize;
        let block_count = (end_block - start_block + 1) as usize;

        assert_eq!(buf.len() % block_size as usize, 0, "buf.len() must be a multiple of block_size for now"); // TODO: make this more flexible

        for (i, block) in (start_block..=end_block).enumerate() {
            if !self.is_block_allocated(file, block)? {
                // TODO: we don't need to allocate if the full content of this block would be zero if the fs allows sparse files
                let free_block_address = self.allocate_block()?;
                if free_block_address.is_none() {
                    return Err(Error::NoSpace);
                }
                let free_block_address = free_block_address.unwrap();
                // TODO: write the block address to the inode
            }
        }

        // we can now be certain that all blocks that we want to write into are allocated

        todo!()
    }
}
