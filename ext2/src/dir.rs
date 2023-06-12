use crate::error::Error;
use crate::superblock::RequiredFeatures;
use crate::{
    bytefield, bytefield_field_read, bytefield_field_write, check_is_implemented, Ext2Fs, Inode,
    Type,
};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::{bitflags, Flags};
use core::fmt::{Debug, Formatter};
use core::iter::FusedIterator;
use filesystem::BlockDevice;

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    pub fn list_dir(&self, inode: &Inode) -> Result<Vec<DirEntry>, Error> {
        if !inode.typ().contains(Type::Directory) {
            return Err(Error::NotDirectory);
        }

        let mut entries = Vec::new();
        let block_size = self.block_device.block_size();
        let dir_entries_have_type = self
            .superblock
            .required_features()
            .contains(RequiredFeatures::DIRECTORY_ENTRIES_HAVE_TYPE);

        for block in (0_usize..12)
            .map(|i| inode.direct_ptr(i))
            .filter(|&p| p != 0)
        {
            let mut data = vec![0_u8; block_size];
            let addr = self.get_block_address(block);
            self.block_device
                .read_at(addr, &mut data)
                .map_err(|_| Error::DeviceRead)?;

            let mut offset = 0;
            while offset < block_size - 8 {
                let dir_entry = DirEntry::from(dir_entries_have_type, &data[offset..]);
                offset += 8 + dir_entry.name_length as usize;
                offset = (offset + 3) & !0x03; // align to 4
                if dir_entry.inode == 0 {
                    // entry invalid, move on
                    continue;
                }
                entries.push(dir_entry);
            }
        }

        Ok(entries)
    }
}

pub struct DirEntry {
    inode: u32,
    total_size: u16,
    name_length: u16,
    type_indicator: Option<DirType>,
    name_bytes: Vec<u8>,
}

impl Debug for DirEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut debug_struct = f.debug_struct("DirEntry");
        debug_struct
            .field("inode", &self.inode)
            .field("total_size", &self.total_size)
            .field("name_length", &self.name_length);
        if let Some(type_indicator) = &self.type_indicator {
            debug_struct.field("type_indicator", &type_indicator);
        }
        debug_struct.field(
            "name",
            &core::str::from_utf8(&self.name_bytes[..self.name_length as usize]),
        );
        debug_struct.field("name_bytes", &&self.name_bytes[..self.name_length as usize]);

        debug_struct.finish()
    }
}

impl DirEntry {
    fn from(dir_entries_have_type: bool, value: &[u8]) -> Self {
        debug_assert!(
            value.len() >= 8,
            "need at least 8 byte, but got {}",
            value.len()
        );

        let arr = DirEntryNoName::try_from(&value[0..8].try_into().unwrap()).unwrap();
        let name_length = if dir_entries_have_type {
            arr.name_length_lsb as u16
        } else {
            arr.name_length_lsb as u16 | ((arr.type_indicator_or_name_length_msb as u16) << 8)
        };
        let type_indicator = if dir_entries_have_type {
            Some(DirType::from_bits_truncate(
                arr.type_indicator_or_name_length_msb,
            ))
        } else {
            None
        };
        let name_bytes = value[8..8 + name_length as usize].to_vec();
        Self {
            inode: arr.inode,
            total_size: arr.total_size,
            name_length,
            type_indicator,
            name_bytes,
        }
    }
}

bytefield! {
    pub struct DirEntryNoName ([u8; 8]) {
        inode: u32 = 0,
        total_size: u16 = 4,
        name_length_lsb: u8 = 6,
        type_indicator_or_name_length_msb: u8 = 7,
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct DirType: u8 {
        const RegularFile = 1;
        const Directory = 2;
        const CharacterDevice = 3;
        const BlockDevice = 4;
        const FIFO = 5;
        const UnixSocket = 6;
        const SymLink = 7;
    }
}
