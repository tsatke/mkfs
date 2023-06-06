use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Work with an ext2 file system
    Ext2(Ext2),
    /// Work with an ext3 file system
    Ext3(Ext3),
    /// Work with an ext4 file system
    Ext4(Ext4),
}

#[derive(Debug, Parser)]
pub struct Ext2 {
    #[arg(long, help = "The file that the file system will be written into")]
    out: PathBuf,
    #[arg(short, long, help = "Overwrite the file if it already exists")]
    force: bool,
}

#[derive(Debug, Parser)]
pub struct Ext3 {}

#[derive(Debug, Parser)]
pub struct Ext4 {}

fn main() {
    let args = Args::parse();
    println!("args={:?}", args);

    match args.subcommand {
        Subcommand::Ext2(ext2) => handle_ext2(ext2),
        Subcommand::Ext3(ext3) => handle_ext3(ext3),
        Subcommand::Ext4(ext4) => handle_ext4(ext4),
    }
}

fn handle_ext2(ext2: Ext2) {}

fn handle_ext3(ext3: Ext3) {}

fn handle_ext4(ext4: Ext4) {}
