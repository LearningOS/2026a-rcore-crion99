//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};

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

/// Read/write one byte in the current task or query its syscall history.
///
/// Chapter 3 applications share the kernel address space, so the exercise
/// deliberately performs the byte access directly.  Chapter 4 replaces this
/// with access through the user page table.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => unsafe { (id as *const u8).read_volatile() as isize },
        1 => {
            unsafe { (id as *mut u8).write_volatile(data as u8) };
            0
        }
        2 => crate::task::syscall_count(id)
            .map(|count| count as isize)
            .unwrap_or(-1),
        _ => -1,
    }
}
