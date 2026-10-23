//! SBI call wrappers

#![allow(unused)]

use core::arch::asm;

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_SYSTEM_RESET: usize = 0x53525354;

/// general sbi call
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") 0,
            in("x17") which,
        );
    }
    ret
}

/// use sbi call to set timer
pub fn set_timer(timer: usize) {
    sbi_call(SBI_SET_TIMER, timer, 0, 0);
}

/// use sbi call to putchar in console (qemu uart handler)
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

/// use sbi call to getchar from console (qemu uart handler)
pub fn console_getchar() -> usize {
    let ch = sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0);
    // Legacy SBI returns -1 when no input is available; readers expect zero.
    if ch == usize::MAX {
        0
    } else {
        ch
    }
}

/// use sbi call to shutdown the kernel
pub fn shutdown() -> ! {
    // SBI System Reset: shutdown (type 0), no failure reason (reason 0).
    unsafe {
        asm!(
            "ecall",
            in("a7") SBI_SYSTEM_RESET,
            in("a6") 0,
            inlateout("a0") 0usize => _,
            inlateout("a1") 0usize => _,
        );
    }
    panic!("It should shutdown!");
}
