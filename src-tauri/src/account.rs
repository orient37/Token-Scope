use crate::quota::{self, QuotaError};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

/// Display-only allowlist. Never persist profile data or expose raw RPC responses.
#[derive(Debug, Serialize)]
pub struct Profile {
    pub status: &'static str,
    pub auth_type: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub collected_at: String,
    pub message: Option<&'static str>,
}
impl Profile {
    fn empty(status: &'static str) -> Self {
        Self {
            status,
            auth_type: None,
            email: None,
            plan: None,
            collected_at: Utc::now().to_rfc3339(),
            message: None,
        }
    }
}
fn optional_text(v: &Value, max: usize) -> std::result::Result<Option<String>, QuotaError> {
    if v.is_null() {
        return Ok(None);
    }
    let text = v.as_str().ok_or(QuotaError::Protocol)?.trim();
    if text.chars().count() > max || text.chars().any(char::is_control) {
        return Err(QuotaError::Protocol);
    }
    Ok((!text.is_empty()).then(|| text.to_string()))
}
fn project(v: &Value) -> std::result::Result<Profile, QuotaError> {
    let account = v.get("account").ok_or(QuotaError::Protocol)?;
    let required = v["requiresOpenaiAuth"]
        .as_bool()
        .ok_or(QuotaError::Protocol)?;
    if account.is_null() {
        return Ok(Profile::empty(if required {
            "signed_out"
        } else {
            "no_account_required"
        }));
    }
    let auth = account["type"].as_str().ok_or(QuotaError::Protocol)?;
    if !matches!(auth, "chatgpt" | "apiKey" | "amazonBedrock") {
        return Ok(Profile::empty("unsupported"));
    }
    let mut profile = Profile::empty("synced");
    profile.auth_type = Some(auth.to_owned());
    if auth == "chatgpt" {
        profile.email = optional_text(&account["email"], 320)?;
        profile.plan = optional_text(&account["planType"], 64)?;
    }
    Ok(profile)
}
pub fn read() -> Profile {
    match quota::request_account().and_then(|v| project(&v)) {
        Ok(profile) => profile,
        Err(error) => {
            let (status, message) = match error {
                QuotaError::Unauthorized => ("signed_out", "请在 Codex 中登录后刷新。"),
                QuotaError::Unsupported => ("unsupported", "当前 Codex 不支持账号资料接口。"),
                QuotaError::NotFound => ("unavailable", "未找到 Codex 程序。"),
                QuotaError::Startup { .. } => ("unavailable", "Codex 账号服务启动失败。"),
                QuotaError::Timeout => ("unavailable", "账号资料读取超时，请稍后刷新。"),
                QuotaError::Protocol => ("unavailable", "账号资料响应不兼容，请更新 Codex。"),
                QuotaError::Rejected => {
                    ("unavailable", "账号资料读取失败，请检查 Codex 登录状态。")
                }
            };
            let mut profile = Profile::empty(status);
            profile.message = Some(message);
            profile
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn known_profile_only_keeps_verified_fields() {
        let p = project(&json!({"account":{"type":"chatgpt","email":"test@example.com","planType":"plus","access_token":"SECRET","name":"Unverified","avatarUrl":"https://untrusted.invalid"},"requiresOpenaiAuth":true})).unwrap();
        assert_eq!(p.email.as_deref(), Some("test@example.com"));
        assert_eq!(p.plan.as_deref(), Some("plus"));
        let raw = serde_json::to_string(&p).unwrap();
        for value in ["SECRET", "Unverified", "untrusted.invalid", "avatarUrl"] {
            assert!(!raw.contains(value));
        }
    }
    #[test]
    fn missing_account_and_auth_modes_are_distinct() {
        for (value, expected) in [
            (
                json!({"account":null,"requiresOpenaiAuth":true}),
                "signed_out",
            ),
            (
                json!({"account":null,"requiresOpenaiAuth":false}),
                "no_account_required",
            ),
            (
                json!({"account":{"type":"future"},"requiresOpenaiAuth":true}),
                "unsupported",
            ),
        ] {
            let p = project(&value).unwrap();
            assert_eq!(p.status, expected);
            assert!(p.email.is_none());
        }
        for auth in ["apiKey", "amazonBedrock"] {
            let p=project(&json!({"account":{"type":auth,"email":"do-not-expose@example.com"},"requiresOpenaiAuth":true})).unwrap();
            assert!(p.email.is_none());
            assert_eq!(p.status, "synced");
        }
        let p=project(&json!({"account":{"type":"chatgpt","email":null,"planType":"plus"},"requiresOpenaiAuth":true})).unwrap();
        assert!(p.email.is_none());
    }
    #[test]
    fn malformed_profile_does_not_look_signed_in() {
        for value in [
            json!({}),
            json!({"account":{},"requiresOpenaiAuth":true}),
            json!({"account":null}),
            json!({"account":{"type":"chatgpt","email":42},"requiresOpenaiAuth":true}),
            json!({"account":{"type":"chatgpt","email":"x\nSECRET"},"requiresOpenaiAuth":true}),
        ] {
            assert!(project(&value).is_err());
        }
    }
}
