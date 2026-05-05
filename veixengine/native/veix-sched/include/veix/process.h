/**
 * Veix 进程抽象（子进程创建/等待）；无 main，供宿主或注入层加载。
 */

#ifndef VEIX_PROCESS_H
#define VEIX_PROCESS_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) && defined(VEIX_SCHED_BUILD_SHARED)
#  define VEIX_PROC_API __declspec(dllexport)
#elif defined(_WIN32) && defined(VEIX_SCHED_USE_SHARED)
#  define VEIX_PROC_API __declspec(dllimport)
#else
#  define VEIX_PROC_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct veix_process veix_process_t;

/**
 * CreateProcessW：cmdline 为 UTF-16 LE、以 0 结尾的可写缓冲区（Win32 规则）。
 */
VEIX_PROC_API int veix_process_create_utf16(veix_process_t **out,
                                            uint16_t *cmdline);

/**
 * 等待进程结束；超时毫秒，INFINITE=(DWORD)-1。
 * 返回 0 表示等到退出；若 `exit_code_out` 非 NULL 则写入 Win32 退出码。
 */
VEIX_PROC_API int veix_process_wait(veix_process_t *p, uint32_t timeout_ms,
                                   uint32_t *exit_code_out);

VEIX_PROC_API void veix_process_close(veix_process_t *p);

#ifdef __cplusplus
}
#endif

#endif /* VEIX_PROCESS_H */
