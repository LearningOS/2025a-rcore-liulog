//! Process management syscalls
use crate::mm::MapPermission;
use crate::mm::PageTable;
use crate::mm::VirtAddr;
use crate::mm::{translated_ptr, translated_ptr_mut, translated_timeval};
use crate::task::current_user_token;
use crate::task::{
    change_program_brk, current_user_mmap_one_page, exit_current_and_run_next,
    get_current_syscall_count, suspend_current_and_run_next, current_user_unmap_one_page
};
use crate::timer::get_time_us;

/// TimeVal struct for sys_get_time
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    /// seconds
    pub sec: usize,
    /// microseconds
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
    let timeval = translated_timeval(current_user_token(), ts);
    let us = get_time_us();
    timeval.sec = us / 1_000_000;
    timeval.usec = us % 1_000_000;
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => match translated_ptr(current_user_token(), id as *const u8) {
            None => return -1,
            Some(c) => return c as isize,
        },
        1 => match translated_ptr_mut(current_user_token(), id as *const u8) {
            None => return -1,
            Some(c) => {
                *c = data as u8;
                return 0;
            }
        },
        2 => get_current_syscall_count(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if start & 0xFFF != 0|| port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let mut remaining_len = len as isize;
    let mut va = start;
    // Check if the page is already mapped
    while remaining_len > 0 {
        if let Some(pte) = PageTable::from_token(current_user_token()).translate(VirtAddr::from(va).floor()){
            if pte.is_valid() {
                return -1;
            }
        }
        remaining_len -= 4096;
        va += 4096;
    }
    // Map the pages
    remaining_len = len as isize;
    va = start;
    while remaining_len > 0 {
        // Map the page, if failed, return -1
        if current_user_mmap_one_page(
            VirtAddr::from(va),
            MapPermission::U | MapPermission::from_bits_truncate((port << 1) as u8),   // this port is syscall used, it needs to shift left 1 to convert to MapPermission
        )
        .is_err()
        {
            return -1;
        }
        remaining_len -= 4096;
        va += 4096;
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start & 0xFFF != 0 || len == 0 {
        return -1;
    }
    let mut remaining_len = len as isize;
    let mut va = start;
    // Check if the page is already mapped
    while remaining_len > 0 {
        if PageTable::from_token(current_user_token())
            .translate(VirtAddr::from(va).into())
            .is_none()
        {
            return -1;
        }
        remaining_len -= 4096;
        va += 4096;
    }
    // Unmap the pages
    remaining_len = len as isize;
    va = start;
    while remaining_len > 0 {
        // Map the page, if failed, return -1
        if current_user_unmap_one_page(VirtAddr::from(va)).is_err() {
            return -1;
        }
        remaining_len -= 4096;
        va += 4096;
    }
    0
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
