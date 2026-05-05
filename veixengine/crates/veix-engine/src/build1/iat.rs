//! 导入地址表（IAT）劫持：在已加载模块内将指定 `dll!name` 的解析地址替换为拦截例程。

use std::ffi::CString;
use std::os::raw::c_char;
use thiserror::Error;

#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
extern "C" {
    /// 在 `module` 指向的映像中，将 `import_dll!import_name` 的 IAT 项改为 `detour`。
    /// 成功时 `out_original` 为此前 IAT 中保存的地址（可转交或链式调用）。
    fn veix_iat_hijack_named(
        module: *mut std::ffi::c_void,
        import_dll: *const c_char,
        import_name: *const c_char,
        detour: *mut std::ffi::c_void,
        out_original: *mut *mut std::ffi::c_void,
    ) -> i32;
}

/// 已加载模块的基址（例如 `GetModuleHandleW` 返回值）。
#[derive(Debug, Clone, Copy)]
pub struct ModuleBase(pub *mut std::ffi::c_void);

#[derive(Debug, Error)]
pub enum IatError {
    #[error("IAT 劫持失败: 未找到导入或平台桩未启用 (code {0})")]
    Failed(i32),
    #[error("无效模块基址")]
    BadModule,
    #[error("字符串包含内嵌 0 字节")]
    Nul,
}

/// 将 `import_dll` 中的指定按名导入重定向到 `detour`，返回原 IAT 槽位中的地址。
#[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
pub fn hijack_named(
    module: ModuleBase,
    import_dll: &str,
    import_name: &str,
    detour: *mut std::ffi::c_void,
) -> Result<*mut std::ffi::c_void, IatError> {
    if module.0.is_null() {
        return Err(IatError::BadModule);
    }
    let dll = CString::new(import_dll).map_err(|_| IatError::Nul)?;
    let name = CString::new(import_name).map_err(|_| IatError::Nul)?;
    let mut orig: *mut std::ffi::c_void = std::ptr::null_mut();
    let code = unsafe {
        veix_iat_hijack_named(
            module.0,
            dll.as_ptr(),
            name.as_ptr(),
            detour,
            &mut orig,
        )
    };
    if code == 0 {
        Ok(orig)
    } else {
        Err(IatError::Failed(code))
    }
}

/// 非 Windows 或禁用了 `inject-native` 时的占位实现。
#[cfg(not(all(feature = "inject-native", target_os = "windows", target_env = "msvc")))]
pub fn hijack_named(
    _module: ModuleBase,
    _import_dll: &str,
    _import_name: &str,
    _detour: *mut std::ffi::c_void,
) -> Result<*mut std::ffi::c_void, IatError> {
    Err(IatError::Failed(-1))
}

