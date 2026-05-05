package veix;

import java.io.File;
import java.net.URI;
import java.security.CodeSource;

/**
 * 加载 Rust 构建的 {@code veix_mc_bridge.dll}（JNI_OnLoad 缓存 JavaVM，供 C ABI 调用
 * {@code veix_mc_register_item}、{@code veix_mc_register_item_by_key}、{@code veix_mc_java_vm_ready}；
 * 见 {@code native/veix-mod-sdk/include/veix/mod_api.h}）。
 */
public final class VeixMcBridgeLoader {

  private VeixMcBridgeLoader() {}

  private static volatile boolean attempted;
  private static volatile boolean loaded;

  public static synchronized boolean ensureLoaded(String gameDir) {
    if (attempted) {
      return loaded;
    }
    attempted = true;
    File dll = resolveBridgeDll(gameDir);
    if (dll == null || !dll.isFile()) {
      VeixAgent.appendLogLine(
          safeGd(gameDir),
          "VeixMcBridge: 未找到 veix_mc_bridge.dll（与 veix-agent.jar 同目录或 gameDir/veix/）");
      return false;
    }
    try {
      System.load(dll.getAbsolutePath());
      loaded = true;
      VeixAgent.appendLogLine(safeGd(gameDir), "VeixMcBridge: 已加载 " + dll.getAbsolutePath());
    } catch (UnsatisfiedLinkError e) {
      VeixAgent.appendLogLine(safeGd(gameDir), "VeixMcBridge: load failed: " + e.getMessage());
    }
    return loaded;
  }

  public static boolean isLoaded() {
    return loaded;
  }

  private static String safeGd(String gameDir) {
    return gameDir != null ? gameDir : ".";
  }

  private static File resolveBridgeDll(String gameDir) {
    String prop = System.getProperty("veix.bridge.lib");
    if (prop != null && !prop.isEmpty()) {
      File f = new File(prop);
      if (f.isFile()) {
        return f;
      }
    }
    try {
      CodeSource cs = VeixMcBridgeLoader.class.getProtectionDomain().getCodeSource();
      if (cs != null && cs.getLocation() != null) {
        File jarOrDir = new File(cs.getLocation().toURI());
        File parent = jarOrDir.getParentFile();
        if (parent != null) {
          File side = new File(parent, "veix_mc_bridge.dll");
          if (side.isFile()) {
            return side;
          }
        }
      }
    } catch (Throwable ignored) {
      // continue
    }
    if (gameDir != null) {
      File inVeix = new File(gameDir, "veix/veix_mc_bridge.dll");
      if (inVeix.isFile()) {
        return inVeix;
      }
    }
    return null;
  }
}
