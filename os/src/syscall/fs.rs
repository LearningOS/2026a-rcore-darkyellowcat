//! File and filesystem-related syscalls
use crate::config::PAGE_SIZE;
use crate::fs::{link_file, open_file, unlink_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer, VirtAddr};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// Read file metadata and copy it to a writable user buffer.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let file = match inner.fd_table.get(fd).and_then(|entry| entry.as_ref()) {
        Some(file) => file.clone(),
        None => return -1,
    };
    let start = st as usize;
    let end = match start.checked_add(core::mem::size_of::<Stat>()) {
        Some(end) if start != 0 && end <= (1usize << 38) => end,
        _ => return -1,
    };
    for page in start / PAGE_SIZE..(end - 1) / PAGE_SIZE + 1 {
        let pte = inner
            .memory_set
            .translate(VirtAddr::from(page * PAGE_SIZE).floor());
        if !pte.map_or(false, |pte| {
            pte.is_valid() && pte.is_user() && pte.readable() && pte.writable()
        }) {
            return -1;
        }
    }
    drop(inner);
    let stat = match file.stat() {
        Some(stat) => stat,
        None => return -1,
    };

    let mut bytes = [0u8; core::mem::size_of::<Stat>()];
    bytes[..8].copy_from_slice(&stat.dev.to_le_bytes());
    bytes[8..16].copy_from_slice(&stat.ino.to_le_bytes());
    bytes[16..20].copy_from_slice(&stat.mode.bits().to_le_bytes());
    bytes[20..24].copy_from_slice(&stat.nlink.to_le_bytes());
    let mut offset = 0;
    for buffer in translated_byte_buffer(token, st as *const u8, bytes.len()) {
        let end = offset + buffer.len();
        buffer.copy_from_slice(&bytes[offset..end]);
        offset = end;
    }
    0
}

/// Add a directory name referring to the same inode as the old name.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    let token = current_user_token();
    let old_name = translated_str(token, old_name);
    let new_name = translated_str(token, new_name);
    if link_file(old_name.as_str(), new_name.as_str()) {
        0
    } else {
        -1
    }
}

/// Remove one directory name and reclaim the file when no links or opens remain.
pub fn sys_unlinkat(name: *const u8) -> isize {
    let name = translated_str(current_user_token(), name);
    if unlink_file(name.as_str()) {
        0
    } else {
        -1
    }
}
