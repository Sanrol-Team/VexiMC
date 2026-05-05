package veix;

import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.List;

/**
 * 贴近原版：{@code ResourceLocation.parse} → {@code Item.Properties} → {@code new Item} →
 * {@code Registry.register(BuiltInRegistries.ITEM, ResourceKey, Item)}（均为 Mojang 映射名）。
 *
 * <p>混淆客户端需替换类名或注入映射表。注册表可能在 Main 之前已冻结，失败时返回 {@code -14}。
 */
public final class VeixRegistryBridge {

  private VeixRegistryBridge() {}

  /** @param namespacedId 形如 {@code veix:ruby_gem} */
  public static int registerItemByKey(String namespacedId) {
    if (namespacedId == null || !namespacedId.contains(":")) {
      return -10;
    }
    try {
      Class<?> rlClass = Class.forName("net.minecraft.resources.ResourceLocation");
      Method parse = rlClass.getMethod("parse", String.class);
      Object rl = parse.invoke(null, namespacedId);

      Class<?> itemClass = Class.forName("net.minecraft.world.item.Item");
      Class<?> propsClass = Class.forName("net.minecraft.world.item.Item$Properties");
      Object props = propsClass.getDeclaredConstructor().newInstance();
      Constructor<?> itemCtor = itemClass.getDeclaredConstructor(propsClass);
      itemCtor.setAccessible(true);
      Object item = itemCtor.newInstance(props);

      Class<?> builtIn = Class.forName("net.minecraft.core.registries.BuiltInRegistries");
      Object itemRegistry = builtIn.getField("ITEM").get(null);

      Class<?> registries = Class.forName("net.minecraft.core.registries.Registries");
      Object registryNetworkKey = registries.getField("ITEM").get(null);

      Class<?> resourceKeyClass = Class.forName("net.minecraft.resources.ResourceKey");
      Method createKey = resourceKeyClass.getMethod("create", resourceKeyClass, rlClass);
      Object itemResourceKey = createKey.invoke(null, registryNetworkKey, rl);

      Class<?> registryApi = Class.forName("net.minecraft.core.Registry");
      Method register =
          findStaticThreeArgRegister(registryApi, itemRegistry, itemResourceKey, item);
      if (register != null) {
        try {
          register.invoke(null, itemRegistry, itemResourceKey, item);
          return 0;
        } catch (Throwable t) {
          String m = rootMsg(t);
          if (m != null && (m.contains("frozen") || m.contains("Frozen"))) {
            return -14;
          }
          return -14;
        }
      }
      return -13;
    } catch (ClassNotFoundException e) {
      return -11;
    } catch (Throwable t) {
      VeixAgent.appendLogLine(".", "VeixRegistryBridge: " + rootMsg(t));
      return -99;
    }
  }

  private static String rootMsg(Throwable t) {
    return t.getCause() != null ? t.getCause().getMessage() : t.getMessage();
  }

  /**
   * 选取 {@code static register(Registry, ResourceKey, Object)} 且第一参数可被 {@code itemRegistry}
   * 赋值的做法。
   */
  private static Method findStaticThreeArgRegister(
      Class<?> registryApi,
      Object itemRegistry,
      Object itemResourceKey,
      Object item)
      throws IllegalAccessException {
    for (Method m : registryApi.getDeclaredMethods()) {
      if (!Modifier.isStatic(m.getModifiers()) || !"register".equals(m.getName())) {
        continue;
      }
      if (m.getParameterCount() != 3) {
        continue;
      }
      Class<?>[] p = m.getParameterTypes();
      if (!p[0].isInstance(itemRegistry)) {
        continue;
      }
      if (!p[1].isInstance(itemResourceKey)) {
        continue;
      }
      if (!p[2].isInstance(item)) {
        continue;
      }
      m.setAccessible(true);
      return m;
    }
    for (Method m : registryApi.getMethods()) {
      if (!Modifier.isStatic(m.getModifiers()) || !"register".equals(m.getName())) {
        continue;
      }
      if (m.getParameterCount() != 3) {
        continue;
      }
      return m;
    }
    return null;
  }

  public static void registerAllFromIds(String gameDir, List<String> ids) {
    if (ids == null || ids.isEmpty()) {
      return;
    }
    String gd = gameDir != null ? gameDir : ".";
    for (String id : ids) {
      int rc = registerItemByKey(id);
      VeixAgent.appendLogLine(gd, "VeixRegistryBridge: register \"" + id + "\" rc=" + rc);
    }
  }
}
