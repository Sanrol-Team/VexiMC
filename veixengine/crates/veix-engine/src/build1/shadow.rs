//! 影子对象映射：与宿主对象并行的侧车数据，不修改宿主布局。
//!
//! Build1 提供通用 `ShadowArena`；与 JVM / 游戏对象的绑定在后续版本按句柄或稳定键接入。

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// 单调分配的阴影对象 ID（进程内唯一）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShadowId(pub u64);

struct Inner<K: Eq + std::hash::Hash, T> {
    next: AtomicU64,
    by_key: RwLock<HashMap<K, ShadowId>>,
    objects: RwLock<HashMap<ShadowId, T>>,
}

/// 线程安全的影子对象仓库。
pub struct ShadowArena<K: Eq + std::hash::Hash, T> {
    inner: std::sync::Arc<Inner<K, T>>,
}

impl<K: Eq + std::hash::Hash + Clone, T> ShadowArena<K, T> {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Inner {
                next: AtomicU64::new(1),
                by_key: RwLock::new(HashMap::new()),
                objects: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// 为键分配或复用 `ShadowId`，并写入/更新对象负载。
    pub fn upsert(&self, key: K, value: T) -> ShadowId {
        let mut by_key = self.inner.by_key.write();
        if let Some(id) = by_key.get(&key).copied() {
            self.inner.objects.write().insert(id, value);
            return id;
        }
        let id = ShadowId(self.inner.next.fetch_add(1, Ordering::Relaxed));
        by_key.insert(key.clone(), id);
        self.inner.objects.write().insert(id, value);
        id
    }

    pub fn get(&self, id: ShadowId) -> Option<T>
    where
        T: Clone,
    {
        self.inner.objects.read().get(&id).cloned()
    }

    pub fn remove(&self, id: ShadowId) -> Option<T> {
        let mut objs = self.inner.objects.write();
        let v = objs.remove(&id)?;
        let mut by_key = self.inner.by_key.write();
        by_key.retain(|_, v_id| *v_id != id);
        Some(v)
    }
}

impl<K: Eq + std::hash::Hash + Clone, T> Clone for ShadowArena<K, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
