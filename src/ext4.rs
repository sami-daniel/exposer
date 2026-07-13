use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::{Result, bail};

use crate::types::{ext4_group_desc, ext4_super_block};

/// The superblock always starts 1024 bytes into the volume, whatever the block
/// size is. The bytes before it belong to boot records.
const SUPERBLOCK_OFFSET: u64 = 1024;

const EXT4_MAGIC: u16 = 0xEF53;

const INCOMPAT_FILETYPE: u32 = 0x0002;
const INCOMPAT_EXTENTS: u32 = 0x0040;
const INCOMPAT_64BIT: u32 = 0x0080;

#[derive(Debug, Clone)]
pub struct Superblock {
    pub raw: ext4_super_block,

    pub block_size: u64,
    pub blocks_count: u64,
    pub free_blocks_count: u64,
    pub inodes_count: u32,
    pub free_inodes_count: u32,

    pub blocks_per_group: u32,
    pub inodes_per_group: u32,
    pub first_data_block: u32,

    pub inode_size: u16,
    pub desc_size: u16,

    pub has_64bit: bool,
    pub has_extents: bool,
    pub has_filetype: bool,
}

impl Superblock {
    fn from_raw(raw: ext4_super_block) -> Result<Self> {
        let magic = u16::from_le(raw.s_magic);
        if magic != EXT4_MAGIC {
            bail!(
                "not an ext4 volume: magic is {:#06x}, expected {:#06x}",
                magic,
                EXT4_MAGIC
            );
        }

        let incompat = u32::from_le(raw.s_feature_incompat);
        let has_64bit = incompat & INCOMPAT_64BIT != 0;
        let has_extents = incompat & INCOMPAT_EXTENTS != 0;
        let has_filetype = incompat & INCOMPAT_FILETYPE != 0;

        // For some reason this is stored as a shift: https://docs.kernel.org/filesystems/ext4/super.html
        let block_size = 1024u64 << u32::from_le(raw.s_log_block_size);

        let blocks_count = fold64(raw.s_blocks_count_lo, raw.s_blocks_count_hi, has_64bit);
        let free_blocks_count = fold64(
            raw.s_free_blocks_count_lo,
            raw.s_free_blocks_count_hi,
            has_64bit,
        );

        let desc_size = if has_64bit {
            u16::from_le(raw.s_desc_size).max(32)
        } else {
            32
        };

        let inode_size = if u32::from_le(raw.s_rev_level) == 0 {
            128
        } else {
            u16::from_le(raw.s_inode_size)
        };

        Ok(Self {
            raw,
            block_size,
            blocks_count,
            free_blocks_count,
            inodes_count: u32::from_le(raw.s_inodes_count),
            free_inodes_count: u32::from_le(raw.s_free_inodes_count),
            blocks_per_group: u32::from_le(raw.s_blocks_per_group),
            inodes_per_group: u32::from_le(raw.s_inodes_per_group),
            first_data_block: u32::from_le(raw.s_first_data_block),
            inode_size,
            desc_size,
            has_64bit,
            has_extents,
            has_filetype,
        })
    }

    pub fn volume_name(&self) -> String {
        c_str(&self.raw.s_volume_name)
    }

    pub fn uuid(&self) -> String {
        let u = &self.raw.s_uuid;
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
             {:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            u[0],
            u[1],
            u[2],
            u[3],
            u[4],
            u[5],
            u[6],
            u[7],
            u[8],
            u[9],
            u[10],
            u[11],
            u[12],
            u[13],
            u[14],
            u[15],
        )
    }
}

impl fmt::Display for Superblock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.volume_name();
        let name = if name.is_empty() { "(none)" } else { &name };

        writeln!(f, "ext4 volume")?;
        writeln!(f, "  volume name:       {name}")?;
        writeln!(f, "  uuid:              {}", self.uuid())?;
        writeln!(f, "  block size:        {} bytes", self.block_size)?;
        writeln!(f, "  inode size:        {} bytes", self.inode_size)?;
        writeln!(f, "  descriptor size:   {} bytes", self.desc_size)?;
        writeln!(
            f,
            "  blocks:            {} total, {} free",
            self.blocks_count, self.free_blocks_count
        )?;
        writeln!(
            f,
            "  inodes:            {} total, {} free",
            self.inodes_count, self.free_inodes_count
        )?;
        writeln!(f, "  blocks per group:  {}", self.blocks_per_group)?;
        writeln!(f, "  inodes per group:  {}", self.inodes_per_group)?;
        writeln!(f, "  first data block:  {}", self.first_data_block)?;
        write!(
            f,
            "  features:          64bit={} extents={} filetype={}",
            self.has_64bit, self.has_extents, self.has_filetype
        )
    }
}

