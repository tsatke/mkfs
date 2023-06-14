use ext2::{DirType, Ext2Fs, RegularFile, Type};
use filesystem::MemoryBlockDevice;
use std::env;
use std::fs::File;
use std::io::Read;

#[test]
fn test_list_directory() {
    do_test_list_directory(512);
}

#[test]
fn test_list_directory_tiny_device_block_size() {
    do_test_list_directory(1);
}

#[test]
fn test_list_directory_small_device_block_size() {
    do_test_list_directory(32);
}

#[test]
fn test_list_directory_large_device_block_size() {
    do_test_list_directory(32768); // 32 KiB
}

#[test]
fn test_list_directory_huge_device_block_size() {
    do_test_list_directory(1048576); // 1 MiB
}

fn do_test_list_directory(sector_size: usize) {
    let mut image = env::current_dir().unwrap();
    image.push(&"tests/filesystems/read.img");

    let mut data = Vec::new();

    let mut file = File::open(&image).unwrap();
    assert_eq!(1048576_u64, file.metadata().unwrap().len());
    file.read_to_end(&mut data).unwrap();

    // This sector size is the device sector/block size, not the ext2 block size. Therefor, all values must
    // work, as long as the device size is divisible by it.
    let device = MemoryBlockDevice::try_new(sector_size, data).unwrap();

    let fs = Ext2Fs::try_new(device).unwrap();
    let root = fs.read_root_inode().unwrap().try_into().unwrap();
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
                entry.inode().unwrap().get() == inode
                    && entry.name().is_some_and(|n| n == name)
                    && entry.typ() == Some(typ)
            })
            .is_some());
    }
}

#[test]
fn test_read_file() {
    do_test_read_file(512);
}

#[test]
fn test_read_file_tiny_device_block_size() {
    do_test_read_file(1);
}

#[test]
fn test_read_file_small_device_block_size() {
    do_test_read_file(32);
}

#[test]
fn test_read_file_large_device_block_size() {
    do_test_read_file(32768); // 32 KiB
}

#[test]
fn test_read_file_huge_device_block_size() {
    do_test_read_file(1048576); // 1 MiB
}

fn do_test_read_file(sector_size: usize) {
    let mut image = env::current_dir().unwrap();
    image.push(&"tests/filesystems/read.img");

    let mut data = Vec::new();

    let mut file = File::open(&image).unwrap();
    assert_eq!(1048576_u64, file.metadata().unwrap().len());
    file.read_to_end(&mut data).unwrap();

    let device = MemoryBlockDevice::try_new(sector_size, data).unwrap();

    let fs = Ext2Fs::try_new(device).unwrap();
    let root = fs.read_root_inode().unwrap().try_into().unwrap();

    let hello_txt: RegularFile = fs
        .lookup_dir_entry(&root, |e| e.name().is_some_and(|n| n == "hello.txt"))
        .expect("lookup_dir_entry failed")
        .expect("hello.txt not found")
        .try_into()
        .unwrap();
    assert_eq!(14, hello_txt.len());
    assert_eq!(Type::RegularFile, hello_txt.typ());

    {
        // read the whole file
        let mut hello_txt_data = [0xFC_u8; 20];
        let read_bytes = fs
            .read_from_file(&hello_txt, 0, &mut hello_txt_data)
            .expect("read_from_file failed");
        assert_eq!(14, read_bytes); // check that the right amount of bytes have been read
        assert_eq!(b"Hello, World!\n", &hello_txt_data[0..14]); // check that the read bytes are correct
        assert_eq!([0xFC_u8; 5], hello_txt_data[15..20]); // check that the remaining part of the buffer remains untouched
    }
    {
        // read a part of the file
        let mut hello_txt_data = [0_u8; 5];
        let read_bytes = fs
            .read_from_file(&hello_txt, 7, &mut hello_txt_data)
            .expect("read_from_file failed");
        assert_eq!(5, read_bytes);
        assert_eq!(b"World", &hello_txt_data[..]);
    }
}
