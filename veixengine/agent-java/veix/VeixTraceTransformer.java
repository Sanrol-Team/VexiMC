package veix;

import java.lang.instrument.ClassFileTransformer;
import java.security.ProtectionDomain;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * 仅观察：在类加载时打印 / 写日志，返回 null 表示不修改字节码。
 * 真正「游戏进行时」改写方法体需后续接入 ASM / Javassist（Build2+）。
 */
public final class VeixTraceTransformer implements ClassFileTransformer {

  private final String gameDir;
  private final String internalPrefix;
  private final int maxLogs;
  private final AtomicInteger seen = new AtomicInteger(0);

  public VeixTraceTransformer(String gameDir, String internalPrefix, int maxLogs) {
    this.gameDir = gameDir;
    String p = internalPrefix == null || internalPrefix.isEmpty()
        ? "net/minecraft/"
        : internalPrefix.replace('.', '/');
    if (!p.endsWith("/")) {
      p = p + "/";
    }
    this.internalPrefix = p;
    this.maxLogs = maxLogs <= 0 ? 400 : maxLogs;
  }

  @Override
  public byte[] transform(
      ClassLoader loader,
      String className,
      Class<?> classBeingRedefined,
      ProtectionDomain protectionDomain,
      byte[] classfileBuffer) {
    if (className == null || classfileBuffer == null) {
      return null;
    }
    if (seen.get() >= maxLogs) {
      return null;
    }
    if (!className.startsWith(internalPrefix)) {
      return null;
    }
    int n = seen.incrementAndGet();
    String dot = className.replace('/', '.');
    String msg = "[trace " + n + "/" + maxLogs + "] load " + dot + " (" + classfileBuffer.length + " B)";
    System.out.println("[VeixEngine] " + msg);
    VeixAgent.appendLogLine(gameDir, msg);
    return null;
  }
}
