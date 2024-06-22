#[macro_export]
macro_rules! check_is_implemented {
    ($implementor:ty, $t:tt) => {
        const _: () = {
            const fn check<T: $t>() {}
            check::<$implementor>();
        };
    };
}

#[macro_export]
macro_rules! bytefield_field_read {
    (u8, $offset:literal, $source:expr) => {
        $source[$offset]
    };
    (u16, $offset:literal, $source:expr) => { $crate::bytefield_field_read!(integer u16, $offset, $source) };
    (u32, $offset:literal, $source:expr) => { $crate::bytefield_field_read!(integer u32, $offset, $source) };
    (u64, $offset:literal, $source:expr) => { $crate::bytefield_field_read!(integer u64, $offset, $source) };
    (u128, $offset:literal, $source:expr) => { $crate::bytefield_field_read!(integer u128, $offset, $source) };
    (integer $typ:ty, $offset:literal, $source:expr) => {
        <$typ>::from_le_bytes(
            $source[$offset..$offset + core::mem::size_of::<$typ>()]
                .try_into()
                .unwrap(),
        )
    };
    ([u8; $len:literal], $offset:literal, $source:expr) => { $crate::bytefield_field_read!(u8_array [u8; $len], $offset, $source) };
    (u8_array $typ:ty, $offset:literal, $source:expr) => {
        {
            check_is_implemented!($typ, Default);
            let mut t: $typ = Default::default();
            t.copy_from_slice(&$source[$offset..$offset + core::mem::size_of::<$typ>()]);
            t
        }
    };
}

#[macro_export]
macro_rules! bytefield_field_write {
    (u8, $field:expr, $offset:literal, $target:expr) => {
        $target[$offset] = $field;
    };
    (u16, $field:expr, $offset:literal, $target:expr) => { $crate::bytefield_field_write!(integer u16, $field, $offset, $target) };
    (u32, $field:expr, $offset:literal, $target:expr) => { $crate::bytefield_field_write!(integer u32, $field, $offset, $target) };
    (u64, $field:expr, $offset:literal, $target:expr) => { $crate::bytefield_field_write!(integer u64, $field, $offset, $target) };
    (u128, $field:expr, $offset:literal, $target:expr) => { $crate::bytefield_field_write!(integer u128, $field, $offset, $target) };
    (integer $typ:ty, $field:expr, $offset:literal, $target:expr) => {
        $target[$offset..$offset + core::mem::size_of::<$typ>()].copy_from_slice(&$field.to_le_bytes());
    };
    ([u8; $len:literal], $field:expr, $offset:literal, $target:expr) => { $crate::bytefield_field_write!(u8_array [u8; $len], $field, $offset, $target) };
    (u8_array $typ:ty, $field:expr, $offset:literal, $target:expr) => {
        $target[$offset..$offset + core::mem::size_of::<$typ>()].copy_from_slice(&$field);
    };
}

#[macro_export]
macro_rules! bytefield {
    (
        $(#[$struct_attribute:meta])*
        pub struct $name:ident ( $source:ty ) {
            $(
                $(#[$field_attribute:meta])*
                $field:ident : $typ:tt /* must be a tt and not a ty because of some weird matching */ = $offset:literal
            ),* $(,)?
        }
    ) => {
        check_is_implemented!($source, Default);

        $(#[$struct_attribute])*
        pub struct $name {
            $(
            $(#[$field_attribute])*
            $field: $typ,
            )*
        }

        impl TryFrom<$source> for $name {
            type Error = ();

            fn try_from(value: $source) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl TryFrom<&$source> for $name {
            type Error = ();

            fn try_from(value: &$source) -> Result<Self, Self::Error> {
                Ok(
                    Self {
                        $(
                        $field: bytefield_field_read!($typ, $offset, value)
                        ),*
                    }
                )
            }
        }

        impl From<$name> for $source {
            fn from(value: $name) -> Self {
                Self::from(&value)
            }
        }

        impl From<&$name> for $source {
            fn from(value: &$name) -> Self {
                let mut r: Self = Default::default();
                $(
                bytefield_field_write!($typ, value.$field, $offset, r);
                )*
                r
            }
        }
    };
}
