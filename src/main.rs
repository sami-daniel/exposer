mod types;

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use anyhow::{Ok, Result};
use clap::Parser;

use crate::types::ext4_super_block;

#[derive(Parser, Debug)]
struct Args {
    /// Name of the partition to analyze
    #[arg(short, long)]
    partition: PathBuf,
}

macro_rules! print_ext4_fields {
    ($sb:expr, $($field:ident),+ $(,)?) => {
        $(
            println!("{}: {:?}", stringify!($field), $sb.$field);
        )+
    };
}

fn main() -> Result<()> {
    // https://elixir.bootlin.com/linux/v6.11.8/source/fs/ext4/ext4.h#L1304

    let args = Args::parse();

    if !args.partition.exists() {
        eprintln!("Invalid, partition does not exists");
    }

    let super_block = build_super_block(&args.partition)?;

    // FIXME:
    // Here we are just trusting that the device will follow the same
    // layout from there. We should study more about this to create a
    // safe representation of the disk
    print_ext4_fields!(
        super_block,
        s_inodes_count,
        s_magic,
        s_free_inodes_count,
        s_first_data_block,
        s_free_blocks_count_lo
    );

    Ok(())
}

fn build_super_block(file: &Path) -> Result<ext4_super_block> {
    let mut file = File::open(file)?;
    // I tried to use mmap, but it didnt work, idk why.
    // I was getting 0 bytes every time. We could just
    // seek and eveything works so...
    file.seek(SeekFrom::Start(1024))?;

    let mut buf = [0u8; 1024];
    file.read_exact(&mut buf)?;

    let super_block: &ext4_super_block = bytemuck::from_bytes(&buf);

    Ok(super_block.clone())
}
