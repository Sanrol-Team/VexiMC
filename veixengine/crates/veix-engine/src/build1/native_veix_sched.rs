//! 单一 `#[link]` 入口：静态库 `veix_sched`（sched_win + process_win）。

#![cfg(target_os = "windows")]

use std::ffi::c_void;

/// C 侧不透明句柄；仅占位，禁止构造。
#[repr(C)]
pub struct VeixSchedulerOpaque {
    _private: [u8; 0],
}

#[repr(C)]
pub struct VeixProcessOpaque {
    _private: [u8; 0],
}

pub type VeixTaskFn = unsafe extern "C" fn(*mut c_void);

#[link(name = "veix_sched", kind = "static")]
extern "C" {
    pub fn veix_sched_create(out: *mut *mut VeixSchedulerOpaque, worker_count: u32) -> i32;
    pub fn veix_sched_destroy(s: *mut VeixSchedulerOpaque);
    pub fn veix_sched_submit(
        s: *mut VeixSchedulerOpaque,
        f: VeixTaskFn,
        ctx: *mut c_void,
    ) -> i32;

    pub fn veix_process_create_utf16(
        out: *mut *mut VeixProcessOpaque,
        cmdline: *mut u16,
    ) -> i32;
    pub fn veix_process_wait(
        p: *mut VeixProcessOpaque,
        timeout_ms: u32,
        exit_code_out: *mut u32,
    ) -> i32;
    pub fn veix_process_close(p: *mut VeixProcessOpaque);
}
