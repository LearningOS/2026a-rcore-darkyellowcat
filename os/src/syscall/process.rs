//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE,
    fs::{open_file, OpenFlags},
    mm::{
        translated_byte_buffer, translated_refmut, translated_str, MapPermission, VirtAddr,
        VirtPageNum,
    },
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskControlBlock,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// Get time in seconds and microseconds, including across user page boundaries.
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let time_us = get_time_us();
    let time = TimeVal {
        sec: time_us / 1_000_000,
        usec: time_us % 1_000_000,
    };

    let mut bytes = [0u8; core::mem::size_of::<TimeVal>()];
    let word_size = core::mem::size_of::<usize>();
    bytes[..word_size].copy_from_slice(&time.sec.to_le_bytes());
    bytes[word_size..].copy_from_slice(&time.usec.to_le_bytes());

    let mut offset = 0;
    for buffer in translated_byte_buffer(current_user_token(), ts as *const u8, bytes.len()) {
        let end = offset + buffer.len();
        buffer.copy_from_slice(&bytes[offset..end]);
        offset = end;
    }
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if start % PAGE_SIZE != 0 || port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }

    if len == 0 {
        return 0;
    }

    let end = match start.checked_add(len) {
        Some(end) => end,
        None => return -1,
    };

    if end > (1usize << 38) {
        return -1;
    }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;

    let first_page = start / PAGE_SIZE;
    let last_page = (end + PAGE_SIZE - 1) / PAGE_SIZE;

    for page in first_page..last_page {
        let addr = VirtAddr::from(page * PAGE_SIZE);
        if memory_set
            .translate(addr.floor())
            .map_or(false, |pte| pte.is_valid())
        {
            return -1;
        }
    }

    let perm = MapPermission::from_bits((port << 1) as u8).unwrap() | MapPermission::U;

    for page in first_page..last_page {
        let page_start = page * PAGE_SIZE;
        if !memory_set.try_insert_framed_page(VirtAddr::from(page_start), perm) {
            if page > first_page {
                memory_set.unmap_user_range(VirtPageNum::from(first_page), VirtPageNum::from(page));
            }
            return -1;
        }
    }

    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % PAGE_SIZE != 0 || _len == 0 {
        return -1;
    }

    let end = match _start.checked_add(_len) {
        Some(end) => end,
        None => return -1,
    };

    if end > (1usize << 38) {
        return -1;
    }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    let start_vpn = VirtPageNum::from(_start / PAGE_SIZE);
    let end_vpn = VirtPageNum::from((end - 1) / PAGE_SIZE + 1);

    if inner.memory_set.unmap_user_range(start_vpn, end_vpn) {
        0
    } else {
        -1
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// Create a child process directly from an executable in the filesystem.
pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let app_inode = match open_file(path.as_str(), OpenFlags::RDONLY) {
        Some(inode) => inode,
        None => return -1,
    };
    let elf_data = app_inode.read_all();
    let parent = current_task().unwrap();
    let child = Arc::new(TaskControlBlock::new(elf_data.as_slice()));
    let child_pid = child.getpid();

    {
        let mut child_inner = child.inner_exclusive_access();
        child_inner.parent = Some(Arc::downgrade(&parent));
    }
    {
        let mut parent_inner = parent.inner_exclusive_access();
        parent_inner.children.push(child.clone());
    }

    add_task(child);
    child_pid as isize
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
