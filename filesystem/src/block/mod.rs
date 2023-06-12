use alloc::vec;

mod mem;

pub use mem::*;

pub trait BlockDevice {
    type Error;

    /// Determines the block size of this device.
    /// The returned value must never change.
    fn block_size(&self) -> usize;

    /// Determines the amount of blocks that
    /// this device has available.
    fn block_count(&self) -> usize;

    /// Reads the block with the given block_index into the given buffer.
    /// The buffer must be exactly as big as the value returned by
    /// [`BlockDevice::block_size`], otherwise an implementation may
    /// panic.
    /// Reads that are out of the bounds of this device must not panic, but return
    /// an appropriate error.
    fn read_block(&self, block_index: usize, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Writes the given buffer into the block with the given block
    /// index. The buffer must be exactly as big as the value returned
    /// by [`BlockDevice::block_size`], otherwise an implementation may
    /// panic.
    /// Reads that are out of the bounds of this device must not panic, but return
    /// an appropriate error.
    fn write_block(&mut self, block_index: usize, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Reads `buf.len()` bytes starting at at the given **byte** offset
    /// into the given buffer. Returns an error if the read would exceed
    /// the length of this block device.
    ///
    /// Zero-sized reads are allowed.
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.len() == 0 {
            return Ok(0);
        }

        let block_size = self.block_size();

        if offset % buf.len() == 0 && buf.len() == block_size {
            // if we read exactly one block, and that read is aligned, delegate to the device impl
            return self.read_block(offset / block_size, buf);
        }

        let start_block = offset / block_size;
        let relative_offset = offset % block_size;
        let end_block = if relative_offset + buf.len() <= block_size {
            start_block
        } else {
            (offset + buf.len()) / block_size
        };
        let block_count = end_block - start_block
            + if relative_offset == 0 && start_block != end_block {
                0
            } else {
                1
            };

        // read blocks
        let mut data = vec![0_u8; block_count * block_size];
        for i in 0..block_count {
            let start_index = i * block_size;
            let end_index = start_index + block_size;
            let read_block_index = start_block + i;

            self.read_block(read_block_index, &mut data[start_index..end_index])?;
        }
        buf.copy_from_slice(&data[relative_offset..relative_offset + buf.len()]);

        Ok(buf.len())
    }
}
