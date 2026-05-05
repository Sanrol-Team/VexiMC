package veix;

import java.lang.instrument.ClassFileTransformer;
import java.security.ProtectionDomain;

import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassVisitor;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.MethodVisitor;
import org.objectweb.asm.Opcodes;

/**
 * 用 ASM 改写 {@link net.minecraft.client.main.Main#main}：在方法体最前插入对
 * {@link VeixAgentHooks#onMinecraftMain()} 的静态调用（JVM 栈机字节码，非 x86 汇编）。
 */
public final class VeixAsmMainTransformer implements ClassFileTransformer {

  private static final String TARGET_INTERNAL = "net/minecraft/client/main/Main";
  private static final int API = Opcodes.ASM9;

  @Override
  public byte[] transform(
      ClassLoader loader,
      String className,
      Class<?> classBeingRedefined,
      ProtectionDomain protectionDomain,
      byte[] classfileBuffer) {
    if (className == null || !TARGET_INTERNAL.equals(className)) {
      return null;
    }
    try {
      ClassReader cr = new ClassReader(classfileBuffer);
      ClassWriter cw = new ClassWriter(cr, ClassWriter.COMPUTE_MAXS);
      ClassVisitor cv =
          new ClassVisitor(API, cw) {
            @Override
            public MethodVisitor visitMethod(
                int access,
                String name,
                String descriptor,
                String signature,
                String[] exceptions) {
              MethodVisitor mv =
                  super.visitMethod(access, name, descriptor, signature, exceptions);
              if ("main".equals(name) && "([Ljava/lang/String;)V".equals(descriptor)) {
                return new MethodVisitor(API, mv) {
                  @Override
                  public void visitCode() {
                    visitMethodInsn(
                        Opcodes.INVOKESTATIC,
                        "veix/VeixAgentHooks",
                        "onMinecraftMain",
                        "()V",
                        false);
                    super.visitCode();
                  }
                };
              }
              return mv;
            }
          };
      cr.accept(cv, ClassReader.EXPAND_FRAMES);
      return cw.toByteArray();
    } catch (Throwable t) {
      t.printStackTrace();
      return null;
    }
  }
}
