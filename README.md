# Exposer

A read-only ext4 dissector. Point it at an ext4 volume and a path, and it walks
the on-disk structures by hand to tell you everything about that file: its inode
metadata and the physical blocks its data lives in.

It never mounts the filesystem and never writes to the device. It parses the raw
bytes itself instead of asking the OS, so it trusts nothing the mounted layer
would tell you.

## How it works

An ext4 filesystem is a chain of pointers on disk. Exposer follows that chain one
link at a time, and each link is the same move: read some bytes at a computed
offset, cast them into a typed struct, follow a field to the next structure.

```
superblock -> block group descriptors -> inode table -> inode -> extent tree -> data blocks
                                                           |
                                     root inode (#2) ------+--> directory entries -> name to inode
```

The superblock lives at byte 1024 and holds the facts everything else is computed
from: block size (`1024 << s_log_block_size`), where inodes live, inode size, and
which features are enabled.

## Usage

```
exposer --partition /dev/sdaN
```

Today this reads and prints fields from the superblock. Path resolution is not
built yet (see the TODO).

To test without touching a real disk, make a file-backed image:

```
dd if=/dev/zero of=test.img bs=1M count=64
mkfs.ext4 test.img
exposer --partition test.img
```
