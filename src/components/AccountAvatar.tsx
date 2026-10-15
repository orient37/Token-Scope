import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { RefreshCw, UserRound } from "lucide-react";
import { localTime } from "../lib/format";

interface Profile {
  status: string;
  auth_type: string | null;
  email: string | null;
  plan: string | null;
  collected_at: string;
  message: string | null;
}
export function AccountAvatar() {
  const [profile, setProfile] = useState<Profile | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const active = useRef(false);
  const pending = useRef(false);
  const lastAttempt = useRef(0);
  const refresh = useCallback(async () => {
    if (!isTauri() || pending.current) return;
    // Keep focus inside the card while its refresh button becomes disabled.
    if (root.current?.contains(document.activeElement)) {
      root.current
        .querySelector<HTMLButtonElement>(".account-avatar-trigger")
        ?.focus({ preventScroll: true });
    }
    pending.current = true;
    lastAttempt.current = Date.now();
    setBusy(true);
    setProfile(null);
    setFailed(false);
    try {
      const next = await invoke<Profile>("read_account");
      if (active.current) setProfile(next);
    } catch {
      if (active.current) setFailed(true);
    } finally {
      pending.current = false;
      if (active.current) setBusy(false);
    }
  }, []);
  useEffect(() => {
    active.current = true;
    void refresh();
    const timer = setInterval(() => void refresh(), 60000);
    const focus = () => {
      if (Date.now() - lastAttempt.current >= 5000) void refresh();
    };
    window.addEventListener("focus", focus);
    return () => {
      active.current = false;
      clearInterval(timer);
      window.removeEventListener("focus", focus);
    };
  }, [refresh]);
  const synced = profile?.status === "synced";
  const label = busy
    ? "正在读取账号"
    : failed
      ? "账号资料读取失败"
      : !isTauri()
        ? "浏览器预览 · 未连接"
        : ({
            synced: "已读取登录信息",
            signed_out: "未登录或登录已失效",
            no_account_required: "当前配置无需 OpenAI 账号",
            unsupported: "账号资料暂不支持",
            unavailable: "账号资料不可用",
          }[profile?.status ?? ""] ?? "等待账号信息");
  const authLabel = {
    chatgpt: "ChatGPT",
    apiKey: "API Key",
    amazonBedrock: "Amazon Bedrock",
  }[profile?.auth_type ?? ""];
  return (
    <div
      className="account-avatar"
      ref={root}
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => {
        if (!root.current?.contains(document.activeElement)) setOpen(false);
      }}
      onFocus={() => setOpen(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setOpen(false);
          event.stopPropagation();
        }
      }}
    >
      <button
        type="button"
        className="avatar account-avatar-trigger"
        aria-label="查看 Codex 账号资料"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={open ? "account-profile" : undefined}
        onClick={() => setOpen(true)}
      >
        <UserRound size={17} aria-hidden="true" />
      </button>
      {open && (
        <section
          className="account-popover"
          id="account-profile"
          role="dialog"
          aria-label="Codex 账号资料"
        >
          <div className="account-profile-heading">
            <span className="account-profile-avatar">
              <UserRound size={22} aria-hidden="true" />
            </span>
            <div>
              <strong>Codex 账号</strong>
              <small>{label}</small>
            </div>
          </div>
          {synced ? (
            <dl>
              <dt>登录方式</dt>
              <dd>{authLabel ?? "未提供"}</dd>
              <dt>邮箱</dt>
              <dd className="account-email">{profile.email ?? "未提供"}</dd>
              {profile.plan && (
                <>
                  <dt>套餐</dt>
                  <dd>{profile.plan}</dd>
                </>
              )}
            </dl>
          ) : (
            <p>
              {profile?.message ??
                (busy
                  ? "正在读取 Codex 当前登录信息…"
                  : "请在 Codex 中检查登录状态后刷新。")}
            </p>
          )}
          <p className="account-profile-note">
            默认头像 · Codex 当前未提供账号头像。
          </p>
          <div className="account-profile-footer">
            <small>
              {profile
                ? `读取于 ${localTime(profile.collected_at)}`
                : "仅在本机内存中显示"}
            </small>
            <button
              type="button"
              className="secondary"
              disabled={busy || !isTauri()}
              aria-busy={busy}
              onClick={() => void refresh()}
            >
              <RefreshCw size={13} aria-hidden="true" /> 刷新账号
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
