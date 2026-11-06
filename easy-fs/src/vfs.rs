use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ, NAME_LENGTH_LIMIT,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

/// File identity and link count read under the filesystem lock.
pub struct InodeStat {
    /// Index in the on-disk inode area.
    pub inode_id: u32,
    /// Whether this inode is a directory.
    pub is_dir: bool,
    /// Number of directory entries referring to this inode.
    pub nlink: u32,
}

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    fn link_count(&self, fs: &EasyFileSystem, inode_id: u32) -> u32 {
        let (block_id, block_offset) = fs.get_disk_inode_pos(0);
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .read(block_offset, |root: &DiskInode| {
                let mut count = 0;
                let mut dirent = DirEntry::empty();
                for i in 0..root.size as usize / DIRENT_SZ {
                    assert_eq!(
                        root.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device),
                        DIRENT_SZ,
                    );
                    if !dirent.name().is_empty() && dirent.inode_id() == inode_id {
                        count += 1;
                    }
                }
                count
            })
    }
    /// Read inode identity, type and the current number of directory links.
    pub fn stat(&self) -> InodeStat {
        let fs = self.fs.lock();
        let inode_id = fs.get_disk_inode_id(self.block_id as u32, self.block_offset);
        let is_dir = self.read_disk_inode(|inode| inode.is_dir());
        InodeStat {
            inode_id,
            is_dir,
            nlink: self.link_count(&fs, inode_id),
        }
    }
    /// Register one open file description for this inode.
    pub fn open(&self) {
        let mut fs = self.fs.lock();
        let inode_id = fs.get_disk_inode_id(self.block_id as u32, self.block_offset);
        *fs.open_inodes.entry(inode_id).or_insert(0) += 1;
    }
    /// Release one open file description and reclaim an orphan after the last close.
    pub fn close(&self) {
        let mut fs = self.fs.lock();
        let inode_id = fs.get_disk_inode_id(self.block_id as u32, self.block_offset);
        let count = fs.open_inodes.get_mut(&inode_id).unwrap();
        *count -= 1;
        if *count == 0 {
            fs.open_inodes.remove(&inode_id);
            if self.link_count(&fs, inode_id) == 0 {
                fs.dealloc_inode(inode_id);
                block_cache_sync_all();
            }
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        if name.is_empty() {
            return None;
        }
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    fn insert_dirent(
        &self,
        name: &str,
        inode_id: u32,
        root: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        let mut offset = root.size as usize;
        let mut dirent = DirEntry::empty();
        for i in 0..root.size as usize / DIRENT_SZ {
            root.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device);
            if dirent.name().is_empty() {
                offset = i * DIRENT_SZ;
                break;
            }
        }
        if offset == root.size as usize {
            self.increase_size((offset + DIRENT_SZ) as u32, root, fs);
        }
        let dirent = DirEntry::new(name, inode_id);
        root.write_at(offset, dirent.as_bytes(), &self.block_device);
    }
    /// Add another directory name for an existing inode without copying its data.
    pub fn link(&self, old_name: &str, new_name: &str) -> bool {
        if new_name.is_empty() || new_name.len() > NAME_LENGTH_LIMIT {
            return false;
        }
        let mut fs = self.fs.lock();
        let inode_id = self.read_disk_inode(|root| {
            if self.find_inode_id(new_name, root).is_some() {
                return None;
            }
            self.find_inode_id(old_name, root)
        });
        let inode_id = match inode_id {
            Some(inode_id) => inode_id,
            None => return false,
        };
        self.modify_disk_inode(|root| {
            self.insert_dirent(new_name, inode_id, root, &mut fs);
        });
        block_cache_sync_all();
        true
    }
    /// Remove a directory name, retaining data until its last open description closes.
    pub fn unlink(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut fs = self.fs.lock();
        let inode_id = self.modify_disk_inode(|root| {
            let mut dirent = DirEntry::empty();
            for i in 0..root.size as usize / DIRENT_SZ {
                root.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device);
                if dirent.name() == name {
                    root.write_at(
                        i * DIRENT_SZ,
                        DirEntry::empty().as_bytes(),
                        &self.block_device,
                    );
                    return Some(dirent.inode_id());
                }
            }
            None
        });
        let inode_id = match inode_id {
            Some(inode_id) => inode_id,
            None => return false,
        };
        if self.link_count(&fs, inode_id) == 0 && !fs.open_inodes.contains_key(&inode_id) {
            fs.dealloc_inode(inode_id);
        }
        block_cache_sync_all();
        true
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        if name.is_empty() || name.len() > NAME_LENGTH_LIMIT {
            return None;
        }
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            self.insert_dirent(name, new_inode_id, root_inode, &mut fs);
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if !dirent.name().is_empty() {
                    v.push(String::from(dirent.name()));
                }
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
