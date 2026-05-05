//! 虚假动态链接库代理：侧载「代理 DLL」后，将解析请求转交给真实 `target` 中的同名符号。
//! Build1 提供 `load_real` / `resolve` 运行期原语；完整 `DLL/SO` 的导出表需由构建步骤生成（def / 版本脚本）。

use std::ffi::CString;
use std::os::raw::c_char;
use thiserror::Error;

#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
extern "C" {
    fn veix_proxy_load_real(path_utf16: *const u16) -> i32;
    fn veix_proxy_resolve(name: *const c_char) -> *mut std::ffi::c_void;
}

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("加载真实库失败 (code {0})")]
    LoadFailed(i32),
    #[error("解析符号失败")]
    ResolveFailed,
    #[error("路径或符号含非法空字符")]
    Nul,
}

/// 以宽字符路径加载目标 DLL（供代理 `DllMain` 内调用一次）。
#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
pub fn load_real(path: &std::ffi::OsStr) -> Result<(), ProxyError> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.encode_wide().chain(std::iter::once(0)).collect();
    let code = unsafe { veix_proxy_load_real(wide.as_ptr()) };
    if code == 0 {
        Ok(())
    } else {
        Err(ProxyError::LoadFailed(code))
    }
}

#[cfg(not(all(feature = "inject-native", target_os = "windows", target_env = "msvc")))]
pub fn load_real(_path: &std::ffi::OsStr) -> Result<(), ProxyError> {
    Err(ProxyError::LoadFailed(-1))
}

/// 自真实库解析 `name`（ANSI），用于代理导出函数转发。
#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
pub fn resolve(name: &str) -> Result<*mut std::ffi::c_void, ProxyError> {
    let c = CString::new(name).map_err(|_| ProxyError::Nul)?;
    let p = unsafe { veix_proxy_resolve(c.as_ptr()) };
    if p.is_null() {
        Err(ProxyError::ResolveFailed)
    } else {
        Ok(p)
    }
}

#[cfg(not(all(feature = "inject-native", target_os = "windows", target_env = "msvc")))]
pub fn resolve(_name: &str) -> Result<*mut std::ffi::c_void, ProxyError> {
    Err(ProxyError::ResolveFailed)
}
