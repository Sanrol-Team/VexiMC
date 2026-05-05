//! veix-sched C 侧进程封装：`CreateProcessW`（UTF-16 可写命令行）。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("当前平台未实现（仅 Windows）")]
    UnsupportedPlatform,
    #[error("veix_process_create_utf16 失败: 错误码 {0}")]
    CreateFailed(i32),
    #[error("等待进程超时")]
    Timeout,
    #[error("等待进程失败")]
    WaitFailed,
}

#[cfg(target_os = "windows")]
use crate::build1::native_veix_sched::{
    veix_process_close, veix_process_create_utf16, veix_process_wait, VeixProcessOpaque,
};

pub struct ChildProcess {
    #[cfg(target_os = "windows")]
    inner: std::ptr::NonNull<VeixProcessOpaque>,
    #[cfg(not(target_os = "windows"))]
    _private: (),
}

#[cfg(target_os = "windows")]
unsafe impl Send for ChildProcess {}

impl ChildProcess {
    /// `cmdline` 须以 **可写** UTF-16 缓冲区传给 Win32，且 **以 0 结尾**。
    pub fn create_utf16(cmdline: &mut [u16]) -> Result<Self, ProcessError> {
        #[cfg(target_os = "windows")]
        {
            if cmdline.is_empty() || cmdline[cmdline.len() - 1] != 0 {
                return Err(ProcessError::CreateFailed(-100));
            }
            let mut p: *mut VeixProcessOpaque = std::ptr::null_mut();
            let rc =
                unsafe { veix_process_create_utf16(&mut p, cmdline.as_mut_ptr()) };
            if rc != 0 || p.is_null() {
                return Err(ProcessError::CreateFailed(rc));
            }
            Ok(Self {
                inner: unsafe { std::ptr::NonNull::new_unchecked(p) },
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = cmdline;
            Err(ProcessError::UnsupportedPlatform)
        }
    }

    /// `timeout_ms == u32::MAX` 时等价 Win32 `INFINITE`。
    /// 成功时返回子进程退出码（Win32 `GetExitCodeProcess`）。
    pub fn wait(&self, timeout_ms: u32) -> Result<u32, ProcessError> {
        #[cfg(target_os = "windows")]
        {
            let mut code: u32 = 0;
            let rc = unsafe {
                veix_process_wait(
                    self.inner.as_ptr(),
                    timeout_ms,
                    &mut code as *mut u32,
                )
            };
            match rc {
                0 => Ok(code),
                1 => Err(ProcessError::Timeout),
                _ => Err(ProcessError::WaitFailed),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = timeout_ms;
            Err(ProcessError::UnsupportedPlatform)
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::ChildProcess;

    #[test]
    fn wait_cmd_exit_code() {
        let mut cmd: Vec<u16> = "cmd.exe /c exit 42\0"
            .encode_utf16()
            .collect();
        let p = ChildProcess::create_utf16(&mut cmd).expect("create");
        let code = p.wait(u32::MAX).expect("wait");
        assert_eq!(code, 42);
    }
}

#[cfg(target_os = "windows")]
impl Drop for ChildProcess {
    fn drop(&mut self) {
        unsafe {
            veix_process_close(self.inner.as_ptr());
        }
    }
}
