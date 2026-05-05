package veix;

import java.io.File;

/**
 * Windows：通过 {@code veix_native.dll} 调用 {@code LoadLibraryW} 加载模组 DLL，并调用导出符号
 * {@code veix_mod_plugin_boot}（见 DEVSDK）。
 */
public final class VeixPluginLoader {

  private VeixPluginLoader() {}

  private static native int nativeLoadPluginBoot(String absolutePath);

  /**
   * 加载 {@code gameDir/veix/plugins/veix_test_mod.dll} 并执行 {@code veix_mod_plugin_boot}。
   * 须确保已加载 {@code veix_mc_bridge.dll}（与测试模组同目录或 PATH），否则模组内
   * {@code veix_mc_register_item} 会失败。
   *
   * @return 0 成功；负数为错误码（-4 未找到导出；-5 LoadLibrary 失败等）
   */
  public static int loadBundledTestPlugin(String gameDir) {
    String gd = gameDir != null ? gameDir : ".";
    VeixMcBridgeLoader.ensureLoaded(gd);
    VeixNativeItems.ensureLoaded(gd);
    File dll = new File(gd, "veix/plugins/veix_test_mod.dll");
    if (!dll.isFile()) {
      VeixAgent.appendLogLine(
          gd,
          "VeixPluginLoader: 未找到测试模组 DLL — " + dll.getAbsolutePath());
      return -1;
    }
    int rc = nativeLoadPluginBoot(dll.getAbsolutePath());
    if (rc != 0) {
      VeixAgent.appendLogLine(
          gd, "VeixPluginLoader: nativeLoadPluginBoot rc=" + rc + " path=" + dll.getAbsolutePath());
    } else {
      VeixAgent.appendLogLine(gd, "VeixPluginLoader: 已加载并执行 veix_mod_plugin_boot");
    }
    return rc;
  }
}
