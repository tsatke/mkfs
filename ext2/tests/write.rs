use ext2::{Error, Ext2Fs};
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
    let mut fs = cow_fs!("tests/filesystems/empty.img", sector_size);

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

generate_tests!(
    test_create_files:
    512 - test_create_files_standard,
    1 - test_create_files_tiny,
    32 - test_create_files_small,
    32768 - test_create_files_large,
    1048576 - test_create_files_huge,
);

fn test_create_files(sector_size: usize) {
    let mut fs = cow_fs!("tests/filesystems/empty.img", sector_size);

    let mut root = fs.read_root_inode().unwrap();
    for i in 0..25 {
        let file_name = format!("file_{}.txt", i);
        let file = fs.create_regular_file(&mut root, &file_name).unwrap();
        assert!(fs.list_dir(&root).unwrap().iter().find(|e| e.name() == Some(&file_name)).is_some());
        assert_eq!(file.len(), 0);
    }
}

generate_tests!(
    test_create_file_collision:
    512 - test_create_file_collision_standard,
    1 - test_create_file_collision_tiny,
    32 - test_create_file_collision_small,
    32768 - test_create_file_collision_large,
    1048576 - test_create_file_collision_huge,
);

fn test_create_file_collision(sector_size: usize) {
    let mut fs = cow_fs!("tests/filesystems/empty.img", sector_size);

    let mut root = fs.read_root_inode().unwrap();
    let file_name = "file.txt";
    let file = fs.create_regular_file(&mut root, file_name).unwrap();
    assert!(fs.list_dir(&root).unwrap().iter().find(|e| e.name() == Some(file_name)).is_some());
    assert_eq!(file.len(), 0);

    let mut root = fs.read_root_inode().unwrap();
    let result = fs.create_regular_file(&mut root, file_name);
    assert_eq!(result.unwrap_err(), Error::EntryExists);
}