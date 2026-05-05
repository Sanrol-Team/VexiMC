/**
 * VeixEngine：C++ 模组侧 ABI（与 Rust 产物 veix_mc_bridge.dll 对应）。
 *
 * 典型流程：
 * 1. JVM 已由启动器拉起，Java Agent 加载并对 VeixRegistryBridge 可见。
 * 2. 游戏进程加载 veix_mc_bridge.dll（与 veix-agent.jar 同目录，或由 Agent System.load）。
 * 3. C++ 侧取得 JavaVM*（例如 JNI_OnLoad 导出、或由宿主传入），必要时调用 veix_mc_set_java_vm。
 * 4. veix_mc_register_item("veix", "ruby_gem") 等价于 Java registerItemByKey("veix:ruby_gem")。
 *
 * 返回值：与 veix.VeixRegistryBridge.registerItemByKey 一致（0 成功；负数为错误码）。
 */

#ifndef VEIX_MOD_API_H
#define VEIX_MOD_API_H

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) && defined(VEIX_MOD_BUILD_DLL)
#  define VEIX_MOD_API __declspec(dllexport)
#elif defined(_WIN32) && defined(VEIX_MOD_USE_DLL)
#  define VEIX_MOD_API __declspec(dllimport)
#else
#  define VEIX_MOD_API
#endif

/** 若 DLL 非由 System.load 加载导致未执行 JNI_OnLoad，可显式设置 VM（与 JavaVM* 同 ABI）。 */
VEIX_MOD_API int veix_mc_set_java_vm(void *java_vm);

/**
 * 注册物品（namespace + path，无需冒号拼接）。
 * 依赖 Java 侧 Mojang 映射类名；混淆客户端需另行适配。
 */
VEIX_MOD_API int veix_mc_register_item(const char *namespace_utf8,
                                     const char *path_utf8);

/**
 * 与 Java {@code VeixRegistryBridge.registerItemByKey} 相同：完整 ID（UTF-8），如 {@code "veix:ruby_gem"}。
 */
VEIX_MOD_API int veix_mc_register_item_by_key(const char *namespaced_id_utf8);

/** 若已通过 JNI_OnLoad 或 veix_mc_set_java_vm 缓存 JavaVM 则返回 1，否则 0。 */
VEIX_MOD_API int veix_mc_java_vm_ready(void);

#ifdef __cplusplus
}
#endif

#endif /* VEIX_MOD_API_H */