#[derive(Debug, Clone)]
pub struct GroupDescriptor {
    pub group: u32,
    pub block_bitmap: u64,
    pub inode_bitmap: u64,
    pub inode_table: u64,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub used_dirs_count: u32,
}

impl GroupDescriptor {
    fn from_raw(group: u32, raw: ext4_group_desc, has_64bit: bool) -> Self {
        Self {
            group,
            block_bitmap: fold64(raw.bg_block_bitmap_lo, raw.bg_block_bitmap_hi, has_64bit),
            inode_bitmap: fold64(raw.bg_inode_bitmap_lo, raw.bg_inode_bitmap_hi, has_64bit),
            inode_table: fold64(raw.bg_inode_table_lo, raw.bg_inode_table_hi, has_64bit),
            free_blocks_count: fold32(
                raw.bg_free_blocks_count_lo,
                raw.bg_free_blocks_count_hi,
                has_64bit,
            ),
            free_inodes_count: fold32(
                raw.bg_free_inodes_count_lo,
                raw.bg_free_inodes_count_hi,
                has_64bit,
            ),
            used_dirs_count: fold32(
                raw.bg_used_dirs_count_lo,
                raw.bg_used_dirs_count_hi,
                has_64bit,
            ),
        }
    }
}

impl fmt::Display for GroupDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "group {:>3}: inode table @ block {}, block bitmap @ {}, inode bitmap @ {}, \
             {} free blocks, {} free inodes, {} dirs",
            self.group,
            self.inode_table,
            self.block_bitmap,
            self.inode_bitmap,
            self.free_blocks_count,
            self.free_inodes_count,
            self.used_dirs_count,
        )
    }
}

pub struct Volume {
    file: File,
    pub sb: Superblock,
}

impl Volume {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;

        let mut buf = [0u8; 1024];
        file.seek(SeekFrom::Start(SUPERBLOCK_OFFSET))?;
        file.read_exact(&mut buf)?;
        let raw: ext4_super_block = *bytemuck::from_bytes(&buf);

        let sb = Superblock::from_raw(raw)?;
        Ok(Self { file, sb })
    }

    pub fn read_block(&mut self, block: u64) -> Result<Vec<u8>> {
        self.read_at(block * self.sb.block_size, self.sb.block_size as usize)
    }

    pub fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn group_count(&self) -> u64 {
        self.sb
            .blocks_count
            .div_ceil(self.sb.blocks_per_group as u64)
    }

    pub fn read_group(&mut self, group: u32) -> Result<GroupDescriptor> {
        let desc_size = self.sb.desc_size as usize;
        let table_start = (self.sb.first_data_block as u64 + 1) * self.sb.block_size;
        let offset = table_start + group as u64 * desc_size as u64;

        let mut buf = [0u8; 64];
        let bytes = self.read_at(offset, desc_size)?;
        buf[..desc_size].copy_from_slice(&bytes);
        let raw: ext4_group_desc = *bytemuck::from_bytes(&buf);

        Ok(GroupDescriptor::from_raw(group, raw, self.sb.has_64bit))
    }

    pub fn read_groups(&mut self) -> Result<Vec<GroupDescriptor>> {
        (0..self.group_count() as u32)
            .map(|g| self.read_group(g))
            .collect()
    }
}

fn fold64(lo: u32, hi: u32, has_64bit: bool) -> u64 {
    let lo = u32::from_le(lo) as u64;
    if has_64bit {
        lo | ((u32::from_le(hi) as u64) << 32)
    } else {
        lo
    }
}

fn fold32(lo: u16, hi: u16, has_64bit: bool) -> u32 {
    let lo = u16::from_le(lo) as u32;
    if has_64bit {
        lo | ((u16::from_le(hi) as u32) << 16)
    } else {
        lo
    }
}

fn c_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
