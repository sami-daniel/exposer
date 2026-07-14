use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::{Result, bail};

use crate::types::{ext4_group_desc, ext4_inode, ext4_super_block};

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

#[derive(Debug, Clone)]
pub struct Inode {
    pub number: u32,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub links_count: u16,
    pub blocks: u64,
    pub flags: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub crtime: u32,
}

impl Inode {
    fn from_raw(number: u32, raw: ext4_inode) -> Self {
        let uid = u16::from_le(raw.i_uid) as u32 | ((u16::from_le(raw.l_i_uid_high) as u32) << 16);
        let gid = u16::from_le(raw.i_gid) as u32 | ((u16::from_le(raw.l_i_gid_high) as u32) << 16);
        let size =
            u32::from_le(raw.i_size_lo) as u64 | ((u32::from_le(raw.i_size_high) as u64) << 32);
        let blocks = u32::from_le(raw.i_blocks_lo) as u64
            | ((u16::from_le(raw.l_i_blocks_high) as u64) << 32);

        Self {
            number,
            mode: u16::from_le(raw.i_mode),
            uid,
            gid,
            size,
            links_count: u16::from_le(raw.i_links_count),
            blocks,
            flags: u32::from_le(raw.i_flags),
            atime: u32::from_le(raw.i_atime),
            ctime: u32::from_le(raw.i_ctime),
            mtime: u32::from_le(raw.i_mtime),
            crtime: u32::from_le(raw.i_crtime),
        }
    }

    pub fn file_type(&self) -> &'static str {
        match self.mode & 0xF000 {
            0x1000 => "fifo",
            0x2000 => "character device",
            0x4000 => "directory",
            0x6000 => "block device",
            0x8000 => "regular file",
            0xA000 => "symlink",
            0xC000 => "socket",
            _ => "unknown",
        }
    }

    pub fn permissions(&self) -> u16 {
        self.mode & 0o7777
    }

    pub fn mode_string(&self) -> String {
        let type_char = match self.mode & 0xF000 {
            0x1000 => 'p',
            0x2000 => 'c',
            0x4000 => 'd',
            0x6000 => 'b',
            0x8000 => '-',
            0xA000 => 'l',
            0xC000 => 's',
            _ => '?',
        };

        let mut s = String::with_capacity(10);
        s.push(type_char);
        for (bit, ch) in [
            (0o400, 'r'),
            (0o200, 'w'),
            (0o100, 'x'),
            (0o040, 'r'),
            (0o020, 'w'),
            (0o010, 'x'),
            (0o004, 'r'),
            (0o002, 'w'),
            (0o001, 'x'),
        ] {
            s.push(if self.mode & bit != 0 { ch } else { '-' });
        }
        s
    }
}

impl fmt::Display for Inode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return write!(
                f,
                "{:>8}  {}  uid {:<5} gid {:<5} {:>10} bytes  {} links",
                self.number,
                self.mode_string(),
                self.uid,
                self.gid,
                self.size,
                self.links_count,
            );
        }

        writeln!(f, "inode {}", self.number)?;
        writeln!(f, "  type:              {}", self.file_type())?;
        writeln!(f, "  mode:              {:04o}", self.permissions())?;
        writeln!(f, "  owner:             uid {}, gid {}", self.uid, self.gid)?;
        writeln!(f, "  size:              {} bytes", self.size)?;
        writeln!(f, "  links:             {}", self.links_count)?;
        writeln!(f, "  blocks (512B):     {}", self.blocks)?;
        writeln!(f, "  flags:             {:#x}", self.flags)?;
        write!(
            f,
            "  times (epoch):     a={} c={} m={} cr={}",
            self.atime, self.ctime, self.mtime, self.crtime
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

    pub fn read_inode(&mut self, number: u32) -> Result<Inode> {
        if number == 0 {
            bail!("Select a valid inode");
        }

        let per_group = self.sb.inodes_per_group;
        let group = (number - 1) / per_group;
        let index = (number - 1) % per_group;

        let desc = self.read_group(group)?;
        let inode_size = self.sb.inode_size as u64;
        let offset = desc.inode_table * self.sb.block_size + index as u64 * inode_size;

        let bytes = self.read_at(offset, inode_size as usize)?;
        let mut buf = [0u8; 160];
        let n = bytes.len().min(buf.len());
        buf[..n].copy_from_slice(&bytes[..n]);
        let raw: ext4_inode = *bytemuck::from_bytes(&buf);

        Ok(Inode::from_raw(number, raw))
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
