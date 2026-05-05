/**
 * Veix 线程调度器（C API，无独立 main；可与宿主进程或 DLL 链接）。
 *
 * 实现要求：不依赖 C 标准库的 malloc/stdio，Windows 下使用 Heap API + Win32 线程原语。
 */

#ifndef VEIX_SCHED_H
#define VEIX_SCHED_H

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32) && defined(VEIX_SCHED_BUILD_SHARED)
#  define VEIX_SCHED_API __declspec(dllexport)
#elif defined(_WIN32) && defined(VEIX_SCHED_USE_SHARED)
#  define VEIX_SCHED_API __declspec(dllimport)
#else
#  define VEIX_SCHED_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*veix_task_fn)(void *ctx);

typedef struct veix_scheduler veix_scheduler_t;

/**
 * 创建工作线程池。worker_count==0 时使用逻辑处理器数量。
 * 成功返回 0。
 */
VEIX_SCHED_API int veix_sched_create(veix_scheduler_t **out, uint32_t worker_count);

/** 释放调度器；未执行任务仍会被尽快排空（running 置 0 后协作退出）。 */
VEIX_SCHED_API void veix_sched_destroy(veix_scheduler_t *s);

/**
 * 投递任务（FIFO）。可在任意线程调用。
 * 成功返回 0；调度器已销毁返回非 0。
 */
VEIX_SCHED_API int veix_sched_submit(veix_scheduler_t *s, veix_task_fn fn, void *ctx);

#ifdef __cplusplus
}
#endif

#endif /* VEIX_SCHED_H */
