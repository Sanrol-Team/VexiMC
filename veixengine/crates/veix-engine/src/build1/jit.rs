//! JIT 跳板：在可执行槽位中写入 `jmp` 到 `target` 的指令序列，供 IAT/虚表等指向。

use std::ffi::c_void;
use thiserror::Error;

#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
extern "C" {
    fn veix_jit_alloc_trampoline_x64(target: *mut c_void) -> *mut c_void;
    fn veix_jit_free_slot(slot: *mut c_void);
}

#[derive(Debug, Error)]
pub enum JitError {
    #[error("分配可执行跳板失败")]
    AllocFailed,
}

/// x64：分配 RWX 槽位并写入跳转到 `target` 的跳板。
#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
pub fn trampoline_x64(target: *mut c_void) -> Result<TrampolineSlot, JitError> {
    let p = unsafe { veix_jit_alloc_trampoline_x64(target) };
    if p.is_null() {
        Err(JitError::AllocFailed)
    } else {
        Ok(TrampolineSlot(p))
    }
}

#[cfg(not(all(feature = "inject-native", target_os = "windows", target_env = "msvc")))]
pub fn trampoline_x64(_target: *mut c_void) -> Result<TrampolineSlot, JitError> {
    Err(JitError::AllocFailed)
}

/// 可执行内存槽位，释放时归还系统。
pub struct TrampolineSlot(*mut c_void);

impl Drop for TrampolineSlot {
    fn drop(&mut self) {
        #[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
        unsafe {
            veix_jit_free_slot(self.0);
        }
    }
}

impl TrampolineSlot {
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}
