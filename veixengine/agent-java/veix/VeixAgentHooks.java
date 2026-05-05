package veix;

/**
 * 由 ASM 在目标方法开头插入 {@code INVOKESTATIC} 跳转的回调（纯 Java，便于读栈与写日志）。
 */
public final class VeixAgentHooks {

  private static volatile String gameDir = ".";

  private VeixAgentHooks() {}

  public static void setGameDir(String dir) {
    gameDir = dir != null ? dir : ".";
  }

  public static String getGameDir() {
    return gameDir;
  }

  /**
   * 插在 {@code net.minecraft.client.main.Main#main(String[])} 的 visitCode 最前：
   * 等价字节码 {@code INVOKESTATIC veix/VeixAgentHooks.onMinecraftMain ()V}。
   */
  public static void onMinecraftMain() {
    String msg = "[ASM] Main.main entry (INVOKESTATIC veix.VeixAgentHooks.onMinecraftMain)";
    System.out.println("[VeixEngine] " + msg);
    VeixAgent.appendLogLine(gameDir, msg);
    VeixItems.onMainEntry(gameDir);
  }
}
