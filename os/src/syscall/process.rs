//! Process management syscalls
//!
use crate::mm::MapPermission;
use crate::mm::PageTable;
use crate::mm::VirtAddr;
use crate::timer::get_time_us;
use alloc::sync::Arc;
use crate::config::BIG_STRIDE;

use crate::{
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str, translated_timeval},
    task::{
        add_task, current_task, current_user_mmap_one_page, current_user_token,
        current_user_unmap_one_page, exit_current_and_run_next, suspend_current_and_run_next,
    },
};

/// TimeVal struct for sys_get_time
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    /// seconds
    pub sec: usize,
    /// microseconds
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
    let timeval = translated_timeval(current_user_token(), ts);
    let us = get_time_us();
    timeval.sec = us / 1_000_000;
    timeval.usec = us % 1_000_000;
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    if start & 0xFFF != 0 || port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let mut remaining_len = len as isize;
    let mut va = start;
    // Check if the page is already mapped
    while remaining_len > 0 {
        if let Some(pte) =
            PageTable::from_token(current_user_token()).translate(VirtAddr::from(va).floor())
        {
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
            MapPermission::U | MapPermission::from_bits_truncate((port << 1) as u8), // this port is syscall used, it needs to shift left 1 to convert to MapPermission
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

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
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
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    // Check if the app exists
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(all_data.as_slice());
        let new_pid = new_task.pid.0;
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        trap_cx.x[10] = 0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!("kernel:pid[{}] sys_set_priority", current_task().unwrap().pid.0);
    // We need prio >= 2
    if prio <= 1{
        return -1;
    }
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.priority = prio as usize;
    inner.pass = BIG_STRIDE / prio as usize;
    prio
}
