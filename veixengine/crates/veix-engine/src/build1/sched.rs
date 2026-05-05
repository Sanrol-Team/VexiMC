//! veix-sched（C）线程池：由 `build.rs` 编译为静态库 `veix_sched` 并链接。

use std::ptr::NonNull;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchedError {
    #[error("当前平台未编译 veix_sched（仅 Windows）")]
    UnsupportedPlatform,
    #[error("veix_sched_create 失败: 错误码 {0}")]
    CreateFailed(i32),
    #[error("veix_sched_submit 失败: 错误码 {0}")]
    SubmitFailed(i32),
}

#[cfg(target_os = "windows")]
use crate::build1::native_veix_sched::{
    veix_sched_create, veix_sched_destroy, veix_sched_submit, VeixSchedulerOpaque,
};

/// 与 C 线程池句柄共享：可在多线程 `submit`；仅允许单所有者 `Drop`。
pub struct Scheduler {
    #[cfg(target_os = "windows")]
    inner: NonNull<VeixSchedulerOpaque>,
    #[cfg(not(target_os = "windows"))]
    _private: (),
}

#[cfg(target_os = "windows")]
unsafe impl Send for Scheduler {}

#[cfg(target_os = "windows")]
unsafe impl Sync for Scheduler {}

impl Scheduler {
    /// `worker_count == 0` 时使用 CPU 逻辑核心数（与 C 一致）。
    pub fn new(worker_count: u32) -> Result<Self, SchedError> {
        #[cfg(target_os = "windows")]
        {
            let mut p: *mut VeixSchedulerOpaque = std::ptr::null_mut();
            let rc = unsafe { veix_sched_create(&mut p, worker_count) };
            if rc != 0 || p.is_null() {
                return Err(SchedError::CreateFailed(rc));
            }
            Ok(Self {
                inner: unsafe { NonNull::new_unchecked(p) },
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = worker_count;
            Err(SchedError::UnsupportedPlatform)
        }
    }

    pub fn submit<F: FnOnce() + Send + 'static>(&self, f: F) -> Result<(), SchedError> {
        #[cfg(target_os = "windows")]
        {
            let ctx = Box::into_raw(Box::new(f)) as *mut std::ffi::c_void;
            let rc = unsafe {
                veix_sched_submit(self.inner.as_ptr(), invoke_once::<F>, ctx)
            };
            if rc != 0 {
                unsafe {
                    drop(Box::from_raw(ctx as *mut F));
                }
                return Err(SchedError::SubmitFailed(rc));
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = f;
            Err(SchedError::UnsupportedPlatform)
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "C" fn invoke_once<F: FnOnce() + Send>(ctx: *mut std::ffi::c_void) {
    let f = Box::from_raw(ctx as *mut F);
    f();
}

#[cfg(target_os = "windows")]
impl Drop for Scheduler {
    fn drop(&mut self) {
        unsafe {
            veix_sched_destroy(self.inner.as_ptr());
        }
    }
}
