package veix;

import java.io.File;
import java.net.URI;
import java.security.CodeSource;

/**
 * JNI 入口：加载 {@code veix_native.dll}（与 {@code veix-agent.jar} 同目录，或 {@code gameDir/veix/}）。
 * 缺失时由 {@link VeixItems} 使用纯 Java FNV-1a 回退。
 */
public final class VeixNativeItems {

  private VeixNativeItems() {}

  private static volatile boolean loaded;

  /** native：与 Java {@link VeixItems#fnv1a32Hex} 相同算法（UTF-8 / JNI modified UTF-8 对 ASCII 一致）。 */
  static native String nativeComputeItemKey(String id);

  /** native：追加一行到 {@code gameDir/veix/veix_native_items.log}；返回 0 成功。 */
  static native int nativeAppendItemsLog(String gameDir, String line);

  /**
   * 尝试加载 DLL；失败则保持 {@link #isLoaded()} false，不影响 Java 回退路径。
   */
  public static synchronized void ensureLoaded(String gameDir) {
    if (loaded) {
      return;
    }
    File dll = resolveDll(gameDir);
    if (dll == null || !dll.isFile()) {
      return;
    }
    try {
      System.load(dll.getAbsolutePath());
      loaded = true;
    } catch (UnsatisfiedLinkError e) {
      VeixAgent.appendLogLine(safeGameDir(gameDir), "VeixNativeItems: load failed: " + e.getMessage());
    }
  }

  public static boolean isLoaded() {
    return loaded;
  }

  private static String safeGameDir(String gameDir) {
    return gameDir != null ? gameDir : ".";
  }

  /**
   * 解析顺序：1) 与 Agent JAR 同目录的 {@code veix_native.dll}；2) {@code gameDir/veix/veix_native.dll}；
   * 3) 系统属性 {@code veix.native.lib} 完整路径。
   */
  private static File resolveDll(String gameDir) {
    String prop = System.getProperty("veix.native.lib");
    if (prop != null && !prop.isEmpty()) {
      File f = new File(prop);
      if (f.isFile()) {
        return f;
      }
    }
    try {
      CodeSource cs = VeixNativeItems.class.getProtectionDomain().getCodeSource();
      if (cs != null && cs.getLocation() != null) {
        URI uri = cs.getLocation().toURI();
        File jarOrDir = new File(uri);
        File parent = jarOrDir.getParentFile();
        if (parent != null) {
          File side = new File(parent, "veix_native.dll");
          if (side.isFile()) {
            return side;
          }
        }
      }
    } catch (Throwable ignored) {
      // fall through
    }
    if (gameDir != null) {
      File inVeix = new File(gameDir, "veix/veix_native.dll");
      if (inVeix.isFile()) {
        return inVeix;
      }
    }
    return null;
  }
}
