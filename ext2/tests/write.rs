use ext2::Ext2Fs;
use filesystem::MemoryBlockDevice;

mod common;

generate_tests!(
    test_create_and_write_file:
    512 - test_create_and_write_file_standard,
    1 - test_create_and_write_file_tiny,
    32 - test_create_and_write_file_small,
    32768 - test_create_and_write_file_large,
    1048576 - test_create_and_write_file_huge,
);

fn test_create_and_write_file(sector_size: usize) {
    let image_data = common::load_copy_of_image("tests/filesystems/empty.img");
    let device = MemoryBlockDevice::try_new(sector_size, image_data).unwrap();
    let mut fs = Ext2Fs::try_new(device).unwrap();

    let file_name = "my_file.txt";
    let mut root = fs.read_root_inode().unwrap();
    let mut file = fs.create_regular_file(&mut root, file_name).unwrap();
    assert!(fs.list_dir(&root).unwrap().iter().find(|e| e.name() == Some(file_name)).is_some());
    assert_eq!(file.len(), 0);

    let data = b"Hello, world!";
    // write `data` until all direct pointers are used
    for i in 0..((1024 * 12) / data.len()) {
        assert_eq!(fs.write_to_file(&mut file, i * data.len(), data).unwrap(), data.len());
    }
}