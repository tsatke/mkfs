use core::num::NonZeroU32;
use core::ops::Deref;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct InodeAddress(NonZeroU32);

impl Deref for InodeAddress {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NonZeroU32> for InodeAddress {
    fn from(value: NonZeroU32) -> Self {
        Self(value)
    }
}

impl From<InodeAddress> for u32 {
    fn from(value: InodeAddress) -> u32 {
        value.0.get()
    }
}

impl InodeAddress {
    pub const fn new(n: u32) -> Option<Self> {
        let nzu32 = NonZeroU32::new(n);
        match nzu32 {
            None => None,
            Some(v) => Some(Self(v)),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct BlockAddress(NonZeroU32);

impl Deref for BlockAddress {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NonZeroU32> for BlockAddress {
    fn from(value: NonZeroU32) -> Self {
        Self(value)
    }
}

impl BlockAddress {
    pub const fn new(n: u32) -> Option<Self> {
        let nzu32 = NonZeroU32::new(n);
        match nzu32 {
            None => None,
            Some(v) => Some(Self(v)),
        }
    }

    pub fn into_u32(self) -> u32 {
        self.0.get()
    }
}