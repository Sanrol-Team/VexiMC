/**
 * Windows 线程池调度：CriticalSection + ConditionVariable，堆内存来自 HeapAlloc。
 * 不使用 CRT malloc / printf。
 */

#include <veix/sched.h>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <Windows.h>

typedef struct veix_work_item veix_work_item;
struct veix_work_item {
  veix_task_fn fn;
  void *ctx;
  veix_work_item *next;
};

struct veix_scheduler {
  HANDLE *threads;
  uint32_t thread_count;
  volatile LONG running;
  CRITICAL_SECTION q_lock;
  CONDITION_VARIABLE q_cv;
  veix_work_item *head;
  veix_work_item *tail;
};

static void *veix_heap_alloc(size_t n) {
  return HeapAlloc(GetProcessHeap(), 0, n);
}

static void veix_heap_free(void *p) {
  if (p) {
    HeapFree(GetProcessHeap(), 0, p);
  }
}

static DWORD WINAPI veix_worker_main(void *param) {
  veix_scheduler_t *s = (veix_scheduler_t *)param;

  for (;;) {
    EnterCriticalSection(&s->q_lock);
    while (s->head == NULL && s->running) {
      SleepConditionVariableCS(&s->q_cv, &s->q_lock, INFINITE);
    }
    if (!s->running && s->head == NULL) {
      LeaveCriticalSection(&s->q_lock);
      break;
    }
    veix_work_item *w = s->head;
    if (w) {
      s->head = w->next;
      if (s->head == NULL) {
        s->tail = NULL;
      }
    }
    LeaveCriticalSection(&s->q_lock);

    if (w) {
      if (w->fn) {
        w->fn(w->ctx);
      }
      veix_heap_free(w);
    }
  }
  return 0;
}

int veix_sched_create(veix_scheduler_t **out, uint32_t worker_count) {
  veix_scheduler_t *s;
  uint32_t i;
  SYSTEM_INFO si;

  if (!out) {
    return -1;
  }
  *out = NULL;

  s = (veix_scheduler_t *)veix_heap_alloc(sizeof(*s));
  if (!s) {
    return -2;
  }
  s->threads = NULL;
  s->thread_count = 0;
  s->running = 1;
  s->head = NULL;
  s->tail = NULL;
  InitializeCriticalSection(&s->q_lock);
  InitializeConditionVariable(&s->q_cv);

  if (worker_count == 0) {
    GetSystemInfo(&si);
    worker_count = si.dwNumberOfProcessors;
    if (worker_count == 0) {
      worker_count = 1;
    }
  }

  s->threads = (HANDLE *)veix_heap_alloc(sizeof(HANDLE) * worker_count);
  if (!s->threads) {
    DeleteCriticalSection(&s->q_lock);
    veix_heap_free(s);
    return -3;
  }
  s->thread_count = worker_count;

  for (i = 0; i < worker_count; i++) {
    s->threads[i] = CreateThread(NULL, 0, veix_worker_main, s, 0, NULL);
    if (!s->threads[i]) {
      uint32_t j;
      s->running = 0;
      WakeAllConditionVariable(&s->q_cv);
      for (j = 0; j < i; j++) {
        WaitForSingleObject(s->threads[j], INFINITE);
        CloseHandle(s->threads[j]);
      }
      DeleteCriticalSection(&s->q_lock);
      veix_heap_free(s->threads);
      veix_heap_free(s);
      return -4;
    }
  }

  *out = s;
  return 0;
}

void veix_sched_destroy(veix_scheduler_t *s) {
  uint32_t i;

  if (!s) {
    return;
  }

  EnterCriticalSection(&s->q_lock);
  s->running = 0;
  WakeAllConditionVariable(&s->q_cv);
  LeaveCriticalSection(&s->q_lock);

  for (i = 0; i < s->thread_count; i++) {
    WaitForSingleObject(s->threads[i], INFINITE);
    CloseHandle(s->threads[i]);
  }

  veix_heap_free(s->threads);

  EnterCriticalSection(&s->q_lock);
  while (s->head) {
    veix_work_item *w = s->head;
    s->head = w->next;
    veix_heap_free(w);
  }
  LeaveCriticalSection(&s->q_lock);

  DeleteCriticalSection(&s->q_lock);
  veix_heap_free(s);
}

int veix_sched_submit(veix_scheduler_t *s, veix_task_fn fn, void *ctx) {
  veix_work_item *w;

  if (!s || !fn) {
    return -1;
  }

  w = (veix_work_item *)veix_heap_alloc(sizeof(*w));
  if (!w) {
    return -2;
  }
  w->fn = fn;
  w->ctx = ctx;
  w->next = NULL;

  EnterCriticalSection(&s->q_lock);
  if (!s->running) {
    LeaveCriticalSection(&s->q_lock);
    veix_heap_free(w);
    return -3;
  }
  if (s->tail) {
    s->tail->next = w;
  } else {
    s->head = w;
  }
  s->tail = w;
  WakeConditionVariable(&s->q_cv);
  LeaveCriticalSection(&s->q_lock);

  return 0;
}
