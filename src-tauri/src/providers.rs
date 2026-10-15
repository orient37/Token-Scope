use serde::Serialize;
use std::path::PathBuf;
#[derive(Serialize)]
pub struct Capabilities {
    pub discover_sessions: bool,
    pub import_usage: bool,
    pub watch_usage: bool,
    pub read_quota: bool,
    pub health_check: bool,
}
#[derive(Serialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub capabilities: Capabilities,
}
#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    Unsupported(&'static str),
    Unavailable(String),
    Failed(String),
}
impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(c) => write!(f, "capability unsupported: {c}"),
            Self::Unavailable(s) => write!(f, "unavailable: {s}"),
            Self::Failed(s) => write!(f, "failed: {s}"),
        }
    }
}
impl std::error::Error for AdapterError {}
#[derive(Debug, Serialize)]
pub struct ProviderHealth {
    pub source_available: bool,
    pub status: &'static str,
}
#[derive(Debug)]
pub enum WatchMode {
    Poll(std::time::Duration),
}
pub trait ProviderAdapter {
    fn info(&self) -> ProviderInfo;
    fn session_roots(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    fn discover_sessions(&self) -> Result<Vec<PathBuf>, AdapterError> {
        Err(AdapterError::Unsupported("discover_sessions"))
    }
    fn import_usage(
        &self,
        _conn: &mut rusqlite::Connection,
    ) -> Result<crate::ingest::ImportReport, AdapterError> {
        Err(AdapterError::Unsupported("import_usage"))
    }
    fn watch_usage(&self) -> Result<WatchMode, AdapterError> {
        Err(AdapterError::Unsupported("watch_usage"))
    }
    fn read_quota(
        &self,
        _conn: &rusqlite::Connection,
    ) -> Result<crate::quota::Quota, AdapterError> {
        Err(AdapterError::Unsupported("read_quota"))
    }
    fn health_check(&self) -> Result<ProviderHealth, AdapterError> {
        Err(AdapterError::Unsupported("health_check"))
    }
}
pub struct CodexAdapter {
    pub home: PathBuf,
}
impl ProviderAdapter for CodexAdapter {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "codex",
            name: "OpenAI Codex",
            status: "implemented",
            capabilities: Capabilities {
                discover_sessions: true,
                import_usage: true,
                watch_usage: true,
                read_quota: true,
                health_check: true,
            },
        }
    }
    fn session_roots(&self) -> Vec<PathBuf> {
        vec![
            self.home.join("sessions"),
            self.home.join("archived_sessions"),
        ]
    }
    fn discover_sessions(&self) -> Result<Vec<PathBuf>, AdapterError> {
        let mut files = Vec::new();
        for root in self.session_roots() {
            if !root.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(root).follow_links(false) {
                let e = entry
                    .map_err(|_| AdapterError::Failed("session directory read failed".into()))?;
                if e.file_type().is_file()
                    && e.path().extension().and_then(|s| s.to_str()) == Some("jsonl")
                {
                    files.push(e.into_path());
                }
            }
        }
        Ok(files)
    }
    fn import_usage(
        &self,
        conn: &mut rusqlite::Connection,
    ) -> Result<crate::ingest::ImportReport, AdapterError> {
        crate::ingest::import(conn, self)
            .map_err(|_| AdapterError::Failed("local ingest failed".into()))
    }
    fn watch_usage(&self) -> Result<WatchMode, AdapterError> {
        Ok(WatchMode::Poll(std::time::Duration::from_secs(3)))
    }
    fn read_quota(&self, conn: &rusqlite::Connection) -> Result<crate::quota::Quota, AdapterError> {
        let q = crate::quota::read(conn);
        if q.status == "synced" {
            Ok(q)
        } else {
            Err(AdapterError::Unavailable(
                q.message.unwrap_or_else(|| "quota unavailable".into()),
            ))
        }
    }
    fn health_check(&self) -> Result<ProviderHealth, AdapterError> {
        let available = self.session_roots().iter().any(|p| p.is_dir());
        Ok(ProviderHealth {
            source_available: available,
            status: if available {
                "source_available"
            } else {
                "source_missing"
            },
        })
    }
}
pub struct PlannedAdapter {
    pub id: &'static str,
    pub name: &'static str,
}
impl ProviderAdapter for PlannedAdapter {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id,
            name: self.name,
            status: "unsupported",
            capabilities: Capabilities {
                discover_sessions: false,
                import_usage: false,
                watch_usage: false,
                read_quota: false,
                health_check: false,
            },
        }
    }
}
pub fn catalog() -> Vec<ProviderInfo> {
    let mut providers = vec![CodexAdapter {
        home: PathBuf::new(),
    }
    .info()];
    for (id, name) in [
        ("claude-code", "Claude Code"),
        ("deepseek", "DeepSeek"),
        ("glm", "GLM / 智谱"),
        ("kimi", "Kimi"),
    ] {
        providers.push(PlannedAdapter { id, name }.info())
    }
    providers
}
pub fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .unwrap_or_default(),
            )
            .join(".codex")
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planned_capabilities_are_unsupported() {
        let p = PlannedAdapter {
            id: "kimi",
            name: "Kimi",
        };
        assert_eq!(
            p.discover_sessions().unwrap_err(),
            AdapterError::Unsupported("discover_sessions")
        );
        assert!(matches!(
            p.watch_usage(),
            Err(AdapterError::Unsupported("watch_usage"))
        ));
        assert!(!p.info().capabilities.read_quota);
    }
    #[test]
    fn missing_codex_is_distinct_from_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let p = CodexAdapter {
            home: dir.path().into(),
        };
        assert_eq!(p.health_check().unwrap().status, "source_missing");
        assert!(p.discover_sessions().unwrap().is_empty());
        assert!(p.info().capabilities.import_usage);
    }
}
