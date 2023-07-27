use crate::{
    bytefield, bytefield_field_read, bytefield_field_write, check_is_implemented, BlockAddress,
};
use bitflags::bitflags;
use core::ops::{Deref, DerefMut};

macro_rules! inode_type {
    ($name:ident, $typ:expr) => {
        #[derive(Debug)]
        pub struct $name(Inode);

        impl Deref for $name {
            type Target = Inode;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl TryFrom<Inode> for $name {
            type Error = Inode;

            fn try_from(v: Inode) -> Result<Self, Self::Error> {
                if v.typ() == $typ {
                    Ok(Self(v))
                } else {
                    Err(v)
                }
            }
        }

        impl From<$name> for Inode {
            fn from(v: $name) -> Self {
                v.0
            }
        }
    };
}

inode_type!(Fifo, Type::FIFO);
inode_type!(CharacterDeviceFile, Type::CharacterDevice);
inode_type!(Directory, Type::Directory);
inode_type!(BlockDeviceFile, Type::BlockDevice);
inode_type!(RegularFile, Type::RegularFile);
inode_type!(SymLink, Type::SymLink);
inode_type!(UnixSocket, Type::UnixSocket);

pub struct InodeRawArray([u8; 128]);

impl From<[u8; 128]> for InodeRawArray {
    fn from(value: [u8; 128]) -> Self {
        Self(value)
    }
}

impl Default for InodeRawArray {
    fn default() -> Self {
        InodeRawArray([0_u8; 128])
    }
}

impl Deref for InodeRawArray {
    type Target = [u8; 128];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InodeRawArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

bytefield! {
    #[derive(Debug)]
    pub struct Inode (InodeRawArray) {
        type_and_perm: u16 = 0,
        user_id: u16 = 2,
        byte_size_lower: u32 = 4,
        last_access_time: u32 = 8,
        creation_time: u32 = 12,
        last_modification_time: u32 = 16,
        deletion_time: u32 = 20,
        group_id: u16 = 24,
        num_hard_links: u16 = 26,
        num_disk_sectors: u32 = 28,
        flags: u32 = 32,
        os_val_1: [u8; 4] = 36,
        direct_block_ptr_0: u32 = 40,
        direct_block_ptr_1: u32 = 44,
        direct_block_ptr_2: u32 = 48,
        direct_block_ptr_3: u32 = 52,
        direct_block_ptr_4: u32 = 56,
        direct_block_ptr_5: u32 = 60,
        direct_block_ptr_6: u32 = 64,
        direct_block_ptr_7: u32 = 68,
        direct_block_ptr_8: u32 = 72,
        direct_block_ptr_9: u32 = 76,
        direct_block_ptr_10: u32 = 80,
        direct_block_ptr_11: u32 = 84,
        singly_indirect_block_ptr: u32 = 88,
        doubly_indirect_block_ptr: u32 = 92,
        triply_indirect_block_ptr: u32 = 96,
        generation: u32 = 100,
        extended_attribute_block: u32 = 104,
        byte_size_upper_or_dir_acl: u32 = 108,
        fragment_block_address: u32 = 112,
        os_val_2: [u8; 12] = 116,
    }
}

impl Inode {
    pub fn typ(&self) -> Type {
        Type::from_bits_truncate(self.type_and_perm)
    }

    pub fn perm(&self) -> Permissions {
        Permissions::from_bits_truncate(self.type_and_perm)
    }

    pub fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.flags)
    }

    pub fn direct_ptr(&self, index: usize) -> Option<BlockAddress> {
        BlockAddress::new(match index {
            0 => self.direct_block_ptr_0,
            1 => self.direct_block_ptr_1,
            2 => self.direct_block_ptr_2,
            3 => self.direct_block_ptr_3,
            4 => self.direct_block_ptr_4,
            5 => self.direct_block_ptr_5,
            6 => self.direct_block_ptr_6,
            7 => self.direct_block_ptr_7,
            8 => self.direct_block_ptr_8,
            9 => self.direct_block_ptr_9,
            10 => self.direct_block_ptr_10,
            11 => self.direct_block_ptr_11,
            _ => panic!("direct pointer {} does not exist", index),
        })
    }

    pub fn single_indirect_ptr(&self) -> Option<BlockAddress> {
        BlockAddress::new(self.singly_indirect_block_ptr)
    }

    pub fn len(&self) -> usize {
        if self.typ() == Type::Directory {
            self.byte_size_lower as usize
        } else {
            self.byte_size_lower as usize | ((self.byte_size_upper_or_dir_acl as usize) << 32)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct Type: u16 {
        const FIFO = 0x1000;
        const CharacterDevice = 0x2000;
        const Directory = 0x4000;
        const BlockDevice = 0x6000;
        const RegularFile = 0x8000;
        const SymLink = 0xA000;
        const UnixSocket = 0xC000;
    }
}

bitflags! {
    pub struct Permissions: u16 {
        const OtherExec = 0x001;
        const OtherWrite = 0x002;
        const OtherRead = 0x004;
        const GroupExec = 0x008;
        const GroupWrite = 0x010;
        const GroupRead = 0x020;
        const UserExec = 0x040;
        const UserWrite = 0x080;
        const UserRead = 0x100;
        const Sticky = 0x200;
        const SetGID = 0x400;
        const SetUID = 0x800;
    }
}

bitflags! {
    pub struct Flags: u32 {
        const SecureDelete = 0x00000001;
        const CopyOnDelete = 0x00000002;
        const FileCompression = 0x00000004;
        const NoCache = 0x00000008;
        const ImmutableFile = 0x00000010;
        const AppendOnly = 0x00000020;
        const ExcludeFromDump = 0x00000040;
        const KeepLastAccessedTime = 0x00000080;
        const HashIndexedDirectory = 0x00010000;
        const AfsDirectory = 0x00020000;
        const JournalFileData = 0x00040000;
    }
}
