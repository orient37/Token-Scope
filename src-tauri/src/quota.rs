use crate::{ingest::hash, storage::Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Quota {
    pub status: String,
    pub collected_at: Option<String>,
    pub data: Option<Value>,
    pub message: Option<String>,
}
impl Quota {
    pub fn unavailable(message: &str) -> Self {
        Self {
            status: "unavailable".into(),
            collected_at: None,
            data: None,
            message: Some(message.into()),
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct Window {
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub duration_mins: Option<u64>,
    pub resets_at: Option<i64>,
}
pub fn window(v: &Value) -> Option<Window> {
    let used = v["usedPercent"].as_f64()?;
    if !used.is_finite() || used < 0.0 {
        return None;
    }
    Some(Window {
        used_percent: used,
        remaining_percent: (100.0 - used).clamp(0.0, 100.0),
        duration_mins: v["windowDurationMins"].as_u64(),
        resets_at: v["resetsAt"].as_i64(),
    })
}
pub fn project_payload(v: &Value) -> Value {
    let mut buckets = Vec::new();
    let fallback;
    let map = if let Some(m) = v["rateLimitsByLimitId"].as_object() {
        m
    } else {
        fallback = serde_json::Map::from_iter([("default".into(), v["rateLimits"].clone())]);
        &fallback
    };
    for (id, b) in map {
        if !b.is_object() {
            continue;
        }
        buckets.push(json!({"id":id,"name":b["limitName"],"plan":b["planType"],"primary":window(&b["primary"]),"secondary":window(&b["secondary"]),"credits":b["credits"],"spend_control_reached":b["spendControlReached"],"rate_limit_reached_type":b["rateLimitReachedType"]}));
    }
    json!({"buckets":buckets,"ordinary_usage_allowed":v["ordinaryUsageAllowed"],"account":v["accountId"].as_str().map(|s|hash(s).chars().take(24).collect::<String>())})
}
pub fn read(conn: &Connection) -> Quota {
    match request() {
        Ok(v) => {
            let account = v["accountId"]
                .as_str()
                .map(|s| hash(s).chars().take(24).collect::<String>())
                .unwrap_or_else(|| "unknown-live".into());
            let at = Utc::now().to_rfc3339();
            let result = (|| -> Result<()> {
                let tx = conn.unchecked_transaction()?;
                tx.execute(
                    "INSERT OR IGNORE INTO accounts VALUES(?1,'codex')",
                    [&account],
                )?;
                // Only quota-related fields, never auth, banners, prompts or account identity.
                let raw = json!({"rateLimits":v["rateLimits"],"rateLimitsByLimitId":v["rateLimitsByLimitId"],"ordinaryUsageAllowed":v["ordinaryUsageAllowed"]});
                tx.execute("INSERT INTO quota_snapshots(account_id,collected_at,source,payload) VALUES(?1,?2,'app-server',?3)",params![account,at,raw.to_string()])?;
                tx.commit()?;
                Ok(())
            })();
            if result.is_err() {
                return Quota::unavailable("额度已读取，但本地快照保存失败");
            };
            Quota {
                status: "synced".into(),
                collected_at: Some(at),
                data: Some(project_payload(&v)),
                message: None,
            }
        }
        Err(e) => Quota::unavailable(&e.to_string()),
    }
}
#[derive(Debug)]
pub enum QuotaError {
    NotFound,
    Startup { code: Option<i32>, reason: String },
    Timeout,
    Unauthorized,
    Unsupported,
    Rejected,
    Protocol,
}
impl QuotaError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::NotFound => "未找到 Codex 程序。请安装 Codex，或通过 TOKENSCOPE_CODEX_BIN 指定可执行文件后重启 TokenScope。",
            Self::Startup { .. } => "Codex App Server 启动失败。",
            Self::Timeout => "Codex 额度请求超时，请检查网络连接后重试。",
            Self::Unauthorized => "Codex 尚未登录或登录已失效，请在 Codex 中登录后重新同步。",
            Self::Unsupported => "当前 Codex 版本不支持额度接口，请更新 Codex 后重试。",
            Self::Rejected => "Codex 拒绝额度请求，请检查登录、网络与服务状态后重试。",
            Self::Protocol => "Codex App Server 连接中断或响应格式不兼容，请重新同步或更新 Codex。",
        }
    }
}
impl std::fmt::Display for QuotaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())?;
        if let Self::Startup { code, reason } = self {
            write!(f, " 系统启动错误：{reason}")?;
            if let Some(code) = code {
                write!(f, "（代码 {code}）")?;
            }
        }
        Ok(())
    }
}
impl std::error::Error for QuotaError {}

