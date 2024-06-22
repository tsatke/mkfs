use alloc::vec;

pub use mem::*;

mod mem;

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

    /// Reads `buf.len()` bytes starting at the given **byte** offset
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

    /// Writes `buf.len()` bytes starting at the given **byte** offset
    /// onto the block device. Returns an error if the write would exceed
    /// the length of this block device.
    ///
    /// Zero-sized writes are allowed.
    fn write_at(&mut self, offset: usize, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let sector_size = self.sector_size();

        if offset % buf.len() == 0 && buf.len() == sector_size {
            // if we write exactly one sector, and that write is aligned, delegate to the device impl
            return self.write_sector(offset / sector_size, buf);
        }

        let start_sector = offset / sector_size;
        let relative_start_offset = offset % sector_size;
        let end_sector = if relative_start_offset + buf.len() <= sector_size {
            start_sector
        } else {
            (offset + buf.len()) / sector_size
        };
        let relative_end_offset = (relative_start_offset + buf.len()) % sector_size;

        // The write is not aligned, so we have to read the first and last sector, merge
        // the data with the given buffer, and write the merged data back to the device.
        // For all other sectors in between, we can write the data directly to the device.

        if start_sector == end_sector {
            // If we have a read that is shorter than a single sector, read that sector, merge the data
            // and write it back.
            let mut first_sector = vec![0_u8; sector_size];
            self.read_sector(start_sector, &mut first_sector)?;
            // if we have a 1 sector write and a relative_end_offset of 0, that means we need to write until the end of the sector
            let actual_end_offset = if relative_end_offset == 0 { sector_size } else { relative_end_offset };
            first_sector.as_mut_slice()[relative_start_offset..actual_end_offset].copy_from_slice(&buf);
            return self.write_sector(start_sector, &first_sector);
        }

        let (mut first_sector, mut last_sector) = {
            let mut first = vec![0_u8; sector_size];
            self.read_sector(start_sector, &mut first)?;
            let mut last = vec![0_u8; sector_size];
            self.read_sector(end_sector, &mut last)?;
            (first, last)
        };

        // merge the write data into first[relative_offset..]
        first_sector.as_mut_slice()[relative_start_offset..].copy_from_slice(&buf[..sector_size - relative_start_offset]);
        // merge the write data into last[..relative_end_offset]
        last_sector.as_mut_slice()[..relative_end_offset].copy_from_slice(&buf[buf.len() - relative_end_offset..]);
        let in_between_data = &buf[sector_size - relative_start_offset..buf.len() - relative_end_offset];

        // write the first sector
        self.write_sector(start_sector, &first_sector)?;

        // write the in-between sectors
        in_between_data.chunks_exact(sector_size).enumerate().try_for_each(|(i, chunk)| {
            self.write_sector(start_sector + i + 1, chunk).map(|_| ())
        })?;

        // write the last sector
        self.write_sector(end_sector, &last_sector)?;

        Ok(buf.len())
    }
}
