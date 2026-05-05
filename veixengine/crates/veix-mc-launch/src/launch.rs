//! 版本 JSON 解析、规则判定（对齐 Prism / Mojang 语义子集）与参数展开。

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct OsCtx {
    pub family: &'static str,
    pub pointer_width: u32,
}

pub fn current_os_ctx() -> OsCtx {
    let family = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    };
    OsCtx {
        family,
        pointer_width: u32::try_from(usize::BITS).unwrap_or(64),
    }
}

/// 与 Prism `Library::isActive` 同构：无规则则允许；否则自 Disallow 起，取最后一条非 Defer 规则。
pub fn library_rules_allow(rules: &[Value], ctx: &OsCtx) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut rule_result = false;
    for rule in rules {
        if let Some(allow) = apply_rule(rule, ctx) {
            rule_result = allow;
        }
    }
    rule_result
}

fn apply_rule(rule: &Value, ctx: &OsCtx) -> Option<bool> {
    if let Some(features) = rule.get("features") {
        if let Some(m) = features.as_object() {
            if !m.is_empty() {
                return None;
            }
        }
    }
    if let Some(os) = rule.get("os") {
        if !classifier_matches(os, ctx) {
            return None;
        }
    }
    let action = rule.get("action")?.as_str()?;
    Some(action == "allow")
}

fn classifier_matches(os: &Value, ctx: &OsCtx) -> bool {
    let name = os.get("name").and_then(|v| v.as_str()).unwrap_or("");
    match name {
        "windows" => ctx.family == "windows",
        "linux" => ctx.family == "linux",
        "osx" => ctx.family == "macos",
        "universal" => {
            if let Some(arch) = os.get("arch").and_then(|v| v.as_str()) {
                if arch == "x86" {
                    return ctx.pointer_width == 32;
                }
            }
            true
        }
        _ => false,
    }
}

pub struct LaunchSubst {
    pub classpath: String,
    pub natives_directory: PathBuf,
    pub launcher_name: String,
    pub launcher_version: String,
    pub assets_root: PathBuf,
    pub game_directory: PathBuf,
    pub assets_index_name: String,
    pub auth_player_name: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    pub clientid: String,
    pub auth_xuid: String,
    pub version_name: String,
    pub version_type: String,
    pub log_config_path: String,
    pub resolution_width: String,
    pub resolution_height: String,
    pub quick_play_path: String,
    pub quick_play_singleplayer: String,
    pub quick_play_multiplayer: String,
    pub quick_play_realms: String,
}

impl LaunchSubst {
    pub fn expand(&self, s: &str) -> String {
        let mut out = s.to_string();
        let map: [(&str, &str); 21] = [
            ("${classpath}", &self.classpath),
            (
                "${natives_directory}",
                &self.natives_directory.to_string_lossy(),
            ),
            ("${launcher_name}", &self.launcher_name),
            ("${launcher_version}", &self.launcher_version),
            ("${assets_root}", &self.assets_root.to_string_lossy()),
            ("${game_directory}", &self.game_directory.to_string_lossy()),
            ("${assets_index_name}", &self.assets_index_name),
            ("${auth_player_name}", &self.auth_player_name),
            ("${auth_uuid}", &self.auth_uuid),
            ("${auth_access_token}", &self.auth_access_token),
            ("${clientid}", &self.clientid),
            ("${auth_xuid}", &self.auth_xuid),
            ("${version_name}", &self.version_name),
            ("${version_type}", &self.version_type),
            ("${path}", &self.log_config_path),
            ("${resolution_width}", &self.resolution_width),
            ("${resolution_height}", &self.resolution_height),
            ("${quickPlayPath}", &self.quick_play_path),
            (
                "${quickPlaySingleplayer}",
                &self.quick_play_singleplayer,
            ),
            (
                "${quickPlayMultiplayer}",
                &self.quick_play_multiplayer,
            ),
            ("${quickPlayRealms}", &self.quick_play_realms),
        ];
        for (k, v) in map {
            out = out.replace(k, v);
        }
        out
    }
}

pub fn collect_classpath(
    libraries: &[Value],
    libraries_root: &Path,
    game_dir: &Path,
    jar_id: &str,
    ctx: &OsCtx,
) -> Result<String> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut parts: Vec<String> = Vec::new();

    for lib in libraries {
        let rules = lib
            .get("rules")
            .and_then(|r| r.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        if !library_rules_allow(rules, ctx) {
            continue;
        }
        let Some(dl) = lib.get("downloads") else { continue };
        let Some(artifact) = dl.get("artifact") else { continue };
        let Some(rel) = artifact.get("path").and_then(|p| p.as_str()) else {
            continue;
        };
        let full = libraries_root.join(rel);
        if !full.exists() {
            anyhow::bail!(
                "缺少依赖库（请先通过官方启动器或 HMCL 下载依赖）：{}",
                full.display()
            );
        }
        parts.push(full.to_string_lossy().into_owned());
    }

    let client_jar = game_dir.join(format!("{}.jar", jar_id));
    if !client_jar.exists() {
        anyhow::bail!("未找到客户端 jar：{}", client_jar.display());
    }
    parts.push(client_jar.to_string_lossy().into_owned());

    Ok(parts.join(sep))
}

