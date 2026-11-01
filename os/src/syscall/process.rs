//! Process management syscalls
use crate::config::{MAX_SYSCALL_NUM, PAGE_SIZE};
use crate::mm::translated_byte_buffer;
use crate::mm::{MapPermission, PTEFlags, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, current_task_mmap, current_task_munmap, current_user_token,
    exit_current_and_run_next, get_syscall_times, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    // 1. 获取用户页表
    let token = current_user_token();
    let page_table = PageTable::from_token(token);

    // 2. 计算用户地址范围
    let start = ts as usize;
    let len = core::mem::size_of::<TimeVal>();

    let end = match start.checked_add(len) {
        Some(end) => end,
        None => return -1,
    };

    let start_vpn = start / PAGE_SIZE;
    let end_vpn = (end - 1) / PAGE_SIZE;

    // 3. 先检查所有涉及页面的写权限
    for vpn in start_vpn..=end_vpn {
        let va = VirtAddr::from(vpn * PAGE_SIZE);
        let pte = page_table.translate(va.floor());

        match pte {
            Some(pte) => {
                if !pte.is_valid() || !pte.flags().contains(PTEFlags::U) || !pte.writable() {
                    return -1;
                }
            }
            None => return -1,
        }
    }

    // 4. 获取时间
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    // 5. 将 TimeVal 转换为字节
    let src_bytes =
        unsafe { core::slice::from_raw_parts(&time_val as *const TimeVal as *const u8, len) };

    // 6. 将字节复制到用户物理页
    let buffers = translated_byte_buffer(token, ts as *const u8, len);

    let mut offset = 0;

    for buffer in buffers {
        let size = buffer.len();
        buffer.copy_from_slice(&src_bytes[offset..offset + size]);
        offset += size;
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        // 0
        0 => {
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                if pte.is_valid() && pte.readable() && pte.flags().contains(PTEFlags::U) {
                    return pte.ppn().get_bytes_array()[va.page_offset()] as isize;
                }
            }
            -1
        }
        // 1
        1 => {
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                if pte.is_valid() && pte.writable() && pte.flags().contains(PTEFlags::U) {
                    pte.ppn().get_bytes_array()[va.page_offset()] = data as u8;
                    return 0;
                }
            }
            -1
        }
        // 2
        2 => {
            if id < MAX_SYSCALL_NUM {
                get_syscall_times(id) as isize
            } else {
                -1
            }
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    // 检查 start 是否按 PAGE_SIZE 对齐
    if !VirtAddr::from(start).aligned() {
        return -1;
    }
    // 检查 prot 是否包含非法位
    if (port & !0x7) != 0 || (port & 0x7) == 0 {
        return -1;
    }
    // 检查 prot 是否为 0
    if len == 0 {
        return 0;
    }
    // 使用 checked_add 检查 start + len 是否溢出
    if start.checked_add(len).is_none() || start + len >= (1 << 38) {
        return -1;
    }

    // 将 port 左移 1 位对齐 PTE 的 R/W/X 位，并附加上 U 标志位
    let perm = MapPermission::from_bits(((port as u8) << 1) | MapPermission::U.bits()).unwrap();
    current_task_mmap(start, len, perm)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if !VirtAddr::from(start).aligned() {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if start.checked_add(len).is_none() || start + len >= (1 << 38) {
        return -1;
    }

    current_task_munmap(start, len)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
