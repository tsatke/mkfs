# mkfs
`mkfs` written in pure Rust.

## Supported file systems
- [ ] ext2 (in progress)
- [ ] ext3
- [ ] ext4
- [ ] and more...

## Features
- Command line tool to create and edit file system images
- Usable as library (see below)
- Compatibility with `no_std` crates (you need to implement the `BlockDevice` trait for your data source)

# Usage

## Command line
(subject to change)
```shell
# create a 1MiB ext2 file system in fs.img with the structure of my_directory
mkfs ext2 create --size 1MB --out fs.img --in-dir ./my_directory
```

## Library
(not implemented yet)

### Use `mkfs` directly
Works nicely in `build.rs` files.

```rust
mkfs::ext2::create(
    Ext2CreateOption {
        out: PathBuf::from("fs.img"),
        // all the other options
    }
);
```

### Use with a `no_std` crate
```rust
// something like the following, still trying to figure this out
let block_device: MyBlockDevice = todo!("implement the BlockDevice trait");

let fs = Ext2Fs::new(block_device);
let file = fs.create_file("hello_world.txt");
...
```