use crate::BlockDevice;

pub struct MemoryBlockDevice<T> {
    sector_size: usize,
    data: T,
}

impl<T> MemoryBlockDevice<T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    pub fn try_new(sector_size: usize, data: T) -> Option<Self> {
        let mut data = data;
        if data.as_mut().len() % sector_size != 0 {
            return None;
        }
        Some(Self { sector_size, data })
    }
}

impl<T> BlockDevice for MemoryBlockDevice<T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    type Error = ();

    fn sector_size(&self) -> usize {
        self.sector_size
    }

    fn sector_count(&self) -> usize {
        self.data.as_ref().len()
    }

    fn read_sector(&self, sector_index: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        debug_assert_eq!(self.sector_size(), buf.len());
        let start_offset = sector_index * self.sector_size();
        let end_offset = start_offset + self.sector_size();
        let data = self.data.as_ref();
        buf.copy_from_slice(&data[start_offset..end_offset]);
        Ok(buf.len())
    }

    fn write_sector(&mut self, sector_index: usize, buf: &[u8]) -> Result<usize, Self::Error> {
        debug_assert_eq!(self.sector_size(), buf.len());
        let start_offset = sector_index * self.sector_size();
        let end_offset = start_offset + self.sector_size();
        let data = self.data.as_mut();
        data[start_offset..end_offset].copy_from_slice(buf);
        Ok(buf.len())
    }
}

#[cfg(test)]
mod tests {
    use crate::block::mem::MemoryBlockDevice;
    use crate::BlockDevice;
    use alloc::vec;

    #[test]
    fn test_read_at_short() {
        let data = vec![1_u8, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8];
        let device = MemoryBlockDevice::try_new(2, data).unwrap();

        let mut buf = [0_u8; 2];
        device.read_at(1, &mut buf).unwrap();
        assert_eq!([1, 2], buf);

        device.read_at(2, &mut buf).unwrap();
        assert_eq!([2, 2], buf);

        device.read_at(5, &mut buf).unwrap();
        assert_eq!([3, 4], buf);

        device.read_at(14, &mut buf).unwrap();
        assert_eq!([8, 8], buf);

        buf.fill(0);
        device.read_at(15, &mut buf[..1]).unwrap();
        assert_eq!([8, 0], buf);
    }

    #[test]
    fn test_read_at() {
        let data = vec![1_u8, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8];
        let device = MemoryBlockDevice::try_new(2, data).unwrap();

        let mut buf = [0_u8; 8];
        device.read_at(3, &mut buf).unwrap();
        assert_eq!([2, 3, 3, 4, 4, 5, 5, 6], buf);
    }

    #[test]
    fn test_read_at_zero_sized() {
        let data = vec![1_u8, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8];
        let device = MemoryBlockDevice::try_new(2, data).unwrap();

        let mut buf = [0_u8; 0];
        device.read_at(3, &mut buf).unwrap();
        let expected = [0_u8; 0];
        assert_eq!(expected, buf);
    }
}
