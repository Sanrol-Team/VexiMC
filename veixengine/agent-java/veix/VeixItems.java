package veix;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;

/**
 * 物品定义管线：从 {@code gameDir/veix/veix_items.txt} 读取（每行一个 {@code namespace:path}），
 * 经 JNI（若可用）或 Java FNV 计算稳定短键，并写日志。
 *
 * <p><b>说明</b>：向 Minecraft 正式注册新物品需要对应版本的 Registry / 数据组件 API，通常依赖 Fabric
 * 或数据包。本模块提供 Agent 侧「定义 + 哈希 + 晚期反射探测」；真正进游戏注册需按你的 MC 版本补全。
 */
public final class VeixItems {

  private VeixItems() {}

  private static volatile List<String> ids = Collections.emptyList();
  private static volatile boolean bootstrapped;

  public static void bootstrap(String gameDir, boolean enabled) {
    if (!enabled) {
      return;
    }
    bootstrapped = true;
    VeixNativeItems.ensureLoaded(gameDir);
    VeixMcBridgeLoader.ensureLoaded(gameDir);
    loadDefinitions(gameDir);
  }

  public static void onMainEntry(String gameDir) {
    if (!bootstrapped) {
      return;
    }
    tryLateBind(gameDir);
  }

  static void loadDefinitions(String gameDir) {
    if (gameDir == null) {
      gameDir = ".";
    }
    File base = new File(gameDir, "veix");
    if (!base.isDirectory()) {
      base.mkdirs();
    }
    File def = new File(base, "veix_items.txt");
    if (!def.isFile()) {
      VeixAgent.appendLogLine(
          gameDir,
          "VeixItems: 未找到 "
              + def.getAbsolutePath()
              + "（可新建：每行一个物品 id，如 veix:ruby_gem）");
      return;
    }

    List<String> list = new ArrayList<>();
    try (BufferedReader br =
        new BufferedReader(
            new InputStreamReader(new FileInputStream(def), StandardCharsets.UTF_8))) {
      String line;
      while ((line = br.readLine()) != null) {
        line = line.trim();
        if (line.isEmpty() || line.startsWith("#")) {
          continue;
        }
        list.add(line);
      }
    } catch (IOException e) {
      VeixAgent.appendLogLine(gameDir, "VeixItems: 读取失败 " + e.getMessage());
      return;
    }

    ids = Collections.unmodifiableList(list);
    StringBuilder summary = new StringBuilder();
    summary.append("VeixItems: 已加载 ").append(ids.size()).append(" 条定义 — ");
    for (String id : ids) {
      String key = computeItemKeyHex(gameDir, id);
      summary.append('[').append(id).append("→").append(key).append("] ");
      String logLine = "item def id=" + id + " key=" + key + " native=" + VeixNativeItems.isLoaded();
      if (VeixNativeItems.isLoaded()) {
        VeixNativeItems.nativeAppendItemsLog(gameDir, logLine);
      }
    }
    VeixAgent.appendLogLine(gameDir, summary.toString().trim());
  }

  static String computeItemKeyHex(String gameDir, String id) {
    if (VeixNativeItems.isLoaded()) {
      try {
        return VeixNativeItems.nativeComputeItemKey(id);
      } catch (UnsatisfiedLinkError e) {
        // fall through
      }
    }
    return fnv1a32Hex(id);
  }

  /**
   * 与 {@code veix_native.c} 中 UTF-8 字节序列上的 FNV-1a 一致（适用于 ASCII / BMP 常用字符）。
   */
  static String fnv1a32Hex(String s) {
    byte[] b = s.getBytes(StandardCharsets.UTF_8);
    int h = (int) 2166136261L;
    for (byte x : b) {
      h ^= (x & 0xff);
      h = (int) ((h * 16777619L) & 0xffffffffL);
    }
    return String.format(Locale.ROOT, "%08x", h);
  }

  public static List<String> getRegisteredIds() {
    return ids;
  }

  private static void tryLateBind(String gameDir) {
    if (ids.isEmpty()) {
      return;
    }
    VeixMcBridgeLoader.ensureLoaded(gameDir);
    String note =
        "VeixItems: 晚期绑定 — 共 "
            + ids.size()
            + " 条 id；尝试经 VeixRegistryBridge 注册（失败多为注册表已冻结，需更早钩子或模组加载器）。";
    VeixAgent.appendLogLine(gameDir, note);
    VeixRegistryBridge.registerAllFromIds(gameDir, ids);
  }
}
