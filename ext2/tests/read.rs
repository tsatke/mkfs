use ext2::{DirType, Ext2Fs};
use filesystem::MemoryBlockDevice;
use std::env;
use std::fs::File;
use std::io::Read;

#[test]
fn test_read() {
    do_test_read(512);
}

#[test]
fn test_read_tiny_device_block_size() {
    do_test_read(1);
}

#[test]
fn test_read_small_device_block_size() {
    do_test_read(32);
}

#[test]
fn test_read_large_device_block_size() {
    do_test_read(32768); // 32 KiB
}

#[test]
fn test_read_huge_device_block_size() {
    do_test_read(1048576); // 1 MiB
}

fn do_test_read(block_size: usize) {
    let mut image = env::current_dir().unwrap();
    image.push(&"tests/filesystems/read.img");

    let mut data = Vec::new();

    let mut file = File::open(&image).unwrap();
    assert_eq!(1048576_u64, file.metadata().unwrap().len());
    file.read_to_end(&mut data).unwrap();

    // This block size is the device block size, not the ext2 block size. Therefor, all values must
    // work, as long as the device size is divisible by it.
    let device = MemoryBlockDevice::try_new(block_size, data).unwrap();

    let fs = Ext2Fs::try_new(device).unwrap();
    let root = fs.read_root_inode().unwrap();
    let entries = fs.list_dir(&root).unwrap();

    let expected_entries = [
        (2, ".", DirType::Directory),
        (2, "..", DirType::Directory),
        (11, "lost+found", DirType::Directory),
        (12, "hello.txt", DirType::RegularFile),
        (13, "some", DirType::Directory),
    ];
    assert_eq!(expected_entries.len(), entries.len());
    for (inode, name, typ) in expected_entries {
        assert!(entries
            .iter()
            .find(|&entry| {
                entry.inode() == inode
                    && entry.name().is_some_and(|n| n == name)
                    && entry.typ() == Some(typ)
            })
            .is_some());
    }
}
