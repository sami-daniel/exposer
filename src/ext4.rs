use std::{
    fmt,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use anyhow::{Result, bail};

use crate::types::{
    Ext4DirEntry, Ext4Extent, Ext4ExtentHeader, Ext4GroupDescriptor, Ext4Inode, Ext4SuperBlock,
};

const SUPERBLOCK_OFFSET: u64 = 1024;
const EXT4_MAGIC: u16 = 0xEF53;
const INCOMPAT_FILETYPE: u32 = 0x0002;
const INCOMPAT_EXTENTS: u32 = 0x0040;
const INCOMPAT_64BIT: u32 = 0x0080;
const EXT4_EXT_MAGIC: u16 = 0xF30A;
const EXT_INIT_MAX_LEN: u16 = 1 << 15;
const EXT4_LEGACY_INODE_SIZE: u16 = 128;
const EXT4_MIN_BLOCK_SIZE: u64 = 1024;
const EXT4_MIN_DESC_SIZE: u16 = 32;
const EXT4_MAX_DESC_SIZE: usize = 64;

const MODE_PERM_MASK: u16 = 0o7777;

#[derive(Debug, Clone)]
pub struct Superblock {
    pub raw: Ext4SuperBlock,

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
    pub block_map: [u32; 15],
}

#[derive(Debug, Clone, Copy)]
pub struct Extent {
    pub len: u16,
    pub physical: u64,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub inode: u32,
    pub file_type: u8,
    pub name: String,
}

pub struct Volume {
    file: File,
    pub sb: Superblock,
}

impl Superblock {
    fn from_raw(raw: Ext4SuperBlock) -> Result<Self> {
        let magic = raw.magic;
        if magic != EXT4_MAGIC {
            bail!(
                "not an ext4 volume: magic is {:#06x}, expected {:#06x}",
                magic,
                EXT4_MAGIC
            );
        }

        let incompat = raw.feature_incompat;
        let has_64bit = incompat & INCOMPAT_64BIT != 0;
        let has_extents = incompat & INCOMPAT_EXTENTS != 0;
        let has_filetype = incompat & INCOMPAT_FILETYPE != 0;

        // For some reason this is stored as a shift: https://docs.kernel.org/filesystems/ext4/super.html
        let block_size = EXT4_MIN_BLOCK_SIZE << raw.log_block_size;

        let blocks_count = fold64(raw.blocks_count_lo, raw.blocks_count_hi, has_64bit);
        let free_blocks_count = fold64(
            raw.free_blocks_count_lo,
            raw.free_blocks_count_hi,
            has_64bit,
        );

        let desc_size = if has_64bit {
            raw.descriptor_size.max(EXT4_MIN_DESC_SIZE)
        } else {
            EXT4_MIN_DESC_SIZE
        };

        let inode_size = if raw.revision == 0 {
            EXT4_LEGACY_INODE_SIZE
        } else {
            raw.inode_size
        };

        Ok(Self {
            raw,
            block_size,
            blocks_count,
            free_blocks_count,
            inodes_count: raw.inodes_count,
            free_inodes_count: raw.free_inodes_count,
            blocks_per_group: raw.blocks_per_group,
            inodes_per_group: raw.inodes_per_group,
            first_data_block: raw.first_data_block,
            inode_size,
            desc_size,
            has_64bit,
            has_extents,
            has_filetype,
        })
    }

    pub fn volume_name(&self) -> String {
        c_str(&self.raw.volume_name)
    }

    pub fn uuid(&self) -> String {
        let u = &self.raw.uuid;
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

impl GroupDescriptor {
    fn from_raw(group: u32, raw: Ext4GroupDescriptor, has_64bit: bool) -> Self {
        Self {
            group,
            block_bitmap: fold64(raw.block_bitmap_lo, raw.block_bitmap_hi, has_64bit),
            inode_bitmap: fold64(raw.inode_bitmap_lo, raw.inode_bitmap_hi, has_64bit),
            inode_table: fold64(raw.inode_table_lo, raw.inode_table_hi, has_64bit),
            free_blocks_count: fold32(
                raw.free_blocks_count_lo,
                raw.free_blocks_count_hi,
                has_64bit,
            ),
            free_inodes_count: fold32(
                raw.free_inodes_count_lo,
                raw.free_inodes_count_hi,
                has_64bit,
            ),
            used_dirs_count: fold32(raw.used_dirs_count_lo, raw.used_dirs_count_hi, has_64bit),
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

impl Inode {
    fn from_raw(number: u32, raw: Ext4Inode) -> Self {
        let uid = raw.uid_lo as u32 | ((raw.uid_hi as u32) << 16);
        let gid = raw.gid_lo as u32 | ((raw.gid_hi as u32) << 16);
        let size = raw.size_lo as u64 | ((raw.size_hi as u64) << 32);
        let blocks = raw.blocks_lo as u64 | ((raw.blocks_hi as u64) << 32);

        Self {
            number,
            mode: raw.mode,
            uid,
            gid,
            size,
            links_count: raw.links_count,
            blocks,
            flags: raw.flags,
            atime: raw.access_time,
            ctime: raw.change_time,
            mtime: raw.modify_time,
            crtime: raw.create_time,
            block_map: raw.block_map,
        }
    }

    pub fn is_dir(&self) -> bool {
        self.mode as u32 & libc::S_IFMT == libc::S_IFDIR
    }

    pub fn file_type(&self) -> &'static str {
        match self.mode as u32 & libc::S_IFMT {
            libc::S_IFIFO => "fifo",
            libc::S_IFCHR => "character device",
            libc::S_IFDIR => "directory",
            libc::S_IFBLK => "block device",
            libc::S_IFREG => "regular file",
            libc::S_IFLNK => "symlink",
            libc::S_IFSOCK => "socket",
            _ => "unknown",
        }
    }

    pub fn permissions(&self) -> u16 {
        self.mode & MODE_PERM_MASK
    }

    pub fn mode_string(&self) -> String {
        let type_char = match self.mode as u32 & libc::S_IFMT {
            libc::S_IFIFO => 'p',
            libc::S_IFCHR => 'c',
            libc::S_IFDIR => 'd',
            libc::S_IFBLK => 'b',
            libc::S_IFREG => '-',
            libc::S_IFLNK => 'l',
            libc::S_IFSOCK => 's',
            _ => '?',
        };

        let mut s = String::with_capacity(10);
        s.push(type_char);
        for (bit, ch) in [
            (libc::S_IRUSR, 'r'),
            (libc::S_IWUSR, 'w'),
            (libc::S_IXUSR, 'x'),
            (libc::S_IRGRP, 'r'),
            (libc::S_IWGRP, 'w'),
            (libc::S_IXGRP, 'x'),
            (libc::S_IROTH, 'r'),
            (libc::S_IWOTH, 'w'),
            (libc::S_IXOTH, 'x'),
        ] {
            s.push(if self.mode as u32 & bit != 0 { ch } else { '-' });
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

impl DirEntry {
    pub fn file_type_str(&self) -> &'static str {
        match self.file_type {
            1 => "regular file",
            2 => "directory",
            3 => "character device",
            4 => "block device",
            5 => "fifo",
            6 => "socket",
            7 => "symlink",
            _ => "unknown",
        }
    }
}

impl fmt::Display for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:>8}  {:<16}  {}",
            self.inode,
            self.file_type_str(),
            self.name
        )
    }
}

impl Volume {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;

        let mut buf = [0u8; core::mem::size_of::<Ext4SuperBlock>()];
        file.seek(SeekFrom::Start(SUPERBLOCK_OFFSET))?;
        file.read_exact(&mut buf)?;
        let raw: Ext4SuperBlock = *bytemuck::from_bytes(&buf);

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

        let mut buf = [0u8; EXT4_MAX_DESC_SIZE];
        let bytes = self.read_at(offset, desc_size)?;
        buf[..desc_size].copy_from_slice(&bytes);
        let raw: Ext4GroupDescriptor = *bytemuck::from_bytes(&buf);

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

        // TODO: This could be cached
        let desc = self.read_group(group)?;
        let inode_size = self.sb.inode_size as u64;
        let offset = desc.inode_table * self.sb.block_size + index as u64 * inode_size;

        let bytes = self.read_at(offset, inode_size as usize)?;
        let mut buf = [0u8; core::mem::size_of::<Ext4Inode>()];
        let n = bytes.len().min(buf.len());
        buf[..n].copy_from_slice(&bytes[..n]);
        let raw: Ext4Inode = *bytemuck::from_bytes(&buf);

        Ok(Inode::from_raw(number, raw))
    }

    pub fn read_dir(&mut self, inode: &Inode) -> Result<Vec<DirEntry>> {
        if !inode.is_dir() {
            bail!("inode {} is not a directory", inode.number);
        }

        let mut entries = Vec::new();
        for extent in inode_extents(inode)? {
            for i in 0..extent.len as u64 {
                let block = self.read_block(extent.physical + i)?;
                parse_dir_block(&block, &mut entries);
            }
        }
        Ok(entries)
    }
}

fn inode_extents(inode: &Inode) -> Result<Vec<Extent>> {
    let header_size = core::mem::size_of::<Ext4ExtentHeader>();
    let extent_size = core::mem::size_of::<Ext4Extent>();
    let bytes: &[u8] = bytemuck::bytes_of(&inode.block_map);

    let header: Ext4ExtentHeader = *bytemuck::from_bytes(&bytes[..header_size]);
    if header.magic != EXT4_EXT_MAGIC {
        bail!("inode {} is not extent mapped. Not supported", inode.number);
    }

    let depth = header.depth;
    // TODO: Support non depth 0 extent trees
    if depth != 0 {
        bail!("inode {} has a depth {depth} extent tree", inode.number);
    }

    let count = header.entries as usize;
    let mut extents = Vec::with_capacity(count);
    for i in 0..count {
        let off = header_size + i * extent_size;
        let e: Ext4Extent = *bytemuck::from_bytes(&bytes[off..off + extent_size]);

        // length over EXT_INIT_MAX_LEN marks an unwritten (preallocated) extent,
        // whose real length is length - EXT_INIT_MAX_LEN. A value of exactly
        // EXT_INIT_MAX_LEN is a written extent of that length, not unwritten, so
        // this is a compare, not a mask (see ext4_ext_get_actual_len)
        let raw_len = e.length;
        let len = if raw_len <= EXT_INIT_MAX_LEN {
            raw_len
        } else {
            raw_len - EXT_INIT_MAX_LEN
        };
        let physical = e.start_lo as u64 | ((e.start_hi as u64) << 32);

        extents.push(Extent { len, physical });
    }
    Ok(extents)
}

fn parse_dir_block(block: &[u8], out: &mut Vec<DirEntry>) {
    let head_size = core::mem::size_of::<Ext4DirEntry>();
    let mut pos = 0;
    while pos + head_size <= block.len() {
        let head: Ext4DirEntry = *bytemuck::from_bytes(&block[pos..pos + head_size]);
        let record_length = head.record_length as usize;
        let name_length = head.name_length as usize;

        if record_length < head_size {
            break;
        }

        if head.inode != 0 && pos + head_size + name_length <= block.len() {
            let name = &block[pos + head_size..pos + head_size + name_length];
            out.push(DirEntry {
                inode: head.inode,
                file_type: head.file_type,
                name: String::from_utf8_lossy(name).into_owned(),
            });
        }

        pos += record_length;
    }
}

fn fold64(lo: u32, hi: u32, has_64bit: bool) -> u64 {
    let lo = lo as u64;
    if has_64bit {
        lo | ((hi as u64) << 32)
    } else {
        lo
    }
}

fn fold32(lo: u16, hi: u16, has_64bit: bool) -> u32 {
    let lo = lo as u32;
    if has_64bit {
        lo | ((hi as u32) << 16)
    } else {
        lo
    }
}

fn c_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
