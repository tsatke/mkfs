use core::num::NonZeroU32;
use core::ops::Deref;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
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
}