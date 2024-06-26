use std::env::temp_dir;
use std::fs;
use std::path::{Path, PathBuf};

use rand::distributions::Alphanumeric;
use rand::Rng;

#[macro_export]
macro_rules! cow_fs {
    ($path:literal, $device_sector_size:expr) => {
        {
            let image_data = common::load_copy_of_image($path);
            let device = MemoryBlockDevice::try_new($device_sector_size, image_data).unwrap();
            Ext2Fs::try_new(device).unwrap()
        }
    };
}

#[macro_export]
macro_rules! generate_tests {
    ($test_fn:ident : $($size:literal - $name:ident),*,) => {
        const _: &dyn Fn(usize) = &$test_fn;
        $(
            #[test]
            fn $name() {
                $test_fn($size);
            }
        )*
    };
}

pub fn copy_test_image_for_use(test_image: impl AsRef<Path>) -> PathBuf {
    let temp_dir = temp_dir();
    let name = rand::thread_rng().sample_iter(&Alphanumeric).take(10).map(char::from).collect::<String>();
    let temp_image = temp_dir.join(format!("{}.img", name));

    fs::copy(test_image, &temp_image).unwrap();
    temp_image
}

pub fn load_copy_of_image(test_image: impl AsRef<Path>) -> Vec<u8> {
    let copy = copy_test_image_for_use(test_image);
    println!("Using image: {:?}", copy);
    fs::read(&copy).unwrap()
}