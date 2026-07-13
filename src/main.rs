mod ext4;
mod types;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::ext4::Volume;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    partition: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let volume = Volume::open(&args.partition)?;
    println!("{}", volume.sb);

    Ok(())
}
