# mkfs
`mkfs` written in pure Rust.

## Motivation
I needed a library that could work with file systems in `no_std` environments for my kernel.
Additionally, for testing, I needed a library that I could use easily to create and edit
new file system images, and that I could use in my `build.rs` file. Those things can be done
by `mkfs`. However, `mkfs` is not available out of the box on all platforms, so I opted for
something that can be integrated in the `cargo` build process.

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