pub fn expand_arg_list(
    raw: &[Value],
    ctx: &OsCtx,
    subst: &LaunchSubst,
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in raw {
        match entry {
            Value::String(s) => out.push(subst.expand(s)),
            Value::Object(obj) => {
                let rules = obj
                    .get("rules")
                    .and_then(|r| r.as_array())
                    .map(|a| a.as_slice())
                    .unwrap_or(&[]);
                if !library_rules_allow(rules, ctx) {
                    continue;
                }
                let Some(val) = obj.get("value") else { continue };
                match val {
                    Value::String(s) => out.push(subst.expand(s)),
                    Value::Array(arr) => {
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                out.push(subst.expand(s));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

pub fn load_version_json(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("读取版本 JSON：{}", path.display()))?;
    let v: Value = serde_json::from_str(&text).context("解析 JSON")?;
    Ok(v)
}

pub fn pick_natives_dir(game_dir: &Path) -> PathBuf {
    for name in [
        "natives-windows-x86_64",
        "natives",
        "natives-windows",
    ] {
        let p = game_dir.join(name);
        if p.exists() && p.is_dir() {
            return p;
        }
    }
    game_dir.join("natives-windows-x86_64")
}

/// 简易离线 UUID（Minecraft 惯例）。
pub fn offline_uuid_string(username: &str) -> String {
    let d = md5::compute(format!("OfflinePlayer:{}", username).as_bytes());
    let mut b: [u8; 16] = d.into();
    b[6] = (b[6] & 0x0f) | 0x30;
    b[8] = (b[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(b).hyphenated().to_string()
}

pub fn build_subst(
    json: &Value,
    cp: String,
    natives: PathBuf,
    game_dir: PathBuf,
    assets_root: PathBuf,
    auth_name: &str,
    token: &str,
    uuid_override: Option<&str>,
    log4j: Option<PathBuf>,
) -> Result<LaunchSubst> {
    let id = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("版本 JSON 缺少 id"))?;
    let assets = json
        .get("assets")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let index_name = json
        .get("assetIndex")
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or(assets);
    let vtype = json
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("release");
    let log_path = log4j
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let auth_uuid = if let Some(u) = uuid_override {
        u.trim().to_string()
    } else {
        offline_uuid_string(auth_name)
    };

    Ok(LaunchSubst {
        classpath: cp,
        natives_directory: natives,
        launcher_name: "veix".into(),
        launcher_version: env!("CARGO_PKG_VERSION").into(),
        assets_root,
        game_directory: game_dir,
        assets_index_name: index_name.into(),
        auth_player_name: auth_name.into(),
        auth_uuid,
        auth_access_token: token.into(),
        clientid: String::new(),
        auth_xuid: String::new(),
        version_name: id.into(),
        version_type: vtype.into(),
        log_config_path: log_path,
        resolution_width: "854".into(),
        resolution_height: "480".into(),
        quick_play_path: String::new(),
        quick_play_singleplayer: String::new(),
        quick_play_multiplayer: String::new(),
        quick_play_realms: String::new(),
    })
}

/// `javaagent_extras` 接在 `gameDir=...` 后，须以 `,` 开头，例如 `,traceClasses=1,maxTrace=200`。
pub fn gather_launch_command(
    json: &Value,
    java: PathBuf,
    agent_jar: PathBuf,
    libraries_root: PathBuf,
    assets_root: PathBuf,
    game_dir: PathBuf,
    username: &str,
    access_token: &str,
    uuid_override: Option<&str>,
    ctx: &OsCtx,
    javaagent_extras: &str,
) -> Result<Vec<String>> {
    let libraries = json
        .get("libraries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("版本 JSON 缺少 libraries"))?;
    let jar_id = json
        .get("jar")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("id").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow!("无法确定客户端 jar id"))?;

    let natives = pick_natives_dir(&game_dir);
    if !natives.exists() {
        anyhow::bail!(
            "未找到原生库目录（请先运行一次官方启动器下载 natives，或指定已解压目录）：{}",
            natives.display()
        );
    }

    let cp = collect_classpath(libraries, &libraries_root, &game_dir, jar_id, ctx)?;
    let log4j = game_dir.join("log4j2.xml");
    let log4j = if log4j.exists() {
        Some(log4j)
    } else {
        None
    };
    let subst = build_subst(
        json,
        cp,
        natives.clone(),
        game_dir.clone(),
        assets_root.clone(),
        username,
        access_token,
        uuid_override,
        log4j.clone(),
    )?;

    let args_obj = json
        .get("arguments")
        .ok_or_else(|| anyhow!("版本 JSON 缺少 arguments"))?;
    let jvm_raw = args_obj
        .get("jvm")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("arguments.jvm 缺失"))?;
    let game_raw = args_obj
        .get("game")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("arguments.game 缺失"))?;

    let mut jvm = expand_arg_list(jvm_raw, ctx, &subst)?;
    if subst.log_config_path.is_empty() {
        jvm.retain(|s| !s.trim_start().starts_with("-Dlog4j.configurationFile"));
    }
    let game = expand_arg_list(game_raw, ctx, &subst)?;

    let game_dir_utf = game_dir.to_string_lossy().to_string();
    let agent_param = if javaagent_extras.is_empty() {
        format!(
            "-javaagent:{}=gameDir={}",
            agent_jar.to_string_lossy(),
            game_dir_utf
        )
    } else {
        format!(
            "-javaagent:{}=gameDir={}{}",
            agent_jar.to_string_lossy(),
            game_dir_utf,
            javaagent_extras
        )
    };
    jvm.insert(0, agent_param);

    let main_class = json
        .get("mainClass")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("缺少 mainClass"))?;

    let mut cmd: Vec<String> = vec![
        java.to_string_lossy().into_owned(),
        "-Xmx2G".into(),
    ];
    cmd.extend(jvm);
    cmd.push(main_class.into());
    cmd.extend(game);

    Ok(cmd)
}
