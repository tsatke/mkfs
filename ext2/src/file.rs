use crate::{Error, Ext2Fs, RegularFile};
use alloc::vec;
use filesystem::BlockDevice;

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
        let file_len = file.len();

        if buf.is_empty() {
            return Ok(0);
        }
        let mut read_bytes = 0;

        // we use this for reading full blocks from the file system, and copy from this
        // buffer into the actual read buffer
        let mut block_buf = vec![0_u8; self.superblock.block_size() as usize];

        let start_ptr = offset / self.superblock.block_size() as usize;
        let mut current_block_ptr = start_ptr;
        while let Some(block_addr) = file.direct_ptr(current_block_ptr) {
            self.read_block(block_addr, &mut block_buf)?;

            let remaining_bytes_in_file = file_len - read_bytes;
            let range = read_bytes
                ..buf
                    .len() // read the full read buffer...
                    .min(block_buf.len()) // ...or the full block buffer, if that is shorter than the remaining read buffer...
                    .min(read_bytes + remaining_bytes_in_file); // ...or the remaining bytes in the file, if that is shorter
            read_bytes += range.len();

            let block_buffer_start_offset = if current_block_ptr == start_ptr {
                // if we're reading the first block, we need to respect the specified offset
                offset
            } else {
                0
            };
            let data =
                &block_buf[block_buffer_start_offset..block_buffer_start_offset + range.len()];
            buf[range].copy_from_slice(data);

            current_block_ptr += 1;

            if read_bytes >= buf.len() {
                break;
            }
        }
        Ok(read_bytes)
    }
}
