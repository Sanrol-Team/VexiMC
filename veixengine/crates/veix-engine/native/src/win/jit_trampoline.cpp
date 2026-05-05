// JIT trampoline: RWX slot with mov rax, imm64 ; jmp rax.

#include <Windows.h>

#include <cstdint>
#include <cstring>

extern "C" void* veix_jit_alloc_trampoline_x64(void* target) {
  if (!target) return nullptr;

  constexpr SIZE_T kSize = 16;
  void* p = VirtualAlloc(nullptr, kSize, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
  if (!p) return nullptr;

  auto* c = static_cast<uint8_t*>(p);
  const uint64_t t = reinterpret_cast<uint64_t>(target);
  c[0] = 0x48;
  c[1] = 0xB8;
  std::memcpy(c + 2, &t, 8);
  c[10] = 0xFF;
  c[11] = 0xE0;

  DWORD old = 0;
  if (!VirtualProtect(p, kSize, PAGE_EXECUTE_READ, &old)) {
    VirtualFree(p, 0, MEM_RELEASE);
    return nullptr;
  }

  FlushInstructionCache(GetCurrentProcess(), p, kSize);
  return p;
}

extern "C" void veix_jit_free_slot(void* slot) {
  if (slot) VirtualFree(slot, 0, MEM_RELEASE);
}
