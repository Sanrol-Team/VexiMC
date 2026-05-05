//! Build1 子系统：平台原语 + 纯 Rust 侧车结构。

pub mod iat;
pub mod jit;
#[cfg(target_os = "windows")]
pub(crate) mod native_veix_sched;
pub mod process;
pub mod proxy;
pub mod sched;
pub mod shadow;

use shadow::ShadowArena;

/// VeixEngine Build1 运行时句柄：持有影子映射并暴露注入层初始化入口。
pub struct Build1 {
    pub shadow: ShadowArena<u64, ()>,
}

impl Default for Build1 {
    fn default() -> Self {
        Self {
            shadow: ShadowArena::new(),
        }
    }
}

impl Build1 {
    pub fn new() -> Self {
        Self::default()
    }
}
