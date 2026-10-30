//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};

use crate::task::get_syscall_count;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    match trace_request {
        0 => {
            // TODO: 将 id 转换成 *const u8
            // TODO: 使用 unsafe 读取一个字节
            // TODO: 返回该字节的值
            let ptr = id as *const u8;
            unsafe { *ptr as isize }
        }
        1 => {
            // TODO: 将 id 转换成 *mut u8
            // TODO: 将 data 转换成 u8
            // TODO: 使用 unsafe 写入
            // TODO: 返回 0
            let ptr = id as *mut u8;
            let value = data as u8;
            unsafe {
                *ptr = value;
            }
            0
        }
        2 => {
            let count = get_syscall_count(id);
            count as isize
        }
        _ => -1,
    }
}
