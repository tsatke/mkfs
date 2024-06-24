use filesystem::BlockDevice;

use crate::{Directory, Error, Ext2Fs, Inode, InodeAddress, RegularFile, Type};

impl<T> Ext2Fs<T>
where
    T: BlockDevice,
{
    pub fn create_inode(&mut self, parent: &mut Directory, name: &str, typ: Type) -> Result<(InodeAddress, Inode), Error> {
        let inode_address = self.allocate_inode()?.ok_or(Error::NoSpace)?;
        let inode = Inode::new(typ);

        self.write_inode(inode_address, &inode)?;

        self.add_entry_to_dir(parent, name, inode_address, inode.typ().into())?;

        Ok((inode_address, inode))
    }

    pub fn create_regular_file(&mut self, parent: &mut Directory, name: &str) -> Result<RegularFile, Error> {
        self.create_inode(parent, name, Type::RegularFile)
            .map(|v| v.try_into().unwrap()) // if we don't get an inode with type RegularFile, something is really broken
    }
}