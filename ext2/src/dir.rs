use crate::error::Error;
use crate::superblock::RequiredFeatures;
use crate::{
    bytefield, bytefield_field_read, bytefield_field_write, check_is_implemented, Directory,
    Ext2Fs, Inode, InodeAddress, Type,
};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use filesystem::BlockDevice;

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    // TODO: make this return some Result<impl Iterator<Item=DirEntry>, Error>
    pub fn list_dir(&self, dir: &Inode) -> Result<Vec<DirEntry>, Error> {
        if dir.typ() != Type::Directory {
            return Err(Error::NotDirectory);
        }

        let mut entries = Vec::new();
        let block_size = self.superblock.block_size() as usize;
        let dir_entries_have_type = self
            .superblock
            .required_features()
            .contains(RequiredFeatures::DIRECTORY_ENTRIES_HAVE_TYPE);

        for addr in (0_usize..12).filter_map(|i| dir.direct_ptr(i)) {
            let mut data = vec![0_u8; block_size];
            self.read_block(addr, &mut data)
                .map_err(|_| Error::DeviceRead)?;

            let mut offset = 0;
            while offset < block_size - 8 {
                let dir_entry = DirEntry::from(dir_entries_have_type, &data[offset..]);
                offset += dir_entry.total_size as usize;
                // we don't need to align the offset, as there must be no space between entries
                if dir_entry.inode().is_none() {
                    // entry invalid, move on
                    continue;
                }
                entries.push(dir_entry);
            }
        }

        // TODO: handle indirect ptrs

        Ok(entries)
    }

    pub fn lookup_dir_entry<P>(
        &self,
        dir: &Directory,
        p: P,
    ) -> Result<Option<(InodeAddress, Inode)>, Error>
    where
        P: FnMut(&DirEntry) -> bool,
    {
        self.list_dir(dir)?
            .into_iter()
            .find(p)
            .map(|e| self.resolve_dir_entry(e))
            .transpose()
    }

    pub fn resolve_dir_entry(&self, entry: DirEntry) -> Result<(InodeAddress, Inode), Error> {
        let address =
            InodeAddress::new(entry.inode).ok_or(Error::InvalidInodeAddress(entry.inode))?;
        self.read_inode(address)
    }
}

pub struct DirEntry {
    inode: u32,
    total_size: u16,
    name_length: u16,
    type_indicator: Option<DirType>,
    name_bytes: Vec<u8>,
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

    pub fn name(&self) -> Option<&str> {
        match core::str::from_utf8(&self.name_bytes) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    pub fn typ(&self) -> Option<DirType> {
        self.type_indicator
    }

    pub fn inode(&self) -> Option<InodeAddress> {
        InodeAddress::new(self.inode)
    }
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
