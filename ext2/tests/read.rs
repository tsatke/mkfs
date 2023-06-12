use ext2::{Ext2Fs, Inode};
use filesystem::MemoryBlockDevice;
use std::env;
use std::fs::File;
use std::io::Read;

#[test]
fn test_read() {
    let mut image = env::current_dir().unwrap();
    image.push(&"tests/filesystems/read.img");

    let mut data = Vec::new();

    let mut file = File::open(&image).unwrap();
    file.read_to_end(&mut data).unwrap();

    let device = MemoryBlockDevice::try_new(512, data).unwrap();

    let fs = Ext2Fs::try_new(device).unwrap();
    let root = fs.read_root_inode().unwrap();
    let entries = fs.list_dir(&root).unwrap();
    // entries.iter().for_each(|entry| println!("{:?}", entry));
    assert_eq!(5, entries.len());
}
