// Proxy DLL runtime: load real module, resolve exports for forwarding.

#include <Windows.h>

static HMODULE g_veix_real_module = nullptr;

extern "C" int veix_proxy_load_real(const wchar_t* path_utf16) {
  if (!path_utf16) return -1;
  if (g_veix_real_module) {
    FreeLibrary(g_veix_real_module);
    g_veix_real_module = nullptr;
  }
  g_veix_real_module = LoadLibraryW(path_utf16);
  return g_veix_real_module ? 0 : static_cast<int>(GetLastError());
}

extern "C" void* veix_proxy_resolve(const char* name) {
  if (!g_veix_real_module || !name) return nullptr;
  return reinterpret_cast<void*>(GetProcAddress(g_veix_real_module, name));
}
