/**
 * Veix JNI：供 Java Agent 调用（物品 ID 哈希、原生侧追加日志）。
 * Windows MSVC：由 veix-mc-launch/build.rs 以 /LD 编成 veix_native.dll。
 */

#if defined(_MSC_VER)
#define _CRT_SECURE_NO_WARNINGS
#endif

#include <jni.h>

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#if defined(_WIN32)
#include <windows.h>
#endif

static uint32_t fnv1a32_bytes(const unsigned char *p) {
  uint32_t h = 2166136261u;
  while (*p) {
    h ^= (uint32_t)*p++;
    h *= 16777619u;
  }
  return h;
}

JNIEXPORT jstring JNICALL Java_veix_VeixNativeItems_nativeComputeItemKey(JNIEnv *env, jclass cls,
                                                                       jstring jId) {
  const char *utf = NULL;
  char buf[32];
  uint32_t h;
  (void)cls;

  if (!jId) {
    return NULL;
  }

  utf = (*env)->GetStringUTFChars(env, jId, NULL);
  if (!utf) {
    return NULL;
  }
  h = fnv1a32_bytes((const unsigned char *)utf);
  (*env)->ReleaseStringUTFChars(env, jId, utf);

#if defined(_MSC_VER)
  sprintf_s(buf, sizeof(buf), "%08x", (unsigned)h);
#else
  snprintf(buf, sizeof(buf), "%08x", (unsigned)h);
#endif
  return (*env)->NewStringUTF(env, buf);
}

JNIEXPORT jint JNICALL Java_veix_VeixNativeItems_nativeAppendItemsLog(JNIEnv *env, jclass cls,
                                                                    jstring jGameDir,
                                                                    jstring jLine) {
  char path[4096];
  const char *gd = NULL;
  const char *ln = NULL;
  FILE *fp = NULL;
  int ok = 0;
  (void)cls;

  if (!jGameDir || !jLine) {
    return -1;
  }

  gd = (*env)->GetStringUTFChars(env, jGameDir, NULL);
  ln = (*env)->GetStringUTFChars(env, jLine, NULL);
  if (!gd || !ln) {
    if (gd) {
      (*env)->ReleaseStringUTFChars(env, jGameDir, gd);
    }
    if (ln) {
      (*env)->ReleaseStringUTFChars(env, jLine, ln);
    }
    return -2;
  }

#if defined(_MSC_VER)
  sprintf_s(path, sizeof(path), "%s\\veix\\veix_native_items.log", gd);
#else
  snprintf(path, sizeof(path), "%s/veix/veix_native_items.log", gd);
#endif

  fp = fopen(path, "ab");
  if (fp) {
    fputs(ln, fp);
    fputc((int)'\n', fp);
    fflush(fp);
    fclose(fp);
    ok = 0;
  } else {
    ok = -3;
  }

  (*env)->ReleaseStringUTFChars(env, jGameDir, gd);
  (*env)->ReleaseStringUTFChars(env, jLine, ln);
  return ok;
}

#if defined(_WIN32)

typedef void (*veix_mod_plugin_boot_fn)(void);

JNIEXPORT jint JNICALL Java_veix_VeixPluginLoader_nativeLoadPluginBoot(JNIEnv *env, jclass cls,
                                                                       jstring absPath) {
  const jchar *wch = NULL;
  jsize n = 0;
  wchar_t stack[1024];
  wchar_t *wide = stack;
  HMODULE mod = NULL;
  veix_mod_plugin_boot_fn boot = NULL;
  DWORD gle = 0;
  (void)cls;

  if (!absPath) {
    return -1;
  }

  wch = (*env)->GetStringChars(env, absPath, NULL);
  if (!wch) {
    return -2;
  }
  n = (*env)->GetStringLength(env, absPath);

  if ((size_t)n + 1u > sizeof(stack) / sizeof(stack[0])) {
    wide = (wchar_t *)HeapAlloc(GetProcessHeap(), 0, ((size_t)n + 1u) * sizeof(wchar_t));
    if (!wide) {
      (*env)->ReleaseStringChars(env, absPath, wch);
      return -3;
    }
  }

  memcpy(wide, wch, (size_t)n * sizeof(jchar));
  wide[n] = (wchar_t)0;
  (*env)->ReleaseStringChars(env, absPath, wch);

  mod = LoadLibraryW(wide);
  if (wide != stack) {
    HeapFree(GetProcessHeap(), 0, wide);
    wide = NULL;
  }

  if (!mod) {
    gle = GetLastError();
    return gle ? -(int)gle : -5;
  }

  boot = (veix_mod_plugin_boot_fn)(void *)GetProcAddress(mod, "veix_mod_plugin_boot");
  if (!boot) {
    return -4;
  }

  boot();
  return 0;
}

#else

JNIEXPORT jint JNICALL Java_veix_VeixPluginLoader_nativeLoadPluginBoot(JNIEnv *env, jclass cls,
                                                                     jstring absPath) {
  (void)env;
  (void)cls;
  (void)absPath;
  return -50;
}

#endif
