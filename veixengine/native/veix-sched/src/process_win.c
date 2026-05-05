/**
 * 子进程封装：CreateProcessW，无 CRT stdio。
 */

#include <veix/process.h>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <Windows.h>

struct veix_process {
  HANDLE process_handle;
};

static void veix_zero(void *p, size_t n) {
  unsigned char *b = (unsigned char *)p;
  size_t i;
  for (i = 0; i < n; i++) {
    b[i] = 0;
  }
}

static void *veix_heap_alloc(size_t n) {
  return HeapAlloc(GetProcessHeap(), 0, n);
}

static void veix_heap_free(void *p) {
  if (p) {
    HeapFree(GetProcessHeap(), 0, p);
  }
}

int veix_process_create_utf16(veix_process_t **out, uint16_t *cmdline) {
  STARTUPINFOW si;
  PROCESS_INFORMATION pi;

  if (!out || !cmdline) {
    return -1;
  }
  *out = NULL;

  veix_zero(&si, sizeof(si));
  si.cb = sizeof(si);
  veix_zero(&pi, sizeof(pi));

  if (!CreateProcessW(
          NULL,
          (LPWSTR)cmdline,
          NULL,
          NULL,
          FALSE,
          CREATE_NO_WINDOW,
          NULL,
          NULL,
          &si,
          &pi)) {
    return -(int)GetLastError();
  }

  CloseHandle(pi.hThread);

  {
    veix_process_t *p =
        (veix_process_t *)veix_heap_alloc(sizeof(veix_process_t));
    if (!p) {
      TerminateProcess(pi.hProcess, 1);
      CloseHandle(pi.hProcess);
      return -2;
    }
    p->process_handle = pi.hProcess;
    *out = p;
  }
  return 0;
}

int veix_process_wait(veix_process_t *p, uint32_t timeout_ms,
                      uint32_t *exit_code_out) {
  DWORD ms;
  DWORD wr;

  if (!p) {
    return -1;
  }
  ms = timeout_ms;
  wr = WaitForSingleObject(p->process_handle, ms);
  if (wr == WAIT_OBJECT_0) {
    if (exit_code_out) {
      DWORD code;
      if (!GetExitCodeProcess(p->process_handle, &code)) {
        return -(int)GetLastError();
      }
      *exit_code_out = code;
    }
    return 0;
  }
  if (wr == WAIT_TIMEOUT) {
    return 1;
  }
  return -2;
}

void veix_process_close(veix_process_t *p) {
  if (!p) {
    return;
  }
  if (p->process_handle) {
    CloseHandle(p->process_handle);
  }
  veix_heap_free(p);
}
