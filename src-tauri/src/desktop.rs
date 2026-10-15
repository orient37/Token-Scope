use crate::{
    alerts,
    collector::Collector,
    finance::{self, Fx},
    ingest::ImportReport,
    providers::{self, CodexAdapter, ProviderAdapter},
    quota::{self, Quota},
    services::{self, Dashboard, Filter},
    storage,
};
use std::{
    path::PathBuf,
    sync::{mpsc, Mutex},
    time::Duration,
};
use tauri::{Emitter, Manager, State};
#[derive(Default)]
struct Collection {
    report: ImportReport,
    collector: Collector,
    last_error: Option<String>,
}
struct AppState {
    db: PathBuf,
    test_mode: bool,
    collection: Mutex<Collection>,
    quota: Mutex<Quota>,
    retry: mpsc::SyncSender<()>,
    fx_gate: Mutex<()>,
}
#[tauri::command]
async fn read_account() -> std::result::Result<crate::account::Profile, String> {
    tauri::async_runtime::spawn_blocking(crate::account::read)
        .await
        .map_err(|_| "账号资料服务异常".to_string())
}
#[tauri::command]
async fn read_usage_rate(
    state: State<'_, AppState>,
    filter: Option<Filter>,
) -> std::result::Result<crate::usage_rate::UsageRate, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let c = storage::open(&db).map_err(|_| "速率数据库不可用")?;
        crate::usage_rate::read(&c, &filter.unwrap_or_default(), chrono::Utc::now())
            .map_err(|_| "速率读取失败".into())
    })
    .await
    .map_err(|_| "速率服务异常".to_string())?
}
#[tauri::command]
async fn read_output_rate(
    state: State<'_, AppState>,
    filter: Option<Filter>,
) -> std::result::Result<crate::usage_rate::UsageRate, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let c = storage::open(&db).map_err(|_| "吞吐率数据库不可用")?;
        crate::usage_rate::read_output(&c, &filter.unwrap_or_default(), chrono::Utc::now())
            .map_err(|_| "吞吐率读取失败".into())
    })
    .await
    .map_err(|_| "吞吐率服务异常".to_string())?
}
#[tauri::command]
async fn read_generation_speed(
    state: State<'_, AppState>,
    filter: Option<Filter>,
) -> std::result::Result<crate::generation_speed::GenerationSpeed, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let c = storage::open(&db).map_err(|_| "速率数据库不可用")?;
        crate::generation_speed::read(&c, &filter.unwrap_or_default(), chrono::Utc::now())
            .map_err(|_| "速率读取失败".into())
    })
    .await
    .map_err(|_| "速率服务异常".to_string())?
}
#[tauri::command]
async fn read_dashboard(
    state: State<'_, AppState>,
    filter: Option<Filter>,
) -> std::result::Result<Dashboard, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let conn = storage::open(&db).map_err(|_| "无法读取本地数据库")?;
        services::dashboard(&conn, &filter.unwrap_or_default())
            .map_err(|_| "无法读取统计或筛选条件无效".into())
    })
    .await
    .map_err(|_| "统计服务异常".to_string())?
}
#[tauri::command]
fn read_health(state: State<AppState>) -> serde_json::Value {
    let collection = state.collection.lock().unwrap();
    serde_json::json!({"test_mode":state.test_mode,"report":collection.report,"quota":*state.quota.lock().unwrap(),"collector":collection.collector,"database_path":state.db,"last_error":collection.last_error})
}
#[tauri::command]
fn retry_ingest(state: State<AppState>) -> std::result::Result<(), String> {
    match state.retry.try_send(()) {
        Ok(()) | Err(mpsc::TrySendError::Full(_)) => Ok(()),
        Err(mpsc::TrySendError::Disconnected(_)) => Err("采集服务已停止".into()),
    }
}
#[tauri::command]
async fn refresh_fx(app: tauri::AppHandle) -> std::result::Result<Fx, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _gate = state.fx_gate.lock().map_err(|_| "汇率服务异常")?;
        let conn = storage::open(&state.db).map_err(|_| "数据库不可用")?;
        let fx = finance::refresh_fx(&conn, true).map_err(|_| "汇率服务异常")?;
        let _ = app.emit("usage-updated", ());
        Ok(fx)
    })
    .await
    .map_err(|_| "汇率服务异常".to_string())?
}
#[tauri::command]
async fn set_fx_enabled(app: tauri::AppHandle, enabled: bool) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let conn = storage::open(&state.db).map_err(|_| "数据库不可用")?;
        finance::set_enabled(&conn, enabled).map_err(|_| "无法保存汇率设置")?;
        let _ = app.emit("usage-updated", ());
        Ok(())
    })
    .await
    .map_err(|_| "设置服务异常".to_string())?
}
#[tauri::command]
async fn refresh_quota(app: tauri::AppHandle) -> std::result::Result<Quota, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let conn = storage::open(&state.db).map_err(|_| "数据库不可用".to_string())?;
        let q = quota::read(&conn);
        alerts::evaluate_quota(&conn, &q, chrono::Utc::now())
            .map_err(|_| "预警保存失败".to_string())?;
        *state.quota.lock().unwrap() = q.clone();
        let _ = app.emit("alerts-updated", ());
        let _ = app.emit("quota-updated", &q);
        Ok(q)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn read_alerts(
    state: State<'_, AppState>,
    filter: Option<alerts::AlertFilter>,
) -> std::result::Result<alerts::Inbox, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let c = storage::open(&db).map_err(|_| "预警数据库不可用")?;
        alerts::read_page(&c, &filter.unwrap_or_default()).map_err(|_| "预警读取失败".into())
    })
    .await
    .map_err(|_| "预警服务异常".to_string())?
}
#[tauri::command]
async fn acknowledge_alert(app: tauri::AppHandle, id: String) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let c = storage::open(&state.db).map_err(|_| "数据库不可用")?;
        alerts::acknowledge(&c, &id, chrono::Utc::now()).map_err(|_| "预警状态保存失败")?;
        let _ = app.emit("alerts-updated", ());
        Ok(())
    })
    .await
    .map_err(|_| "预警服务异常".to_string())?
}
#[tauri::command]
async fn set_alert_rules(
    app: tauri::AppHandle,
    enabled: bool,
    threshold: String,
) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let c = storage::open(&state.db).map_err(|_| "数据库不可用")?;
        alerts::configure(&c, enabled, &threshold)
            .map_err(|_| "阈值须为正整数且不超过 u64 范围")?;
        let _ = app.emit("alerts-updated", ());
        let _ = state.retry.try_send(());
        Ok(())
    })
    .await
    .map_err(|_| "预警服务异常".to_string())?
}
#[tauri::command]
async fn export_usage(
    state: State<'_, AppState>,
    filter: Option<Filter>,
) -> std::result::Result<crate::export::ExportReport, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let c = storage::open(&db).map_err(|_| "导出数据库不可用")?;
        crate::export::save(
            &c,
            &filter.unwrap_or_default(),
            &db.parent().ok_or("导出目录不可用")?.join("exports"),
        )
        .map_err(|_| {
            "导出失败：检查本地目录写入权限、磁盘空间和筛选；不完整报告保留为 partial 文件".into()
        })
    })
    .await
    .map_err(|_| "导出服务异常".to_string())?
}
#[tauri::command]
fn open_exports(state: State<AppState>) -> std::result::Result<(), String> {
    let directory = state.db.parent().ok_or("导出目录不可用")?.join("exports");
    if !directory.is_dir() {
        return Err("尚未创建导出目录".into());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&directory)
            .spawn()
            .map_err(|_| "无法打开导出目录")?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("当前平台暂不支持打开导出目录，请使用显示的路径".into())
    }
}
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = std::env::var_os("TOKENSCOPE_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            if !dir.is_absolute() {
                return Err("TOKENSCOPE_DATA_DIR must be absolute".into());
            }
            let _ = std::fs::create_dir_all(&dir);
            let db = dir.join("tokenscope.sqlite3");
            let (retry, requests) = mpsc::sync_channel(1);
            app.manage(AppState {
                db: db.clone(),
                test_mode: std::env::var("TOKENSCOPE_TEST_MODE").as_deref() == Ok("1"),
                collection: Mutex::new(Collection::default()),
                quota: Mutex::new(Quota::unavailable("尚未同步")),
                retry,
                fx_gate: Mutex::new(()),
            });
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let adapter = CodexAdapter {
                    home: providers::codex_home(),
                };
                loop {
                    let result = std::fs::create_dir_all(db.parent().unwrap())
                        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                            format!("创建数据库目录失败：{e}").into()
                        })
                        .and_then(|_| {
                            storage::open(&db).map_err(|e| format!("打开数据库失败：{e}").into())
                        })
                        .and_then(|mut c| {
                            let report = adapter.import_usage(&mut c)?;
                            if alerts::evaluate_usage(&c, chrono::Utc::now())? {
                                let _ = handle.emit("alerts-updated", ());
                            }
                            Ok(report)
                        });
                    let state = handle.state::<AppState>();
                    let (delay, transitioned, changed) = {
                        let mut collection = state.collection.lock().unwrap();
                        let previous = collection.collector.status;
                        collection.last_error = result.as_ref().err().map(|e| e.to_string());
                        let delay = collection
                            .collector
                            .finish(result.as_ref().ok(), chrono::Utc::now());
                        let transitioned = previous != collection.collector.status;
                        let changed = match result {
                            Ok(report) => {
                                let changed = report.changed > 0;
                                collection.report = report;
                                changed
                            }
                            Err(_) => false,
                        };
                        (delay, transitioned, changed)
                    };
                    let _ = handle.emit("ingest-health", ());
                    if changed || transitioned {
                        let _ = handle.emit("usage-updated", ());
                    }
                    // One collector owns cursor writes. Manual retries wake this wait,
                    // never start concurrent imports or accumulate an unbounded queue.
                    if matches!(
                        requests.recv_timeout(Duration::from_secs(delay)),
                        Err(mpsc::RecvTimeoutError::Disconnected)
                    ) {
                        break;
                    }
                }
            });
            let fx_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                {
                    let state = fx_handle.state::<AppState>();
                    if let Ok(_gate) = state.fx_gate.lock() {
                        if let Ok(conn) = storage::open(&state.db) {
                            if finance::refresh_fx(&conn, false).is_ok() {
                                let _ = fx_handle.emit("usage-updated", ());
                            }
                        }
                    };
                }
                std::thread::sleep(Duration::from_secs(3600));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_dashboard,
            read_usage_rate,
            read_output_rate,
            read_generation_speed,
            read_health,
            read_account,
            refresh_quota,
            retry_ingest,
            refresh_fx,
            set_fx_enabled,
            read_alerts,
            acknowledge_alert,
            set_alert_rules,
            export_usage,
            open_exports
        ])
        .run(tauri::generate_context!())
        .expect("TokenScope failed to start");
}
