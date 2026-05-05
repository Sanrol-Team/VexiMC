//! 编译 `agent-java`（依赖 ASM Maven）、解压 ASM 至 classes，打出含 `org.objectweb.asm` 的 fat `veix-agent.jar`。
//! Windows：若存在 `JAVA_HOME/include/jni.h`，将 `agent-java/native/veix_native.c` 编成 `veix_native.dll` 与 JAR 同发。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ASM_VERSION: &str = "9.7";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let agent_root = manifest_dir.join("../../agent-java");
    let veix_pkg = agent_root.join("veix");

    let mut java_sources: Vec<PathBuf> = fs::read_dir(&veix_pkg)
        .unwrap_or_else(|e| panic!("read agent-java/veix: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "java"))
        .collect();
    java_sources.sort();

    for p in &java_sources {
        println!("cargo:rerun-if-changed={}", p.display());
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let classes = out_dir.join("classes");
    let _ = fs::remove_dir_all(&classes);
    fs::create_dir_all(&classes).expect("create classes dir");

    let javac = find_javac().unwrap_or_else(|| {
        println!("cargo:warning=未找到 javac（请安装 JDK 并设置 JAVA_HOME）；跳过 Veix Agent JAR 生成");
        PathBuf::from("__missing_javac__")
    });

    if !javac.exists() || javac.to_string_lossy().contains("missing") {
        write_placeholder_notice(&out_dir);
        return;
    }

    if java_sources.is_empty() {
        panic!("agent-java/veix 下没有 .java 源文件");
    }

    let asm_jar = match ensure_asm_jar(&out_dir, &agent_root) {
        Ok(p) => p,
        Err(e) => {
            println!(
                "cargo:warning=无法获取 ASM {ASM_VERSION}（{e}）。可联网构建或先运行: veixengine/tools/download-build-deps.ps1"
            );
            write_placeholder_notice(&out_dir);
            return;
        }
    };

    println!("cargo:rerun-if-changed={}", asm_jar.display());

    let mut jc = Command::new(&javac);
    jc.arg("-encoding")
        .arg("UTF-8")
        .arg("-cp")
        .arg(asm_jar.as_path())
        .arg("-d")
        .arg(&classes);
    for p in &java_sources {
        jc.arg(p);
    }
    let status = jc.status().expect("javac");

    if !status.success() {
        panic!("javac 编译 agent-java 失败（需 ASM {} 在 classpath）", ASM_VERSION);
    }

    let jar_bin = match find_jar_tool() {
        Some(j) => j,
        None => {
            println!("cargo:warning=未找到 jar 工具，无法打包 veix-agent.jar");
            write_placeholder_notice(&out_dir);
            return;
        }
    };

    let xf = Command::new(&jar_bin)
        .current_dir(&classes)
        .arg("xf")
        .arg(&asm_jar)
        .status()
        .expect("jar xf asm");
    if !xf.success() {
        panic!("解压 ASM 到 classes 失败");
    }

    let jar_path = out_dir.join("veix-agent.jar");
    let manifest_path = out_dir.join("jar-manifest.txt");
    write_jar_manifest(&manifest_path);

    let status = Command::new(&jar_bin)
        .current_dir(&classes)
        .arg("cvfm")
        .arg(&jar_path)
        .arg(&manifest_path)
        .arg(".")
        .status()
        .expect("jar");
    if !status.success() {
        panic!("打包 veix-agent.jar 失败");
    }

    let dll_path = compile_veix_native_jni(&manifest_dir, &out_dir);
    copy_to_target_profile(&jar_path, dll_path.as_deref(), &manifest_dir);
}

#[cfg(windows)]
fn compile_veix_native_jni(manifest_dir: &Path, out_dir: &Path) -> Option<PathBuf> {
    let jdk = jdk_home_with_jni_header(manifest_dir)?;
    let src = manifest_dir.join("../../agent-java/native/veix_native.c");
    if !src.exists() {
        return None;
    }
    println!("cargo:rerun-if-changed={}", src.display());

    let out_dll = out_dir.join("veix_native.dll");
    let inc = jdk.join("include");
    let inc_win = jdk.join("include/win32");

    if try_clang_dll(&src, &inc, &inc_win, &out_dll, out_dir) {
        return Some(out_dll);
    }
    if try_msvc_cl_dll(&src, &inc, &inc_win, &out_dll, out_dir) {
        return Some(out_dll);
    }
    if try_mingw_gcc_dll(&src, &inc, &inc_win, &out_dll, out_dir) {
        return Some(out_dll);
    }

    println!(
        "cargo:warning=无法生成 veix_native.dll（需在 PATH 中找到 clang、cl 或 MinGW gcc，且 JAVA_HOME 含 jni.h）；Agent 仍可用 Java 回退哈希。"
    );
    None
}

#[cfg(windows)]
fn try_clang_dll(src: &Path, inc: &Path, inc_win: &Path, out_dll: &Path, work_dir: &Path) -> bool {
    let st = Command::new("clang")
        .current_dir(work_dir)
        .arg("-shared")
        .arg("-O2")
        .arg("-fvisibility=hidden")
        .arg("-D_CRT_SECURE_NO_WARNINGS")
        .arg(format!("-I{}", inc.display()))
        .arg(format!("-I{}", inc_win.display()))
        .arg("-o")
        .arg(out_dll.as_os_str())
        .arg(src.as_os_str())
        .status();
    matches!(st, Ok(s) if s.success()) && out_dll.is_file()
}

#[cfg(windows)]
fn try_msvc_cl_dll(src: &Path, inc: &Path, inc_win: &Path, out_dll: &Path, work_dir: &Path) -> bool {
    let st = Command::new("cl")
        .current_dir(work_dir)
        .arg("/nologo")
        .arg("/O2")
        .arg("/W3")
        .arg("/LD")
        .arg("/D_CRT_SECURE_NO_WARNINGS")
        .arg(format!("/I{}", inc.display()))
        .arg(format!("/I{}", inc_win.display()))
        .arg(format!("/Fe{}", out_dll.display()))
        .arg(src.as_os_str())
        .status();
    matches!(st, Ok(s) if s.success()) && out_dll.is_file()
}

#[cfg(windows)]
fn try_mingw_gcc_dll(src: &Path, inc: &Path, inc_win: &Path, out_dll: &Path, work_dir: &Path) -> bool {
    for gcc in ["gcc", "x86_64-w64-mingw32-gcc"] {
        let st = Command::new(gcc)
            .current_dir(work_dir)
            .arg("-shared")
            .arg("-O2")
            .arg("-D_CRT_SECURE_NO_WARNINGS")
            .arg(format!("-I{}", inc.display()))
            .arg(format!("-I{}", inc_win.display()))
            .arg("-o")
            .arg(out_dll.as_os_str())
            .arg(src.as_os_str())
            .status();
        if matches!(st, Ok(s) if s.success()) && out_dll.is_file() {
            return true;
        }
    }
    false
}

#[cfg(not(windows))]
fn compile_veix_native_jni(_manifest_dir: &Path, _out_dir: &Path) -> Option<PathBuf> {
    None
}

fn jdk_home_with_jni_header(manifest_dir: &Path) -> Option<PathBuf> {
    for var in ["JAVA_HOME", "VEIX_JAVA_HOME"] {
        if let Ok(v) = env::var(var) {
            let p = PathBuf::from(v);
            if p.join("include/jni.h").is_file() {
                return Some(p);
            }
        }
    }
    let portable = manifest_dir.join("../../tools/jdk");
    if portable.join("include/jni.h").is_file() {
        Some(portable)
    } else {
        None
    }
}

/// 顺序：`agent-java/deps/asm-*.jar`（预下载 / 脚本缓存）→ `OUT_DIR` 缓存 → Maven 联网拉取。
fn ensure_asm_jar(out_dir: &Path, agent_root: &Path) -> Result<PathBuf, String> {
    let name = format!("asm-{ASM_VERSION}.jar");
    let local = agent_root.join("deps").join(&name);
    if local.exists()
        && local
            .metadata()
            .map(|m| m.len() > 100_000)
            .unwrap_or(false)
    {
        println!("cargo:rerun-if-changed={}", local.display());
        return Ok(local);
    }

    let dest = out_dir.join(&name);
    if dest.exists()
        && dest
            .metadata()
            .map(|m| m.len() > 100_000)
            .unwrap_or(false)
    {
        return Ok(dest);
    }
    let url = format!(
        "https://repo1.maven.org/maven2/org/ow2/asm/asm/{ASM_VERSION}/asm-{ASM_VERSION}.jar"
    );
    download_url(&url, &dest)?;
    Ok(dest)
}

fn download_url(url: &str, dest: &Path) -> Result<(), String> {
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    let mut f = fs::File::create(dest).map_err(|e| e.to_string())?;
    std::io::copy(&mut reader, &mut f).map_err(|e| e.to_string())?;
    Ok(())
}

fn find_javac() -> Option<PathBuf> {
    if let Ok(home) = env::var("JAVA_HOME") {
        if let Some(p) = javac_in_jdk_home(Path::new(&home)) {
            return Some(p);
        }
    }
    if let Ok(extra) = env::var("VEIX_JAVA_HOME") {
        if let Some(p) = javac_in_jdk_home(Path::new(&extra)) {
            return Some(p);
        }
    }
    if let Ok(md) = env::var("CARGO_MANIFEST_DIR") {
        let portable = PathBuf::from(md).join("../../tools/jdk");
        if let Some(p) = javac_in_jdk_home(&portable) {
            println!("cargo:warning=使用便携 JDK: {}", portable.display());
            return Some(p);
        }
    }
    which_via_where("javac.exe").or_else(|| which_via_where("javac"))
}

fn javac_in_jdk_home(jdk_root: &Path) -> Option<PathBuf> {
    let win = jdk_root.join("bin/javac.exe");
    if win.exists() {
        return Some(win);
    }
    let unix = jdk_root.join("bin/javac");
    if unix.exists() {
        return Some(unix);
    }
    None
}

fn find_jar_tool() -> Option<PathBuf> {
    if let Ok(home) = env::var("JAVA_HOME") {
        if let Some(p) = jar_in_jdk_home(Path::new(&home)) {
            return Some(p);
        }
    }
    if let Ok(extra) = env::var("VEIX_JAVA_HOME") {
        if let Some(p) = jar_in_jdk_home(Path::new(&extra)) {
            return Some(p);
        }
    }
    if let Ok(md) = env::var("CARGO_MANIFEST_DIR") {
        let portable = PathBuf::from(md).join("../../tools/jdk");
        if let Some(p) = jar_in_jdk_home(&portable) {
            return Some(p);
        }
    }
    which_via_where("jar.exe").or_else(|| which_via_where("jar"))
}

fn jar_in_jdk_home(jdk_root: &Path) -> Option<PathBuf> {
    let win = jdk_root.join("bin/jar.exe");
    if win.exists() {
        return Some(win);
    }
    let unix = jdk_root.join("bin/jar");
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

fn write_jar_manifest(path: &Path) {
    fs::write(
        path,
        r"Manifest-Version: 1.0
Premain-Class: veix.VeixAgent
Agent-Class: veix.VeixAgent
Can-Redefine-Classes: true
Can-Retransform-Classes: true

",
    )
    .expect("write jar-manifest.txt");
}

fn write_placeholder_notice(out_dir: &Path) {
    let notice = out_dir.join("VEIX_AGENT_BUILD_SKIPPED.txt");
    let _ = fs::write(
        &notice,
        "Run tools/download-build-deps.ps1 (ASM + Temurin JDK), or set JAVA_HOME and place asm-9.7.jar under agent-java/deps/\n",
    );
}

fn cargo_target_root(manifest_dir: &Path) -> PathBuf {
    env::var("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        // crates/veix-mc-launch -> ../../../target（工作区根 F:\VeixEngine\26）
        manifest_dir.join("../../../target")
    })
}

fn copy_to_target_profile(jar: &Path, native_dll: Option<&Path>, manifest_dir: &Path) {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let dest_dir = cargo_target_root(manifest_dir).join(&profile);
    let _ = fs::create_dir_all(&dest_dir);
    let dest = dest_dir.join("veix-agent.jar");
    if let Err(e) = fs::copy(jar, &dest) {
        println!("cargo:warning=无法复制 veix-agent.jar 到 {}: {}", dest.display(), e);
    }
    if let Some(dll) = native_dll {
        let dest_dll = dest_dir.join("veix_native.dll");
        if let Err(e) = fs::copy(dll, &dest_dll) {
            println!(
                "cargo:warning=无法复制 veix_native.dll 到 {}: {}",
                dest_dll.display(),
                e
            );
        }
    }
    // veix_mc_bridge.dll 由 crate veix-mc-bridge 直接写入 workspace target/<profile>/ ，与 dest_dir 相同，毋须复制。
}
