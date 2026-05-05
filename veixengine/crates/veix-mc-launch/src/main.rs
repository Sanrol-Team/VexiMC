mod launch;

use anyhow::{bail, Context, Result};
use clap::Parser;
use launch::{current_os_ctx, gather_launch_command, load_version_json};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// 离线占位 token（足够长，满足新版客户端对会话字符串的常见校验；仅本地单机，不参与正版验证）。
const OFFLINE_ACCESS_TOKEN: &str =
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

#[derive(Parser, Debug)]
#[command(name = "veix-mc-launch", about = "VeixEngine：离线 / 正版启动 Minecraft 并注入 Veix Agent（默认离线）")]
struct Cli {
    /// 游戏目录（含版本 json、客户端 jar、log4j2.xml 等）
    #[arg(long, value_name = "DIR")]
    game_dir: PathBuf,

    /// 版本 id，对应 `{game_dir}/{id}.json`
    #[arg(long, default_value = "26.1.2")]
    version: String,

    /// Mojang 式 libraries 缓存根目录（默认 %USERPROFILE%\\.minecraft\\libraries）
    #[arg(long, value_name = "DIR")]
    libraries_dir: Option<PathBuf>,

    /// 资源目录（默认 %USERPROFILE%\\.minecraft\\assets）
    #[arg(long, value_name = "DIR")]
    assets_dir: Option<PathBuf>,

    /// java 可执行文件（默认 JAVA_HOME\\bin\\java.exe）
    #[arg(long, value_name = "EXE")]
    java: Option<PathBuf>,

    /// 离线用户名（对应 OfflinePlayer UUID）
    #[arg(short = 'u', long, default_value = "VeixDev")]
    username: String,

    /// 正版会话（需提供 `--access-token`，未实现 MS 登录流程时请保持默认离线）
    #[arg(long, default_value_t = false)]
    online: bool,

    /// 访问令牌（`--online` 时必填；离线时可省略，将使用内置占位串）
    #[arg(long)]
    access_token: Option<String>,

    /// 覆盖 UUID（默认按 Mojang 离线规则由用户名生成）
    #[arg(long)]
    uuid: Option<String>,

    /// 覆盖 `veix-agent.jar` 路径（默认与 exe 同目录）
    #[arg(long, value_name = "JAR")]
    veix_agent_jar: Option<PathBuf>,

    /// 仅打印将要执行的命令，不启动
    #[arg(long)]
    dry_run: bool,

    /// 为 Java Agent 打开「类加载 trace」（只打 log，不改写字节码；真 hook 需后续接 ASM）
    #[arg(long, default_value_t = false)]
    trace_classes: bool,

    /// trace 最大条数（防刷屏）
    #[arg(long, default_value_t = 400)]
    trace_max: u32,

    /// 只 trace 该前缀（内部名如 net.minecraft. → net/minecraft/）
    #[arg(long, default_value = "net.minecraft.")]
    trace_prefix: String,

    /// 关闭对 `Main#main` 的 ASM 字节码插桩（默认开启）
    #[arg(long, default_value_t = false)]
    no_asm_main_hook: bool,

    /// 启用 Veix 物品管线：读取 `gameDir/veix/veix_items.txt`，可选 JNI `veix_native.dll` 哈希与日志
    #[arg(long, default_value_t = false)]
    veix_items: bool,

    /// 启动时加载 `gameDir/veix/plugins/veix_test_mod.dll` 并执行 `veix_mod_plugin_boot`（需先编译 DEVSDK 示例）
    #[arg(long, default_value_t = false)]
    veix_test_mod: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let os_ctx = current_os_ctx();

    let version_json = cli.game_dir.join(format!("{}.json", cli.version));
    if !version_json.exists() {
        bail!(
            "未找到版本描述文件：{}（请确认 --version 与 json 文件名一致）",
            version_json.display()
        );
    }

    let json = load_version_json(&version_json)?;
    let java = resolve_java(&cli.java)?;
    let agent = resolve_agent_jar(cli.veix_agent_jar.as_ref())?;
    if !agent.exists() {
        bail!(
            "未找到 veix-agent.jar：{}。请安装 JDK、设置 JAVA_HOME 后执行：cargo build -p veix-mc-launch",
            agent.display()
        );
    }

    let libraries_root = resolve_libraries_dir(&cli)?;

    let assets_root = resolve_assets_dir(&cli);

    let token = resolve_access_token(cli.online, cli.access_token.as_deref())?;

    let javaagent_extras = build_javaagent_extras(
        !cli.no_asm_main_hook,
        cli.trace_classes,
        cli.trace_max,
        &cli.trace_prefix,
        cli.veix_items,
        cli.veix_test_mod,
    );

    let cmd = gather_launch_command(
        &json,
        java,
        agent,
        libraries_root,
        assets_root,
        cli.game_dir.clone(),
        &cli.username,
        &token,
        cli.uuid.as_deref(),
        &os_ctx,
        &javaagent_extras,
    )?;

    if cli.dry_run {
        println!("{}", shell_escape_windows(&cmd));
        return Ok(());
    }

    let exe = cmd
        .first()
        .cloned()
        .context("空命令")?;
    let rest = &cmd[1..];

