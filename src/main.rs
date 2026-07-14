mod ext4;
mod types;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::ext4::Volume;

#[derive(Parser, Debug)]
struct Args {
    /// The ext4 volume to inspect
    partition: PathBuf,

    /// Show one inode by number, or every allocated inode when given with no number
    #[arg(long, value_name = "N", num_args = 0..=1, default_missing_value = "0")]
    inode: Option<u32>,

    /// List a directory by inode number. Defaults to the root directory
    #[arg(long, value_name = "N", num_args = 0..=1, default_missing_value = "2")]
    ls: Option<u32>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut volume = Volume::open(&args.partition)?;

    if let Some(number) = args.ls {
        let dir = volume.read_inode(number)?;
        println!("{:>8}  {:<16}  {}", "inode", "file type", "name");
        println!("{}", "-".repeat(40));

        for entry in volume.read_dir(&dir)? {
            println!("{entry}");
        }
        return Ok(());
    }

    println!("{}", volume.sb);
    println!();
    for group in volume.read_groups()? {
        println!("{group}");
    }

    println!();
    match args.inode {
        Some(0) => list_inodes(&mut volume)?,
        Some(number) => println!("{}", volume.read_inode(number)?),
        _ => {}
    }

    Ok(())
}

fn list_inodes(volume: &mut Volume) -> Result<()> {
    for number in 1..=volume.sb.inodes_count {
        let inode = volume.read_inode(number)?;
        if inode.links_count > 0 {
            println!("{inode:#}");
        }
    }
    Ok(())
}
