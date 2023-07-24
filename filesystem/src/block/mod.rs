use alloc::vec;

mod mem;

pub use mem::*;

pub trait BlockDevice {
    type Error;

    /// Determines the sector size of this device.
    /// The returned value must never change.
    fn sector_size(&self) -> usize;

    /// Determines the amount of sectors that
    /// this device has available.
    fn sector_count(&self) -> usize;

    /// Reads the sector with the given sector_index into the given buffer.
    /// The buffer must be exactly as big as the value returned by
    /// [`BlockDevice::sector_size`], otherwise an implementation may
    /// panic.
    /// Reads that are out of the bounds of this device must not panic, but return
    /// an appropriate error.
    fn read_sector(&self, sector_index: usize, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Writes the given buffer into the sector with the given sector
    /// index. The buffer must be exactly as big as the value returned
    /// by [`BlockDevice::sector_size`], otherwise an implementation may
    /// panic.
    /// Reads that are out of the bounds of this device must not panic, but return
    /// an appropriate error.
    fn write_sector(&mut self, sector_index: usize, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Reads `buf.len()` bytes starting at at the given **byte** offset
    /// into the given buffer. Returns an error if the read would exceed
    /// the length of this block device.
    ///
    /// Zero-sized reads are allowed.
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let sector_size = self.sector_size();

        if offset % buf.len() == 0 && buf.len() == sector_size {
            // if we read exactly one sector, and that read is aligned, delegate to the device impl
            return self.read_sector(offset / sector_size, buf);
        }

        let start_sector = offset / sector_size;
        let relative_offset = offset % sector_size;
        let end_sector = if relative_offset + buf.len() <= sector_size {
            start_sector
        } else {
            (offset + buf.len()) / sector_size
        };
        let sector_count = end_sector - start_sector
            + if relative_offset == 0 && buf.len() % sector_size == 0 && start_sector != end_sector
            {
                0
            } else {
                1
            };

        // read sectors
        let mut data = vec![0_u8; sector_count * sector_size];
        for i in 0..sector_count {
            let start_index = i * sector_size;
            let end_index = start_index + sector_size;
            let read_sector_index = start_sector + i;

            self.read_sector(read_sector_index, &mut data[start_index..end_index])?;
        }
        buf.copy_from_slice(&data[relative_offset..relative_offset + buf.len()]);

        Ok(buf.len())
    }
}
