//! VeixEngine Build1 — 基于 Veix 注入层的编排入口。
//!
//! 注入方法组合：
//! - DLL/SO 代理加载 + 导出解析（`build1::proxy`）
//! - 导入地址表（IAT）劫持（`build1::iat`）
//! - JIT 可执行槽位与跳板（`build1::jit`）
//! - 影子对象侧车映射（`build1::shadow`）
//! - C `veix-sched` 线程/进程调度（`build1::sched`、`build1::process`）

pub mod build1;

pub use build1::Build1;
