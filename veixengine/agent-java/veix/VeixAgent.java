package veix;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.lang.instrument.Instrumentation;
import java.time.Instant;

/**
 * Java Agent：JVM 启动极早阶段注入，用于 VeixEngine 与 MC 进程对接（Build1 / Windows 优先）。
 *
 * <p>参数（逗号分隔）：{@code gameDir=...}；{@code asmMainHook=1} 时用 ASM 改写
 * {@code net.minecraft.client.main.Main#main}；可选 {@code traceClasses=1}、{@code maxTrace}、
 * {@code classPrefix}；{@code veixItems=1} 时读取 {@code gameDir/veix/veix_items.txt}，加载可选
 * {@code veix_native.dll}，以及 Rust 产物 {@code veix_mc_bridge.dll}（JNI → {@link VeixRegistryBridge} 物品注册）；
 * {@code veixTestMod=1} 时加载 {@code gameDir/veix/plugins/veix_test_mod.dll} 并调用 {@code veix_mod_plugin_boot}。
 */
public final class VeixAgent {
  private VeixAgent() {}

  public static void premain(String agentArgs, Instrumentation inst) {
    AgentOptions opts = AgentOptions.parse(agentArgs);
    VeixAgentHooks.setGameDir(opts.gameDir);

    String msg = "Hello Veix";
    System.out.println("[VeixEngine] " + msg);
    logToFile(opts.gameDir, msg, null);

    if (opts.asmMainHook) {
      inst.addTransformer(new VeixAsmMainTransformer(), true);
      String amsg =
          "asmMainHook: ASM will patch net.minecraft.client.main.Main#main (INVOKESTATIC hook)";
      System.out.println("[VeixEngine] " + amsg);
      appendLogLine(opts.gameDir, amsg);
    }

    if (opts.traceClasses) {
      VeixTraceTransformer tr =
          new VeixTraceTransformer(opts.gameDir, opts.classPrefix, opts.maxTrace);
      inst.addTransformer(tr, true);
      String tmsg =
          "traceClasses enabled (maxTrace="
              + opts.maxTrace
              + ", prefix="
              + opts.classPrefix
              + ") — log only, no bytecode rewrite";
      System.out.println("[VeixEngine] " + tmsg);
      appendLogLine(opts.gameDir, tmsg);
    }

    if (opts.veixItems) {
      VeixItems.bootstrap(opts.gameDir, true);
      String imsg = "veixItems: 已启用（veix/veix_items.txt + veix_native.dll 可选）";
      System.out.println("[VeixEngine] " + imsg);
      appendLogLine(opts.gameDir, imsg);
    }

    if (opts.veixTestMod) {
      VeixMcBridgeLoader.ensureLoaded(opts.gameDir);
      VeixNativeItems.ensureLoaded(opts.gameDir);
      int prc = VeixPluginLoader.loadBundledTestPlugin(opts.gameDir);
      String pmsg =
          "veixTestMod: 加载 gameDir/veix/plugins/veix_test_mod.dll（rc=" + prc + "）";
      System.out.println("[VeixEngine] " + pmsg);
      appendLogLine(opts.gameDir, pmsg);
    }
  }

  @SuppressWarnings("unused")
  public static void agentmain(String agentArgs, Instrumentation inst) {
    premain(agentArgs, inst);
  }

  /** 供 Transformer 与其它模块追加一行日志（同步写文件，避免启动阶段乱序）。 */
  public static synchronized void appendLogLine(String gameDir, String line) {
    logToFile(gameDir, line, null);
  }

  private static final class AgentOptions {
    String gameDir = ".";
    /** ASM 改写 Main#main 入口 */
    boolean asmMainHook = false;
    boolean traceClasses = false;
    int maxTrace = 400;
    /** 外部写法 {@code net.minecraft.} 或 {@code net/minecraft/} */
    String classPrefix = "net.minecraft.";
    /** 读取 {@code gameDir/veix/veix_items.txt}，JNI / Java 物品键 */
    boolean veixItems = false;
    /** 加载 {@code veix/plugins/veix_test_mod.dll} 并调用 {@code veix_mod_plugin_boot} */
    boolean veixTestMod = false;

    static AgentOptions parse(String agentArgs) {
      AgentOptions o = new AgentOptions();
      if (agentArgs == null || agentArgs.isEmpty()) {
        return o;
      }
      for (String part : agentArgs.split(",")) {
        int eq = part.indexOf('=');
        if (eq <= 0) {
          continue;
        }
        String key = part.substring(0, eq).trim();
        String val = part.substring(eq + 1);
        switch (key) {
          case "gameDir":
            o.gameDir = val;
            break;
          case "asmMainHook":
            o.asmMainHook = truthy(val);
            break;
          case "traceClasses":
            o.traceClasses = truthy(val);
            break;
          case "maxTrace":
            try {
              o.maxTrace = Integer.parseInt(val.trim());
            } catch (NumberFormatException ignored) {
              o.maxTrace = 400;
            }
            break;
          case "classPrefix":
            o.classPrefix = val;
            break;
          case "veixItems":
            o.veixItems = truthy(val);
            break;
          case "veixTestMod":
            o.veixTestMod = truthy(val);
            break;
          default:
            break;
        }
      }
      return o;
    }

    private static boolean truthy(String s) {
      if (s == null) {
        return false;
      }
      String t = s.trim().toLowerCase();
      return t.equals("1")
          || t.equals("true")
          || t.equals("yes")
          || t.equals("on");
    }
  }

  private static void logToFile(String gameDir, String msg, Throwable error) {
    try {
      File base = new File(gameDir);
      File logFile = new File(base, "logs/veix.log");
      File parent = logFile.getParentFile();
      if (parent != null && !parent.exists() && !parent.mkdirs()) {
        System.err.println("[VeixEngine] failed to mkdir logs under " + base.getAbsolutePath());
      }
      try (FileWriter w = new FileWriter(logFile, true)) {
        w.write(Instant.now().toString());
        w.write(' ');
        w.write(msg);
        w.write(System.lineSeparator());
        if (error != null) {
          StringWriter sw = new StringWriter();
          error.printStackTrace(new PrintWriter(sw));
          w.write(sw.toString());
          w.write(System.lineSeparator());
        }
      }
    } catch (IOException e) {
      e.printStackTrace();
    }
  }
}