// Explicit configuration is authoritative; never silently bypass a broken override.
fn resolve_executable(
    override_path: Option<PathBuf>,
    search_path: &std::ffi::OsStr,
    local: Option<&Path>,
) -> Option<PathBuf> {
    // Windows shell launch can preserve a cleared variable as an empty value.
    if let Some(path) = override_path.filter(|p| !p.as_os_str().is_empty()) {
        return Some(path);
    }
    let name = if cfg!(windows) { "codex.exe" } else { "codex" };
    for dir in std::env::split_paths(search_path).filter(|p| !p.as_os_str().is_empty()) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    // Codex Desktop installs its CLI under versioned directories, absent from Explorer's PATH.
    let root = local?.join("OpenAI/Codex/bin");
    let direct = root.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let mut candidates: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path().join(name);
            let metadata = path.metadata().ok()?;
            metadata.is_file().then(|| (metadata.modified().ok(), path))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    candidates.into_iter().next().map(|(_, path)| path)
}
fn rpc_error(v: &Value) -> QuotaError {
    let message = v["error"]["message"]
        .as_str()
        .unwrap_or("")
        .to_ascii_lowercase();
    if v["error"]["code"].as_i64() == Some(-32601) {
        QuotaError::Unsupported
    } else if [
        "unauthorized",
        "not logged in",
        "not authenticated",
        "authentication",
        "login required",
        "401",
    ]
    .iter()
    .any(|s| message.contains(s))
    {
        QuotaError::Unauthorized
    } else {
        QuotaError::Rejected
    }
}
fn quota_result(v: &Value) -> std::result::Result<Value, QuotaError> {
    if v.get("error").is_some() {
        return Err(rpc_error(v));
    }
    v.get("result")
        .filter(|r| {
            r.is_object() && (r["rateLimits"].is_object() || r["rateLimitsByLimitId"].is_object())
        })
        .cloned()
        .ok_or(QuotaError::Protocol)
}
pub fn request() -> std::result::Result<Value, QuotaError> {
    let result = request_rpc("account/rateLimits/read", None)?;
    quota_result(&json!({"result":result}))
}
pub(crate) fn request_account() -> std::result::Result<Value, QuotaError> {
    request_rpc("account/read", Some(json!({"refreshToken":false})))
}
fn request_rpc(method: &str, params: Option<Value>) -> std::result::Result<Value, QuotaError> {
    let exe = resolve_executable(
        std::env::var_os("TOKENSCOPE_CODEX_BIN").map(PathBuf::from),
        &std::env::var_os("PATH").unwrap_or_default(),
        if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        } else {
            None
        }
        .as_deref(),
    )
    .ok_or(QuotaError::NotFound)?;
    let mut command = Command::new(exe);
    command
        .args([
            "app-server",
            "--listen",
            "stdio://",
            "-c",
            "analytics.enabled=false",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            QuotaError::NotFound
        } else {
            QuotaError::Startup {
                code: e.raw_os_error(),
                reason: e.to_string(),
            }
        }
    })?;
    let output = child.stdout.take().expect("piped stdout");
    let (send, recv) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let Ok(line) = line else { break };
            let Ok(v) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if send.send(v).is_err() {
                break;
            }
        }
    });
    let result = (|| -> std::result::Result<Value, QuotaError> {
        let input = child.stdin.as_mut().ok_or(QuotaError::Protocol)?;
        writeln!(
            input,
            "{}",
            json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"tokenscope","title":"TokenScope","version":"0.5.16"}}})
        ).map_err(|_| QuotaError::Protocol)?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let v = recv
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|e| match e {
                    mpsc::RecvTimeoutError::Timeout => QuotaError::Timeout,
                    mpsc::RecvTimeoutError::Disconnected => QuotaError::Protocol,
                })?;
            if v["id"] == 1 {
                if v.get("error").is_some() {
                    return Err(rpc_error(&v));
                }
                writeln!(input, "{}", json!({"method":"initialized"}))
                    .map_err(|_| QuotaError::Protocol)?;
                let mut request = json!({"id":2,"method":method});
                if let Some(params) = &params {
                    request["params"] = params.clone();
                }
                writeln!(input, "{}", request).map_err(|_| QuotaError::Protocol)?;
            }
            if v["id"] == 2 {
                if v.get("error").is_some() {
                    return Err(rpc_error(&v));
                }
                return v
                    .get("result")
                    .filter(|r| r.is_object())
                    .cloned()
                    .ok_or(QuotaError::Protocol);
            }
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_discovery_without_path_and_override_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "codex.exe" } else { "codex" };
        let install = dir.path().join("OpenAI/Codex/bin/version");
        std::fs::create_dir_all(&install).unwrap();
        let exe = install.join(name);
        std::fs::write(&exe, b"fixture").unwrap();
        assert_eq!(
            resolve_executable(None, std::ffi::OsStr::new(""), Some(dir.path())),
            Some(exe.clone())
        );
        assert_eq!(
            resolve_executable(
                Some(PathBuf::new()),
                std::ffi::OsStr::new(""),
                Some(dir.path())
            ),
            Some(exe)
        );
        let explicit = dir.path().join("missing.exe");
        assert_eq!(
            resolve_executable(
                Some(explicit.clone()),
                std::ffi::OsStr::new(""),
                Some(dir.path())
            ),
            Some(explicit)
        );
        let cli = dir.path().join(name);
        std::fs::write(&cli, b"fixture").unwrap();
        assert_eq!(
            resolve_executable(
                None,
                &std::env::join_paths([dir.path()]).unwrap(),
                Some(dir.path())
            ),
            Some(cli)
        );
        assert!(resolve_executable(None, std::ffi::OsStr::new(""), None).is_none());
    }
    #[test]
    fn rpc_failures_are_actionable_and_redacted() {
        let error =
            quota_result(&json!({"error":{"code":401,"message":"Unauthorized secret-token"}}))
                .unwrap_err();
        assert!(matches!(error, QuotaError::Unauthorized));
        assert!(!error.message().contains("secret-token"));
        assert!(matches!(
            quota_result(&json!({"error":{"code":-32601,"message":"private path"}})),
            Err(QuotaError::Unsupported)
        ));
        assert!(matches!(
            quota_result(&json!({"error":{"code":-1,"message":"private path"}})),
            Err(QuotaError::Rejected)
        ));
        assert!(matches!(
            quota_result(&json!({"result":null})),
            Err(QuotaError::Protocol)
        ));
        assert!(matches!(
            quota_result(&json!({"result":{}})),
            Err(QuotaError::Protocol)
        ));
        assert!(quota_result(&json!({"result":{"rateLimitsByLimitId":{}}})).is_ok());
    }
    #[test]
    fn missing_and_multi_pool() {
        assert!(window(&Value::Null).is_none());
        let v = json!({"rateLimitsByLimitId":{"a":{"primary":{"usedPercent":25,"windowDurationMins":300,"resetsAt":1791531961}},"b":{"secondary":{"usedPercent":110}}}});
        let p = project_payload(&v);
        assert_eq!(p["buckets"].as_array().unwrap().len(), 2);
        assert_eq!(p["buckets"][0]["primary"]["remaining_percent"], 75.0);
        assert_eq!(p["buckets"][1]["secondary"]["remaining_percent"], 0.0);
        assert!(p["buckets"][1]["primary"].is_null());
    }
    #[test]
    fn null_is_unknown() {
        assert!(window(&json!({"usedPercent":null})).is_none());
        assert!(window(&json!({"usedPercent":-1})).is_none());
    }
}