    let mut child = Command::new(&exe)
        .args(rest)
        .current_dir(&cli.game_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("启动失败：{}", exe))?;

    let status = child.wait().context("等待游戏进程")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn resolve_java(explicit: &Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        if p.exists() {
            return Ok(p.clone());
        }
        bail!("指定的 java 不存在：{}", p.display());
    }
    if let Ok(h) = std::env::var("JAVA_HOME") {
        let p = PathBuf::from(&h).join("bin/java.exe");
        if p.exists() {
            return Ok(p);
        }
        let p = PathBuf::from(h).join("bin/java");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Ok(h) = std::env::var("VEIX_JAVA_HOME") {
        let p = PathBuf::from(&h).join("bin/java.exe");
        if p.exists() {
            return Ok(p);
        }
        let p = PathBuf::from(h).join("bin/java");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Some(p) = portable_java_next_to_workspace() {
        return Ok(p);
    }
    if let Some(p) = which_via_where("java.exe") {
        return Ok(p);
    }
    if let Some(p) = which_via_where("java") {
        return Ok(p);
    }
    bail!("未找到 java（请设置 JAVA_HOME 或将 java 加入 PATH，或使用 --java）");
}

/// `tools/download-build-deps.ps1` 解压的 Temurin，与 build.rs 查找逻辑一致。
fn portable_java_next_to_workspace() -> Option<PathBuf> {
    let jdk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/jdk");
    let win = jdk.join("bin/java.exe");
    if win.exists() {
        return Some(win);
    }
    let unix = jdk.join("bin/java");
    if unix.exists() {
        return Some(unix);
    }
    None
}

fn which_via_where(cmd: &str) -> Option<PathBuf> {
    let (prog, arg) = if cfg!(windows) {
        ("where", cmd)
    } else {
        ("which", cmd)
    };
    let out = Command::new(prog).arg(arg).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let first = line.lines().next()?.trim();
    let p = PathBuf::from(first);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn resolve_agent_jar(override_path: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p.clone());
    }
    if let Ok(p) = std::env::var("VEIX_AGENT_JAR") {
        return Ok(PathBuf::from(p));
    }
    let exe = std::env::current_exe().context("current_exe")?;
    let dir = exe.parent().context("exe parent")?;
    Ok(dir.join("veix-agent.jar"))
}

fn default_libraries_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".minecraft").join("libraries")
}

/// 常见布局：%USERPROFILE%\.minecraft\libraries、游戏目录旁 Gradle 缓存、portable `game_dir/libraries`。
fn resolve_libraries_dir(cli: &Cli) -> Result<PathBuf> {
    if let Some(ref p) = cli.libraries_dir {
        if p.exists() {
            return Ok(p.clone());
        }
        bail!("指定的 libraries 目录不存在：{}", p.display());
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.push(default_libraries_dir());
    candidates.push(cli.game_dir.join("libraries"));
    if let Some(root) = cli.game_dir.parent() {
        candidates.push(
            root.join("minecraftsrc")
                .join("app")
                .join("libraries"),
        );
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    let listed = candidates
        .iter()
        .map(|p| format!("  - {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "未找到依赖库目录。已尝试：\n{listed}\n请用 HMCL 下载依赖或传 --libraries-dir"
    );
}

fn default_assets_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".minecraft").join("assets")
}

fn resolve_assets_dir(cli: &Cli) -> PathBuf {
    if let Some(ref p) = cli.assets_dir {
        return p.clone();
    }
    let candidates = [
        default_assets_dir(),
        cli.game_dir.join("assets"),
        cli
            .game_dir
            .parent()
            .map(|p| p.join("assets"))
            .unwrap_or_default(),
    ];
    for c in candidates.iter() {
        if c.as_os_str().is_empty() {
            continue;
        }
        if c.exists() {
            return c.clone();
        }
    }
    default_assets_dir()
}

fn build_javaagent_extras(
    asm_main_hook: bool,
    trace: bool,
    max: u32,
    prefix: &str,
    veix_items: bool,
    veix_test_mod: bool,
) -> String {
    let mut s = String::new();
    if asm_main_hook {
        s.push_str(",asmMainHook=1");
    }
    if trace {
        let p = prefix.trim();
        s.push_str(&format!(",traceClasses=1,maxTrace={max},classPrefix={p}"));
    }
    if veix_items {
        s.push_str(",veixItems=1");
    }
    if veix_test_mod {
        s.push_str(",veixTestMod=1");
    }
    s
}

fn resolve_access_token(online: bool, explicit: Option<&str>) -> Result<String> {
    match (online, explicit) {
        (true, None) => bail!("正版模式请传入 --access-token（或使用离线：勿加 --online）"),
        (true, Some(t)) => Ok(t.to_string()),
        (false, Some(t)) => Ok(t.to_string()),
        (false, None) => Ok(OFFLINE_ACCESS_TOKEN.to_string()),
    }
}

fn shell_escape_windows(cmd: &[String]) -> String {
    cmd.iter()
        .map(|s| {
            if s.contains(' ') || s.contains('&') {
                format!("\"{}\"", s.replace('\"', "\\\""))
            } else {
                s.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
