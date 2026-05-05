// IAT hijack: locate PE64 import directory, match DLL + name, patch IAT slot.

#include <Windows.h>

#include <cstdint>
#include <cstring>

namespace {

bool match_import_dll_name(const char* in_pe, const char* requested) {
  return _stricmp(in_pe, requested) == 0;
}

}  // namespace

extern "C" int veix_iat_hijack_named(void* module, const char* import_dll,
                                      const char* import_name, void* detour,
                                      void** out_original) {
  if (!module || !import_dll || !import_name || !detour || !out_original) return -2;
  *out_original = nullptr;

  auto* base = reinterpret_cast<uint8_t*>(module);
  auto* dos = reinterpret_cast<PIMAGE_DOS_HEADER>(base);
  if (dos->e_magic != IMAGE_DOS_SIGNATURE) return -3;

  auto* nt = reinterpret_cast<PIMAGE_NT_HEADERS64>(base + dos->e_lfanew);
  if (nt->Signature != IMAGE_NT_SIGNATURE) return -4;
  if (nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC) return -5;

  auto* dir = &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT];
  if (dir->VirtualAddress == 0 || dir->Size == 0) return -6;

  auto* imp = reinterpret_cast<PIMAGE_IMPORT_DESCRIPTOR>(base + dir->VirtualAddress);

  for (; imp->Name; ++imp) {
    const char* dll = reinterpret_cast<const char*>(base + imp->Name);
    if (!match_import_dll_name(dll, import_dll)) continue;

    if (imp->OriginalFirstThunk == 0) return -7;

    auto* orig_thunk =
        reinterpret_cast<PIMAGE_THUNK_DATA64>(base + imp->OriginalFirstThunk);
    auto* iat_thunk =
        reinterpret_cast<PIMAGE_THUNK_DATA64>(base + imp->FirstThunk);

    for (; orig_thunk->u1.AddressOfData; ++orig_thunk, ++iat_thunk) {
      if (IMAGE_SNAP_BY_ORDINAL64(orig_thunk->u1.Ordinal)) continue;

      auto* ibn = reinterpret_cast<PIMAGE_IMPORT_BY_NAME>(
          base + static_cast<SIZE_T>(orig_thunk->u1.AddressOfData));
      if (std::strcmp(reinterpret_cast<const char*>(ibn->Name), import_name) != 0)
        continue;

      *out_original = reinterpret_cast<void*>(iat_thunk->u1.Function);

      DWORD old_protect = 0;
      if (!VirtualProtect(iat_thunk, sizeof(*iat_thunk), PAGE_READWRITE, &old_protect))
        return -8;

      iat_thunk->u1.Function = reinterpret_cast<ULONGLONG>(detour);

      if (!VirtualProtect(iat_thunk, sizeof(*iat_thunk), old_protect, &old_protect))
        return -9;

      FlushInstructionCache(GetCurrentProcess(), iat_thunk, sizeof(*iat_thunk));
      return 0;
    }
  }
  return -10;
}
