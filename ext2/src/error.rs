use core::fmt::{Debug, Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    InvalidSuperblock,
    UnableToReadSuperblock,
    UnableToReadBlockGroupDescriptorTable,
    NotDirectory,
    NotRegularFile,
    DeviceRead,
    InvalidInodeAddress(u32),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl core::error::Error for Error {}
