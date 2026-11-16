//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{translated_user_buffer, MapPermission, VirtAddr};
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, mmap_current,
    munmap_current, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

/// End of the canonical lower-half user address range in Sv39.
const USER_VA_LIMIT: usize = 1usize << 38;

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

/// Get time with second and microsecond through the current user page table.
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let value = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let source = unsafe {
        core::slice::from_raw_parts(
            (&value as *const TimeVal).cast::<u8>(),
            core::mem::size_of::<TimeVal>(),
        )
    };
    let Some(buffers) = translated_user_buffer(
        current_user_token(),
        ts.cast::<u8>(),
        core::mem::size_of::<TimeVal>(),
        true,
    ) else {
        return -1;
    };
    let mut copied = 0;
    for buffer in buffers {
        let next = copied + buffer.len();
        buffer.copy_from_slice(&source[copied..next]);
        copied = next;
    }
    0
}

/// Trace user memory or the current task's syscall counts.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => translated_user_buffer(current_user_token(), id as *const u8, 1, false)
            .map(|buffers| buffers[0][0] as isize)
            .unwrap_or(-1),
        1 => {
            let Some(mut buffers) =
                translated_user_buffer(current_user_token(), id as *const u8, 1, true)
            else {
                return -1;
            };
            buffers[0][0] = data as u8;
            0
        }
        2 => crate::task::syscall_count(id)
            .map(|count| count as isize)
            .unwrap_or(-1),
        _ => -1,
    }
}

/// Map anonymous user pages.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    if start % PAGE_SIZE != 0 || prot == 0 || prot & !0x7 != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let Some(raw_end) = start.checked_add(len) else {
        return -1;
    };
    let Some(end) = raw_end
        .checked_add(PAGE_SIZE - 1)
        .map(|value| value / PAGE_SIZE * PAGE_SIZE)
    else {
        return -1;
    };
    if end > USER_VA_LIMIT {
        return -1;
    }
    let mut permission = MapPermission::empty();
    if prot & 0x1 != 0 {
        permission |= MapPermission::R;
    }
    if prot & 0x2 != 0 {
        permission |= MapPermission::W;
    }
    if prot & 0x4 != 0 {
        permission |= MapPermission::X;
    }
    if mmap_current(VirtAddr::from(start), VirtAddr::from(end), permission) {
        0
    } else {
        -1
    }
}

/// Unmap a complete anonymous mapping.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let Some(raw_end) = start.checked_add(len) else {
        return -1;
    };
    let Some(end) = raw_end
        .checked_add(PAGE_SIZE - 1)
        .map(|value| value / PAGE_SIZE * PAGE_SIZE)
    else {
        return -1;
    };
    if end > USER_VA_LIMIT {
        return -1;
    }
    if munmap_current(VirtAddr::from(start), VirtAddr::from(end)) {
        0
    } else {
        -1
    }
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
