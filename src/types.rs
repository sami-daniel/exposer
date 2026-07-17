use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4SuperBlock {
    pub inodes_count: u32,
    pub blocks_count_lo: u32,
    pub reserved_blocks_count_lo: u32,
    pub free_blocks_count_lo: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_cluster_size: u32,
    pub blocks_per_group: u32,
    pub clusters_per_group: u32,
    pub inodes_per_group: u32,
    pub modify_time: u32,
    pub write_time: u32,
    pub mount_count: u16,
    pub max_mount_count: u16,
    pub magic: u16,
    pub state: u16,
    pub error_policy: u16,
    pub minor_revision: u16,
    pub last_check_time: u32,
    pub check_interval: u32,
    pub creator_os: u32,
    pub revision: u32,
    pub default_reserved_uid: u16,
    pub default_reserved_gid: u16,
    pub first_inode: u32,
    pub inode_size: u16,
    pub block_group_number: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub uuid: [u8; 16],
    pub volume_name: [u8; 16],
    pub last_mounted_path: [u8; 64],
    pub algorithm_usage_bitmap: u32,
    pub prealloc_blocks: u8,
    pub prealloc_dir_blocks: u8,
    pub reserved_gdt_blocks: u16,
    pub journal_uuid: [u8; 16],
    pub journal_inode: u32,
    pub journal_device: u32,
    pub last_orphan: u32,
    pub hash_seed: [u32; 4],
    pub default_hash_version: u8,
    pub journal_backup_type: u8,
    pub descriptor_size: u16,
    pub default_mount_options: u32,
    pub first_meta_block_group: u32,
    pub mkfs_time: u32,
    pub journal_blocks: [u32; 17],
    pub blocks_count_hi: u32,
    pub reserved_blocks_count_hi: u32,
    pub free_blocks_count_hi: u32,
    pub min_extra_inode_size: u16,
    pub want_extra_inode_size: u16,
    pub flags: u32,
    pub raid_stride: u16,
    pub mmp_update_interval: u16,
    pub mmp_block: u64,
    pub raid_stripe_width: u32,
    pub log_groups_per_flex: u8,
    pub checksum_type: u8,
    pub encryption_level: u8,
    pub reserved_pad: u8,
    pub kbytes_written: u64,
    pub snapshot_inode: u32,
    pub snapshot_id: u32,
    pub snapshot_reserved_blocks_count: u64,
    pub snapshot_list: u32,
    pub error_count: u32,
    pub first_error_time: u32,
    pub first_error_inode: u32,
    pub first_error_block: u64,
    pub first_error_func: [u8; 32],
    pub first_error_line: u32,
    pub last_error_time: u32,
    pub last_error_inode: u32,
    pub last_error_line: u32,
    pub last_error_block: u64,
    pub last_error_func: [u8; 32],
    pub mount_options: [u8; 64],
    pub user_quota_inode: u32,
    pub group_quota_inode: u32,
    pub overhead_clusters: u32,
    pub backup_block_groups: [u32; 2],
    pub encrypt_algorithms: [u8; 4],
    pub encrypt_password_salt: [u8; 16],
    pub lost_found_inode: u32,
    pub project_quota_inode: u32,
    pub checksum_seed: u32,
    pub write_time_hi: u8,
    pub modify_time_hi: u8,
    pub mkfs_time_hi: u8,
    pub last_check_time_hi: u8,
    pub first_error_time_hi: u8,
    pub last_error_time_hi: u8,
    pub padding: [u8; 2],
    pub encoding: u16,
    pub encoding_flags: u16,
    pub reserved: [u32; 95],
    pub checksum: u32,
}

const _: () = assert!(core::mem::size_of::<Ext4SuperBlock>() == 1024);

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4GroupDescriptor {
    pub block_bitmap_lo: u32,
    pub inode_bitmap_lo: u32,
    pub inode_table_lo: u32,
    pub free_blocks_count_lo: u16,
    pub free_inodes_count_lo: u16,
    pub used_dirs_count_lo: u16,
    pub flags: u16,
    pub exclude_bitmap_lo: u32,
    pub block_bitmap_checksum_lo: u16,
    pub inode_bitmap_checksum_lo: u16,
    pub unused_inodes_count_lo: u16,
    pub checksum: u16,
    pub block_bitmap_hi: u32,
    pub inode_bitmap_hi: u32,
    pub inode_table_hi: u32,
    pub free_blocks_count_hi: u16,
    pub free_inodes_count_hi: u16,
    pub used_dirs_count_hi: u16,
    pub unused_inodes_count_hi: u16,
    pub exclude_bitmap_hi: u32,
    pub block_bitmap_checksum_hi: u16,
    pub inode_bitmap_checksum_hi: u16,
    pub reserved: u32,
}

const _: () = assert!(core::mem::size_of::<Ext4GroupDescriptor>() == 64);

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4Inode {
    pub mode: u16,
    pub uid_lo: u16,
    pub size_lo: u32,
    pub access_time: u32,
    pub change_time: u32,
    pub modify_time: u32,
    pub delete_time: u32,
    pub gid_lo: u16,
    pub links_count: u16,
    pub blocks_lo: u32,
    pub flags: u32,
    pub version_lo: u32,
    pub block_map: [u32; 15],
    pub generation: u32,
    pub file_acl_lo: u32,
    pub size_hi: u32,
    pub obsolete_fragment_addr: u32,
    pub blocks_hi: u16,
    pub file_acl_hi: u16,
    pub uid_hi: u16,
    pub gid_hi: u16,
    pub checksum_lo: u16,
    pub reserved: u16,
    pub extra_inode_size: u16,
    pub checksum_hi: u16,
    pub change_time_extra: u32,
    pub modify_time_extra: u32,
    pub access_time_extra: u32,
    pub create_time: u32,
    pub create_time_extra: u32,
    pub version_hi: u32,
    pub project_id: u32,
}

const _: () = assert!(core::mem::size_of::<Ext4Inode>() == 160);

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4ExtentHeader {
    pub magic: u16,
    pub entries: u16,
    pub max_entries: u16,
    pub depth: u16,
    pub generation: u32,
}

const _: () = assert!(core::mem::size_of::<Ext4ExtentHeader>() == 12);

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4Extent {
    pub logical_block: u32,
    pub length: u16,
    pub start_hi: u16,
    pub start_lo: u32,
}

const _: () = assert!(core::mem::size_of::<Ext4Extent>() == 12);

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4ExtentIdx {
    pub logical_block: u32,
    pub leaf_lo: u32,
    pub leaf_hi: u16,
    pub unused: u16,
}

const _: () = assert!(core::mem::size_of::<Ext4ExtentIdx>() == 12);

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Ext4DirEntry {
    pub inode: u32,
    pub record_length: u16,
    pub name_length: u8,
    pub file_type: u8,
    /* name is variable length, read from the raw bytes after this struct instead
     * of bytemucking it */
}

const _: () = assert!(core::mem::size_of::<Ext4DirEntry>() == 8);
