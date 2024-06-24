use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};

use bitflags::bitflags;

use filesystem::BlockDevice;

use crate::{
    bytefield, bytefield_field_read, bytefield_field_write, check_is_implemented, Directory,
    Ext2Fs, Inode, InodeAddress, Type,
};
use crate::error::Error;
use crate::superblock::RequiredFeatures;

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

        for addr in dir.direct_ptrs().filter_map(|v| v) {
            let mut data = vec![0_u8; block_size];
            self.read_block(addr, &mut data)
                .map_err(|_| Error::DeviceRead)?;

            let mut offset = 0;
            while offset < block_size - 8 {
                let dir_entry = DirEntry::from(dir_entries_have_type, &data[offset..]);
                // we don't need to align the offset, as there must be no space between entries
                offset += dir_entry.total_size as usize;
                entries.push(dir_entry);
            }
        }

        // TODO: handle indirect ptrsa

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
        self.read_inode(entry.inode)
    }

    pub fn add_entry_to_dir(
        &mut self,
        dir: &mut Directory,
        name: &str,
        inode_address: InodeAddress,
        typ: DirType,
    ) -> Result<(), Error> {
        let block_size = self.superblock.block_size() as usize;
        let dir_entries_have_type = self
            .superblock
            .required_features()
            .contains(RequiredFeatures::DIRECTORY_ENTRIES_HAVE_TYPE);

        let inode = &mut dir.inode_mut();

        // compute the size of the directory entry that we need
        let required_size = DirEntry::size(name.len() as u16);

        // find a free slot and insert the entry
        for block in inode.direct_ptrs().filter_map(|v| v) {
            let mut block_data = vec![0_u8; block_size];
            self.read_block(block, &mut block_data)?;

            // In the block, we need to find the first entry, where
            // entry.total_size - DirEntry::size(..., entry.name_length) >= required_size,
            // adapt that entry and store our entry there.
            let mut offset = 0;
            while offset < block_size - 8 {
                debug_assert_eq!(offset % 4, 0, "offset is not aligned");

                let mut entry = DirEntry::from(dir_entries_have_type, &block_data[offset..]);
                let entry_size = DirEntry::size(entry.name_length);
                if entry.total_size >= required_size + entry_size {
                    // we found a slot that is big enough

                    let old_total_size = entry.total_size;
                    entry.total_size = entry_size; // resize the old entry

                    // merge the old entry back into the block data
                    let entry_serialized = entry.serialize(dir_entries_have_type);
                    block_data[offset..offset + entry_serialized.len()].copy_from_slice(&entry_serialized);

                    let new_entry_total_size = old_total_size - entry_size;
                    let new_entry_offset = offset + entry_size as usize;
                    debug_assert_eq!(new_entry_offset % 4, 0, "new entry offset is not aligned");

                    let new_entry = DirEntry {
                        inode: inode_address,
                        total_size: new_entry_total_size,
                        name_length: name.len() as u16,
                        type_indicator: if dir_entries_have_type { Some(typ) } else { None },
                        name_bytes: name.as_bytes().to_vec(),
                    };

                    // merge the new entry into the block data
                    let new_entry_serialized = new_entry.serialize(dir_entries_have_type);
                    block_data[new_entry_offset..new_entry_offset + new_entry_serialized.len()].copy_from_slice(&new_entry_serialized);

                    // write the block back to the device
                    self.write_block(block, &block_data)?;

                    return Ok(());
                }

                offset += entry.total_size as usize;
            }
        }

        todo!("add_entry_to_dir with indirect pointers")
    }
}

pub struct DirEntry {
    inode: InodeAddress,
    total_size: u16,
    name_length: u16,
    type_indicator: Option<DirType>,
    name_bytes: Vec<u8>,
}

impl DirEntry {
    const fn size(name_length: u16) -> u16 {
        let unaligned_size = 4 + // inode
            2 + // total_size
            2 + // name_length and type_indicator
            name_length;
        // align up to 4 byte
        (unaligned_size + 3) & !3
    }

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
            inode: InodeAddress::new(arr.inode).unwrap(),
            total_size: arr.total_size,
            name_length,
            type_indicator,
            name_bytes,
        }
    }

    pub fn serialize(self, dir_entries_have_type: bool) -> Vec<u8> {
        let mut result = Vec::with_capacity(Self::size(self.name_length) as usize);

        let mut entry_no_name = DirEntryNoName::try_from([0; 8]).unwrap();
        entry_no_name.inode = self.inode.into();
        entry_no_name.total_size = self.total_size;
        entry_no_name.name_length_lsb = self.name_length as u8;
        if dir_entries_have_type {
            entry_no_name.type_indicator_or_name_length_msb = self.type_indicator.unwrap().bits();
        } else {
            entry_no_name.type_indicator_or_name_length_msb = (self.name_length >> 8) as u8; // TODO: check for data loss
        }

        result.extend_from_slice(&<[u8; 8]>::from(entry_no_name));
        result.extend_from_slice(&self.name_bytes);
        result
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

    pub fn inode(&self) -> InodeAddress {
        self.inode
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

impl From<Type> for DirType {
    fn from(value: Type) -> Self {
        match value {
            Type::RegularFile => Self::RegularFile,
            Type::Directory => Self::Directory,
            Type::CharacterDevice => Self::CharacterDevice,
            Type::BlockDevice => Self::BlockDevice,
            Type::FIFO => Self::FIFO,
            Type::UnixSocket => Self::UnixSocket,
            Type::SymLink => Self::SymLink,
            _ => panic!("invalid type")
        }
    }
}