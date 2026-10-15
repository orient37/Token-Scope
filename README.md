# TokenScope

本地优先的 Codex Token 与套餐额度监控桌面应用，使用 **Tauri 2 + Rust + SQLite + React + TypeScript**。无需部署服务端或单独安装数据库。

**当前版本：v0.5.16 · Schema v6 · 已验证平台：Windows x64。** 当前实现按本版交付；总览速率采用近 60 秒可见输出吞吐率，具体定义见下文。macOS / Linux 尚未完成构建与安装验收，本文提供 Windows 源码运行步骤。

## 功能与数据边界

- **真实 Codex 数据链路**：只读本机会话及归档 JSONL，Rust 归一化、去重后写入 SQLite，通过 Tauri 展示到 React。正常每 3 秒检查追加，异常退避重试；不使用模拟数据填充监控界面。
- **用量看板**：总览、会话、时间线、预警、设置与数据源；支持项目、模型、时间筛选，输入、输出、缓存、推理分类，活跃热力图、趋势图、模型分布与项目消耗球体。
- **套餐额度**：通过 Codex 官方 App Server 读取多个限额池和窗口，显示已使用/剩余比例、重置倒计时、新鲜度与错误状态。套餐额度与本地 Token 消耗分别展示。
- **本地预警与导出**：可配置高用量规则、额度风险与已读状态；时间线页确认后导出当前筛选的全部计量事件，不受界面列表 500 条上限影响。
- **参考成本**：已知模型 API 等价估价及 USD/HKD/CNY 换算，不代表订阅账单。未知模型明确显示覆盖缺失。
- **Provider 范围**：目前只有 OpenAI Codex 可用；Claude Code、DeepSeek、GLM、Kimi 仅有扩展契约、图标与“未支持”状态。

### 总览输出速率口径

近 60 秒内已确认的 `output_tokens - reasoning_output_tokens` 为可见输出总量；`tokens/s = 总量 ÷ 60`，`tokens/m = 总量`。排除输入、缓存与推理 Token，合计当前筛选匹配的多个会话。每 3 秒刷新，并响应采集更新、窗口重新激活。

该指标包含工具等待与空闲时间，计数通常在响应结束后进入日志，因此是**可见输出吞吐率，不是逐 Token 实时解码速度**。采集健康但窗口无记录时可以显示 0；采集失败、过期或计量无法确认时显示未知。部分未知记录不纳入已确认总量。

## 运行环境

### 从源码开发、构建

| 依赖          | 要求与用途                                                                                                                                                                 |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 操作系统      | Windows 10/11 x64；本文命令使用 PowerShell，在仓库根目录执行                                                                                                               |
| Git           | 用于克隆源码；[官方下载](https://git-scm.com/downloads)                                                                                                                    |
| Node.js / npm | `package.json` 要求 Node.js ≥ 20.18；本机验证 Node 20.18.0、npm 10.8.2。新环境可选满足要求的受维护 LTS，使用仓库锁文件安装依赖；[官方下载](https://nodejs.org/en/download) |
| Rust / Cargo  | MSVC 工具链；本机验证 Rust/Cargo 1.93.0、`stable-x86_64-pc-windows-msvc`。仓库未声明更低 MSRV；[rustup 安装](https://www.rust-lang.org/tools/install)                      |
| C++ 编译环境  | Visual Studio Build Tools，安装“使用 C++ 的桌面开发”，包含 MSVC x64/x86 编译工具与 Windows SDK                                                                             |
| WebView2      | Microsoft Edge WebView2 Evergreen Runtime，用于桌面窗口；[官方下载](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)                                        |
| Codex         | 展示真实监控需要本机 Codex 会话日志；读取账户/额度还需要可用的 Codex 程序及对应登录状态。本机最近核查 CLI 为 `0.162.0-alpha.17.2`，日志字段可能随版本改变                  |
| 网络          | 首次安装/编译需下载 npm、Cargo 依赖和打包工具；运行时额度同步需要访问官方服务，汇率更新需要访问 Frankfurter                                                                |

Windows 原生依赖安装可对照 [Tauri 2 官方环境准备](https://v2.tauri.app/start/prerequisites/#windows)。安装完成后重新打开终端。无需全局安装 Tauri CLI，项目通过 npm 使用本地 CLI；SQLite 已通过 Rust `bundled` 特性编译进应用。

**额外验证依赖**：桌面专项测试中的部分脚本需要 PATH 中的 Python 3（标准库 `sqlite3`）；Shell 额度测试默认需要 PowerShell 7 的 `pwsh.exe`。普通用户运行应用不需要 Python，单元测试不要求登录 Codex。

### 仅运行编译好的程序

需要 Windows x64 和 WebView2；不需要 Node.js、Rust、Git、Python 或 Vite。若仓库发布者提供安装包，可运行 `TokenScope_0.5.16_x64-setup.exe`，完成后从开始菜单启动；也可运行独立 EXE。安装包按当前用户安装，缺少 WebView2 时会尝试联网安装运行时。当前安装包未签名。

## 从 GitHub 克隆并启动

### 1. 安装并检查工具链

安装上表依赖后，在 PowerShell 中检查：

```powershell
git --version
node --version
npm --version
rustc --version
cargo --version
rustup show active-toolchain
```

Rust 工具链应为 `x86_64-pc-windows-msvc`。如尚未配置，在 Windows x64 上执行：

```powershell
rustup default stable-x86_64-pc-windows-msvc
rustup component add rustfmt clippy
```

### 2. 克隆并安装前端依赖

将下面地址替换为 GitHub 仓库页面“Code”按钮提供的实际 HTTPS 地址；本 README 不预设仓库所有者。

```powershell
$repoUrl = 'https://github.com/YOUR_USERNAME/YOUR_REPOSITORY.git'
git clone $repoUrl Token-Scope
Set-Location Token-Scope
npm ci
```

请保留 `package-lock.json`、`src-tauri/Cargo.lock` 和 `tools/Cargo.lock`。`npm ci` 按锁文件安装依赖，Cargo 在首次启动/构建时安装 Rust 依赖。首次编译可能耗时较长，后续会复用缓存。

### 3. 准备真实 Codex 数据（需要监控时）

先在 Codex 中完成正常登录并使用一次会话。TokenScope 默认读取当前用户 `.codex` 下的 `sessions` 与 `archived_sessions`。无需向 TokenScope 填写 API Key、复制认证文件或配置服务端。

Codex 程序不在 PATH 时，Windows 还会检查 `%LOCALAPPDATA%\OpenAI\Codex\bin` 下的桌面安装目录。非标准安装可以按“配置与数据目录”指定可执行文件。没有安装或登录 Codex 时也能启动界面，但对应位置会显示无数据、未授权或不可用，不会生成虚假读数。

### 4. 启动完整桌面开发模式

```powershell
npm run desktop
```

该命令自动启动 `http://127.0.0.1:1420` 的 Vite 开发服务，并编译/启动带 `desktop` feature 的 Rust/Tauri 程序。无需另开一个 `npm run dev`。终端保持运行，退出开发模式可按 `Ctrl+C`。

**首次启动检查：**

1. 桌面窗口打开，显示 v0.5.16；设置页可查看实际数据库位置和采集状态。
2. 有可读取日志时，历史用量逐步出现，采集状态变为已连接；历史记录较多时等待首次导入完成。
3. 已登录且接口可用时，总览出现服务端额度窗口；额度失败不等于本地日志统计一定失败。
4. 在 Codex 完成新的响应后，观察采集更新；输出吞吐率按上述 60 秒口径变化，不要求响应进行中逐字变化。

仅预览前端时运行 `npm run dev` 并打开 `http://127.0.0.1:1420`。浏览器没有 Tauri 后端，会显示未连接状态；这不是完整监控应用。

## 构建与启动独立桌面版

在仓库根目录执行：

```powershell
npm run build:desktop
```

此命令依次执行 TypeScript 检查、Vite 构建、Rust Release 编译及 Windows NSIS 打包。默认未设置自定义 Cargo target 目录时，产物为：

| 产物           | 路径                                                                   |
| -------------- | ---------------------------------------------------------------------- |
| 独立程序       | `src-tauri/target/release/tokenscope.exe`                              |
| 当前用户安装包 | `src-tauri/target/release/bundle/nsis/TokenScope_0.5.16_x64-setup.exe` |

直接启动构建产物：

```powershell
& '.\src-tauri\target\release\tokenscope.exe'
```

独立版已内嵌前端资源，不需要同时启动 Vite。默认打包目标为 Windows NSIS；macOS / Linux 需要另行配置目标并验证，不能直接把本配置当作已验收的跨平台发布方案。

只构建 EXE 或构建供测试使用的独立 Debug EXE：

```powershell
npm run build:desktop -- --no-bundle
npm run build:desktop -- --debug --no-bundle
# Debug 产物：src-tauri/target/debug/tokenscope.exe
```

`npm run build` 只生成前端 `dist/`，不会生成桌面 EXE。直接 `cargo run` 也不能替代上述带前端准备和 `desktop` feature 的启动命令。

## 配置与数据目录

无需创建 `.env`。下面的环境变量由启动的应用进程读取；修改后需重新启动 TokenScope。

| 变量                   | 用途                                                                       | 默认行为                                                  |
| ---------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------- |
| `CODEX_HOME`           | 指定 Codex 根目录，内部应有 `sessions` / `archived_sessions`；建议绝对路径 | 当前用户 `.codex`                                         |
| `TOKENSCOPE_CODEX_BIN` | 指定 Codex 原生可执行文件完整路径，如 `codex.exe`                          | 空值视为未配置；依次查 PATH 和 Windows Codex 桌面安装目录 |
| `TOKENSCOPE_DATA_DIR`  | 指定 TokenScope 数据目录，**必须是绝对路径**                               | Tauri 应用数据目录                                        |

PowerShell 示例（替换为实际路径，仅设置需要覆盖的项）：

```powershell
$env:CODEX_HOME = 'D:\CodexData'
$env:TOKENSCOPE_CODEX_BIN = 'C:\Tools\Codex\codex.exe'
$env:TOKENSCOPE_DATA_DIR = 'D:\TokenScopeData'
npm run desktop
```

这些设置仅影响当前终端及其子进程，从资源管理器双击程序不会继承这个终端临时设置。显式指定错误的 Codex 程序路径时会报告错误，不会静默换用其他账户环境。

默认 SQLite 路径通常为 `%APPDATA%\local.tokenscope.desktop\tokenscope.sqlite3`，以应用设置页显示为准。首次启动自动创建并迁移到 Schema v6，不需要执行手动建表 SQL。源码启动和正式版默认共享该目录；需要隔离时先设置 `TOKENSCOPE_DATA_DIR`。独立 EXE 默认也不是“数据保存在 EXE 旁边”的便携模式。

导出文件位于应用数据目录 `exports/` 中，每次导出使用独立目录。升级前可在关闭应用后备份完整数据目录；不要通过删除数据库解决错误，也不要用旧 Schema 的程序打开新版数据库。

## 验证

以下命令均从仓库根目录执行。每条命令成功后再继续；自动化时检查退出码，不要把启动失败视为测试通过。

### 基础检查（无需真实 Codex 账号）

```powershell
npm run typecheck
npm test
npm run build
npx prettier --check src
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features desktop -- -D warnings
cargo clippy --locked --manifest-path tools/Cargo.toml --all-targets -- -D warnings
```

### Windows 桌面回归（隔离合成数据）

先安装 Python 3 并确认 `python --version` 可用，再构建独立 Debug EXE。脚本通过 Playwright 连接应用自己的 WebView2，不需要另行下载 Playwright Chromium。

```powershell
npm run build:desktop -- --debug --no-bundle
$env:TOKENSCOPE_TEST_EXE = (Resolve-Path '.\src-tauri\target\debug\tokenscope.exe').Path
npm run test:desktop
npm run test:output:desktop
npm run test:alerts:desktop
npm run test:reports:desktop
npm run test:account:desktop
npm run test:soak -- 10 9230
```

若验证 Release 或安装版，将 `TOKENSCOPE_TEST_EXE` 改为相应 EXE 的绝对路径。上述脚本使用 `.local/` 下的隔离数据和 WebView2 profile；合成数据明确标为测试，不应混入真实用户库。按顺序运行，避免调试端口冲突。需要本机交互式桌面和可用的回环调试端口，受限沙箱/无桌面 CI 可能无法连接 WebView。`test:soak` 参数为分钟数和端口，10 分钟通过不代表长期稳定性已验证。

### 真实日志与服务验证（可选）

此组需要可用的本机 Codex 数据。验证工具只读源日志，在单独路径保存验证库；`--quota` 额外访问官方额度接口，`--fx` 额外访问汇率服务。

```powershell
New-Item -ItemType Directory -Force '.local' | Out-Null
$verifyDb = Join-Path (Get-Location).Path ('.local/verification-' + [guid]::NewGuid().ToString('N') + '.sqlite3')
cargo run --locked --manifest-path tools/Cargo.toml --bin verify -- $verifyDb --output-rate
# 如需验证已登录账号的真实额度，使用同一隔离验证库：
cargo run --locked --manifest-path tools/Cargo.toml --bin verify -- $verifyDb --quota
# 如需验证联网汇率与已覆盖模型的参考成本：
cargo run --locked --manifest-path tools/Cargo.toml --bin verify -- $verifyDb --fx
```

真实日志在两次扫描之间继续追加时，重放一致性断言可能受新记录影响；应在日志稳定时重跑并核对变化来源。`--fx` 还要求数据包含有参考价格覆盖的模型，不适合作为无数据环境的基础验收。

验证真实 Shell 启动的账户/额度发现（需 PowerShell 7，沿用前面的 `TOKENSCOPE_TEST_EXE`）：

```powershell
$env:TOKENSCOPE_TEST_POWERSHELL = (Get-Command pwsh.exe).Source
npm run test:quota:shell
```

`npm run test:desktop -- --real` 还会启动真实日志/额度场景；该场景默认使用实际应用数据目录，**不是完全隔离测试**，普通贡献者优先使用上面的隔离验证工具。

### 百万记录性能验证（可选）

需要额外数 GB 磁盘空间、Python 3 和桌面 EXE。每次使用新路径，基准程序拒绝覆盖既有测试库。先完成前面的 EXE 构建和 `TOKENSCOPE_TEST_EXE` 设置。

```powershell
New-Item -ItemType Directory -Force '.local' | Out-Null
$benchId = [guid]::NewGuid().ToString('N')
$env:TOKENSCOPE_TEST_MILLION_DB = Join-Path (Get-Location).Path ('.local/million-' + $benchId + '.sqlite3')
cargo run --release --locked --manifest-path tools/Cargo.toml --bin benchmark -- $env:TOKENSCOPE_TEST_MILLION_DB
npm run test:million:desktop
$ingestRoot = Join-Path (Get-Location).Path ('.local/ingest-million-' + $benchId)
cargo run --release --locked --manifest-path tools/Cargo.toml --bin ingest_benchmark -- $ingestRoot
```

测试产物保留在 `.local/`，不进入应用分发包；脚本不批量删除产物。

### 已有验证记录

v0.5.16 本机已通过 17 项前端测试、59 项 Rust 测试、桌面专项及真实日志/额度验证。追加 10 分钟耐久覆盖 170 次追加、1 次重启、3 次断开恢复，前端错误 0；百万事件总量与 CSV 精度通过，聚合查询实测约 5.5–7.2 秒。它们是本机历史验证结果，不保证所有机器性能一致，也不替代克隆后的本地检查。详细命令、环境限制和证据见 [验证记录](docs/VALIDATION.md)。

## 常见问题

| 现象                                   | 排查方式                                                                                     |
| -------------------------------------- | -------------------------------------------------------------------------------------------- |
| `npm.ps1` 被执行策略阻止               | 可用 `npm.cmd ci`、`npm.cmd run desktop`，或在命令提示符中执行等价命令；无需关闭全局安全策略 |
| 找不到 `link.exe`、MSVC 或 Windows SDK | 检查 Build Tools 的 C++ 桌面工作负载及 SDK，重开终端并确认 Rust 使用 MSVC 工具链             |
| 端口 1420 已被占用                     | 关闭自己启动的旧 Vite 开发服务，再运行 `npm run desktop`；不要同时启动两份开发服务器         |
| 浏览器有界面但没有监控数据             | `npm run dev` 仅预览前端，完整应用请运行 `npm run desktop`                                   |
| 无会话 / 等待数据源                    | 核对 `CODEX_HOME`、日志目录权限和是否已有 Codex 会话；不要把其他 Provider 日志当 Codex 导入  |
| 额度不可用                             | 检查 Codex 登录、程序路径、网络及接口版本；在设置页读取具体失败信息，再手动重试              |
| 速率为 0 或响应结束后才变化            | 按近 60 秒已确认可见输出统计；空闲窗口可为 0，不提供逐 Token 流式测速                        |
| 数据库异常                             | 保留数据目录，查看设置页实际路径和错误详情；不要自动删库或运行他人的一次性恢复脚本           |
| 测试找不到 EXE / 无法连接 WebView      | 先独立构建，核对 `TOKENSCOPE_TEST_EXE`、WebView2 和调试端口，使用可交互桌面环境              |
| 首次编译/打包下载失败                  | 检查 npm/Cargo/打包工具下载的网络与代理配置；保留锁文件，不以删除用户库处理依赖错误          |

## 数据隐私与发布说明

本地采集只读 Codex 日志，默认不保存提示词、模型回复、代码正文或认证凭据。账户资料只存于内存。额度通过官方 App Server 获取；可选汇率更新仅发送币种，可在设置中关闭。应用不上传个人用量。

源码发布应保留前后端源码、配置、迁移、图标、测试、文档及三份锁文件。`.gitignore` 已排除 `node_modules/`、`dist/`、Rust `target/`、`.local/`、`artifacts/` 和 `*.sqlite3*`；这些不是克隆后运行所需的源码。通过 GitHub 网页手动上传文件时，忽略规则不会替你筛选文件，勿上传本机数据库、认证文件、日志或验证截图。发行 EXE/安装包可单独放在 GitHub Releases。

## 许可证

本项目采用 [MIT License](LICENSE)，版权声明为 `Copyright (c) 2026 TokenScope contributors`。

选择 MIT 是为了方便个人与团队使用、修改、分发和商用：条款简短，不要求衍生作品必须公开源码，便于集成与贡献。分发本软件或其重要部分时，须保留原版权声明和许可声明；软件按现状提供，不作担保。标准条款见 [Open Source Initiative 的 MIT 文本](https://opensource.org/license/mit)。

第三方依赖及品牌资源仍遵循各自的许可证、商标与使用条件，本项目的 MIT 授权不替代这些条件。品牌资源来源见 [供应商图标说明](src/assets/providers/README.md)。

## 项目结构与文档

```text
src/                     React UI、组件、样式、类型与前端测试
src-tauri/src/           Rust 采集、计量、存储、服务、Tauri Commands
src-tauri/migrations/    SQLite 版本化迁移
src-tauri/icons/         桌面图标
src-tauri/tauri.conf.json 开发服务、窗口与打包配置
tools/                  独立验证与性能基准 Rust crate
tests/                  桌面回归脚本与合成 Fixture
docs/                   架构、数据来源、设计决策与验证记录
```

- [架构与数据流](docs/ARCHITECTURE.md)
- [数据来源与 Codex 版本验证](docs/DATA_SOURCES.md)
- [数据模型与计量规则](docs/DATA_MODEL.md)
- [路线图](docs/ROADMAP.md)、[技术决策](docs/DECISIONS.md)
- [验证记录](docs/VALIDATION.md)、[协作约定](AGENTS.md)

## 历史版本记录

以下保留开发历程；旧版本中的页面入口、速率算法、Schema 与测试路径仅反映当时状态，当前运行方式及口径以上文 v0.5.16 说明为准。当前版本按既有吞吐率方案交付，不以多会话逐 Token 实时测速作为交付条件。

<details>
<summary>展开 v0.1.1–v0.5.16 更新记录</summary>

## v0.1.1 升级

首次启动迁移到 Schema v2，并重新核对日志一次。原始计量记录与额度历史保留，不删除数据库。重置恢复必须具备连续逐响应证据，缺证据仍显示不确定；迁移前后真实数据验证见 docs/VALIDATION.md。

## v0.1.2 更新

新增三种趋势粒度、UTC 对齐且按本地时区展示、长范围合并、采集退避与手动恢复。延用 Schema v2，无需重建用户数据库。验证详情见 [验收记录](docs/VALIDATION.md)。

## v0.2.0 更新

升级至 Schema v4：保留原始用量与额度历史，新增未分配累计历史、查询索引、价格版本、设置和汇率快照。首条快照只有匹配的逐响应证据才能确认本次用量；继承的历史累计单独显示，不进入时间线或估价。无法核实的记录仍显示未知。

成本采用 2026-10-09 已核实的 Standard/短上下文公开模型参考价，重估筛选范围用量；不是 Codex 套餐费用或实际 API 账单。未知模型不代入其他模型价格，展示覆盖比例；缓存与推理子集不重复计价。未包含工具、缓存写入差价、长上下文、加速档和地区溢价。

汇率默认每日更新，后台每小时检查缓存。仅向 Frankfurter 请求 USD→HKD/CNY，不传个人用量；设置可关闭。显示观测日期、采集时间和来源；超过 24 小时的缓存、超过 7 天的报价或同步失败暂停换算。USD 参考估算无需汇率。测试 Fixture 仅在 TOKENSCOPE_TEST_MODE=1 的独立数据库中使用，并显示开发验证提示。

百万事件与原生耐久验证的机器实测和边界见 [验收记录](docs/VALIDATION.md)。最近事件/轮次各最多 500 项，轮次截断明确提示，会话明细独立查询；卡片、筛选汇总和趋势不受列表上限影响。

百万事件验证（仅生成合成数据，路径必须未存在，保留产物以供审计）：

```powershell
cargo run --release --manifest-path tools/Cargo.toml --target-dir src-tauri/target --bin benchmark -- .local/million-test-20261009.sqlite3
cargo run --release --manifest-path tools/Cargo.toml --target-dir src-tauri/target --bin ingest_benchmark -- .local/INGEST_MILLION_TEST_ONLY_NEW
# 指定实际安装的 Release exe 后验证百万查询中的界面响应
$env:TOKENSCOPE_TEST_EXE = 'C:\path\to\installed\tokenscope.exe'
npm run test:million:desktop
```

耐久脚本使用独立目录和 WebView2 profile；源目录移动仅限 TEST_ONLY 范围，并保留所有测试产物。运行 30 分钟短期验证可改为 1440 分钟做一天验证，需保持电脑与程序持续运行；尚未将一天验证记为通过。

耐久与百万 GUI 验证脚本额外需要 PATH 中的 Python 3（标准库 sqlite3）与 npm ci 安装的 Playwright；正常运行 TokenScope 无需 Python。基准保留的数据库和 JSONL 可占用数 GB，请使用独立测试目录；脚本不会批量清理文件。

## v0.3.0 界面与预警

界面采用暖灰浅色/炭灰深色、陶土色强调、简洁侧栏与可读表格；主题本地保存。Alt+1…6 切换页面，键盘可跳到主内容；成本模型与汇率来源按需展开，估价边界/未知历史提示保持可见。小窗口将指标分两列，表格局部滚动，不压缩模型名为逐字排版。

新增预警页面与设置规则：默认每会话 UTC 固定五分钟窗口已确认 Token ≥1,000,000 时提示，阈值可设置正 u64 整数；服务端额度已使用 ≥80% 提示、≥95% 严重提示。未知增量、未分配历史、未来/过期快照、已重置和缺重置时间的额度不触发。高用量阈值不是统计学异常结论，也不是账单。

相同会话窗口或同账户/限额池/重置窗口幂等更新；已读状态重启保留，普通更新不重复变为未读，严重等级升级重新提示。历史页显示最近 200 条，记录检测时的观测值，不代表当前额度状态。关闭规则保留历史。只实现应用内预警，不发送系统通知。

Schema v5 无损新增 alerts 与配置；旧 0.2.x 程序会拒绝更高 Schema，应使用新版，不要删除数据库。验证：npm run test:alerts:desktop（先独立构建），可用 TOKENSCOPE_TEST_EXE 指向已安装 Release；所有 Fixture 使用独立 TEST_ONLY 路径与提示。

## v0.4.0 活跃日历与趋势图

总览新增 GitHub 风格近 365 天活跃热力图。Rust 按本机系统时区逐事件转换日期（不使用固定时差跨越夏令时），对当前时间/模型/项目筛选中的已确认增量聚合；点击、方向键及 Home/End 可查看精确整数。颜色按当前可见最高日用量分四级；空白只表示无可确认消耗，不保证日志覆盖完整。

总览和时间线支持折线、柱状、K 线切换。折线和柱状展示时间桶总量；K 线展示桶内单条计量增量的首值、末值、最低、最高，样本数不是请求数。空桶 K 线留空，同时间事件按稳定 ID 排序。所有值来自 Rust 同一 SQLite 读事务，最多 720 桶；图形浮点近似，提示保留精确十进制整数。未知增量和未分配历史均排除。

参考上传图片调整圆角、卡片间距和数字层级。Codex 数据源、可识别的 OpenAI 模型、成本明细与模型排行显示本地 SVG 供应商标识；未知模型显示问号。标识只表示来源，不宣称支持其他 Provider；无外部图片请求。

## v0.5.0 本地导出与预警分页

总览、会话和时间线可导出当前时间/项目/模型筛选的**全部计量事件**，不受界面 500 条上限影响。Rust 在一个 SQLite 读快照中流式写入，不经 JS 处理 Token 整数。导出位于应用数据目录 exports 下的独立时间戳目录；成功后显示路径，可打开导出目录。

每份报告包含 UTF-8 BOM / CRLF 的 usage.csv 与 metadata.json：确认增量、缓存/推理子集、UTC 事件/采集时间、Provider/项目/会话/模型/epoch/质量；未知增量为空，未分配历史另列并排除汇总。JSON 记录筛选、精确汇总、未知数及口径。电子表格导入时将 Token 列设为文本，避免软件自动转换大整数；公式前缀元数据添加单引号。无提示词、回复、账号 ID 或原始日志路径，无上传。CSV 文件中的 session_id 为日志源会话标识。

导出不覆盖旧报告。写入过程中为 .partial；只有同时存在完整 usage.csv 和 metadata.json 才表示完成。失败会保留不完整目录并显示失败，不以成功报告冒充；目前没有中途取消或自选保存位置。

预警历史每页 50 条，无最近 200 条的范围限制。可筛选类型、级别、已读状态和首次触发时间（本地输入，结束不含）。首屏跟随新预警，开始翻页后按数据库返回的首次触发截止点固定范围，刷新或修改筛选回到实时首屏；已读/级别仍反映最新状态。全局未读数与筛选匹配数分别显示，最后一页因已读筛选缩减时自动回退。Schema 仍为 v5。

验证命令：`npm run test:reports:desktop`（独立 TEST_ONLY 数据）和 `npm run test:million:desktop`（需先生成百万 Fixture，含全量 CSV 独立解析），可设置 TOKENSCOPE_TEST_EXE 指定实际安装版。

## v0.5.1 页面职责与导航

总览聚焦已确认 Token / 输入 / 输出 / 缓存、活跃热力图、账户额度、参考成本、最近会话与项目/模型概况。完整趋势不再在总览重复展示；“查看消耗时间线”沿用当前时间/项目/模型筛选。

时间线集中展示折线/柱状/K 线、分钟/小时/天粒度与最近 500 条计量事件。趋势粒度控件仅在时间线显示。事件表局部纵向滚动并固定表头，避免 500 条记录将页面无限拉长。独立套餐额度导航已合并到总览，全部限额池/窗口/重置/状态、手动同步与额度口径折叠说明保留。额度始终为账户级服务端快照，不受本地模型/项目筛选影响。

五项导航快捷键：Alt+1 总览、Alt+2 会话、Alt+3 时间线、Alt+4 预警、Alt+5 设置与数据源。数据计量、SQLite Schema v5 和导出口径不变。

## v0.5.2 额度同步与导出确认

Windows 普通桌面启动无需将 Codex 加入 PATH：优先使用 TOKENSCOPE_CODEX_BIN，再查 PATH，最后查本机 LOCALAPPDATA/OpenAI/Codex/bin 的桌面安装目录。版本目录按可执行文件修改时间选择，显式配置失败不会偷偷回退；不读取认证文件。未找到程序、启动失败、超时、未登录、接口不支持及响应异常分别提示，不回显服务端原始错误。安装在其他目录时可通过环境变量指定程序，并重启 TokenScope。

点击导出 CSV 先弹出确认框，展示项目、模型、起止时间及本机保存说明；点击确认才写文件。取消、Esc、点击遮罩均不导出。弹窗固定打开时的筛选，确认后的数据库快照覆盖完整范围，仍不是表格当前页导出。

## v0.5.3 空配置修复

Windows 桌面启动可能保留值为空的 TOKENSCOPE_CODEX_BIN。空值现在视为未配置，继续查找 PATH / Codex 安装目录；非空错误配置仍明确失败。启动失败提示补充系统错误，便于区分空路径与权限等问题。新增 npm run test:quota:shell，使用 PowerShell Start-Process、空覆盖变量与不含 Codex 的 PATH 进行真实只读额度验证（需已登录 Codex；可通过 TOKENSCOPE_TEST_POWERSHELL 指定 PowerShell 7 路径）。

## v0.5.4 Token 消耗速率

Codex 信息栏增加 tokens/s、tokens/m，按当前筛选下最近 60 秒已确认的输入 + 输出增量计算。tokens/m 为窗口总量，tokens/s 为总量 / 60（保留两位小数），是消耗速率而非模型输出生成速度。每 3 秒刷新，过期事件自然离开窗口；未知增量提示且排除，缓存与推理子集不重复相加。采集未就绪/异常/过期或查询失败显示 —，不以零冒充不可用。读取最近时间索引，不刷新全历史图表；Schema v5 不变。

## v0.5.5 趋势峰值标记

时间线的折线/柱状图标记最高时间桶总量，K 线标记最高单条计量增量；图上圆点与标签、图表上方峰值摘要同步更新，悬浮提示给出精确整数和本地时间区间。仅从 Rust 已返回的有界聚合中选择标注位置，BigInt 比较避免大整数精度造成峰值选错；并列最高只标最早时间桶，空数据、空 K 线桶和全零不生成非零峰值。标注遵循当前项目/模型/时间与粒度；后端计量和 Schema v5 不变。

## v0.5.6 模型生成速度估算

总览 Codex 栏改为模型可见输出生成速度（估算），替换此前的输入 + 输出消耗速率。可见输出 = output_tokens − reasoning_output_tokens；耗时 = task_complete.duration_ms − time_to_first_token_ms；tokens/s = 可见输出 × 1000 / 耗时毫秒，tokens/m = 可见输出 × 60000 / 耗时毫秒。两种单位分别从同一精确整数比率四舍五入至两位小数，避免先舍入每秒再乘 60 的误差。

仅接受 task_started / task_complete 完整、有且只有一个独立响应证据、无工具/压缩或其他非消息项、模型一致且输出计数与已归一化增量一致的轮次；拒绝缺失/非正耗时、未知计量及没有可见输出的样本。展示当前筛选匹配的最近完成轮次（最多近 15 分钟），多个并发会话取最近一条，不相加；最近轮次不合格时显示未知，不退回较早的高速度。不是实时流式测量，扣除首 Token 等待仍可能包含收尾开销，故明确标注估算。

Schema v6 新增 generation_samples 和索引，仅保存耗时、计数和响应身份哈希，不保存消息正文或工具参数。游标版本 3 首次升级只读重放一次以回填，之后继续增量采集；原计量和额度记录保留。旧版本拒绝 Schema v6，升级后应继续使用新版。read_usage_rate 保留为独立消耗统计，UI 改用 read_generation_speed。

## v0.5.7 导出入口集中

“导出当前筛选的计量事件”卡片仅在时间线页显示，总览与会话页移除该入口。原有全局筛选、CSV 完整范围、确认/取消及键盘交互不变；后端计量与 Schema v6 不变。

## v0.5.8 GUI 风格应用图标

保留脉冲线品牌符号，使用 GUI 的暖白 #faf9f5、陶土橙 #b45c40 与中性边框 #dedbd3，圆角底板及透明外角适配桌面图标。src-tauri/icons/source.svg 为图标源，Tauri CLI 生成 PNG / ICO / ICNS；侧栏与 favicon 复用同一 SVG，bundle.icon 明确指定桌面资源。原图标已备份到 .local/ICON_BACKUP_0.5.7_*；数据采集、计量和 Schema v6 不变。其他平台仅生成资源，未进行安装验证。

## v0.5.9 Codex 账号资料卡

右上角改为账号入口，支持鼠标悬浮、键盘聚焦、Esc 关闭与手动刷新。使用官方 account/read（refreshToken=false）读取当前 Codex CLI 配置对应的登录方式、真实邮箱和套餐，每分钟及窗口重新激活时刷新。账号可能与使用其他 CODEX_HOME 的 Agent 实例不同；不从会话日志推断当前身份。

当前实测接口不提供账号头像、姓名或用户名，因此使用明确标注的默认头像，这些字段显示“未提供”。API Key、Bedrock、无需 OpenAI 账号、未登录、不支持、不可用及加载中分别展示。资料仅存于进程/页面内存，不写 SQLite、localStorage、报告或日志；不读取认证文件、不访问第三方头像服务，刷新失败清除旧资料。Schema v6 不变。

## v0.5.10 账号资料精简与供应商图标

移除账号资料卡中的姓名 / 名称、用户名两行，仅保留实际可读取的登录方式、邮箱与套餐；默认头像说明仅描述头像缺失。工作空间、输出 Token 卡片、会话列表及设置中的 Codex 数据源均复用 ProviderMark 的 OpenAI 标识，移除终端图标；其他尚未接入的 Provider 保留未知标识和未支持状态，不推断其已可用。现有模型标识保持按已验证模型前缀判断。后端采集、计量与 Schema v6 不变。

## v0.5.11 未来 Provider 品牌标识

设置与数据源的 Provider 能力列表为 Claude Code、DeepSeek、GLM / 智谱、Kimi 显示各自官方图标，仍明确标注未支持。品牌资源随应用本地打包，不在运行时联网加载；Kimi 使用官方浅色/深色变体。资源来源见 src/assets/providers/README.md。统一 Claude Code 的前端占位 ID 为后端使用的 claude-code，图标只依 Provider ID 选择，不推断模型或接入能力。采集、计量与 Schema v6 不变。

## v0.5.12 图标方向与配色统一

输入 Token 保留右上箭头，输出 Token 使用相反的左下箭头。项目 / 会话入口的 OpenAI 标识统一为与模型标签相同的 20 px 与中性主题配色；移除 Provider 列表对 SVG 的强调色覆盖，OpenAI 标识在浅色 / 深色下继承统一黑白色系。其他供应商品牌色及未支持状态不变；采集、计量和 Schema v6 不变。

## v0.5.13 规则保存、分布图与按钮反馈

监控规则使用独立编辑草稿，后台预警读取不会覆盖未保存输入；保存期间阻止重复提交，丢弃保存前的旧读取结果，保存后回读确认数据库值并显示生效阈值。错误不再被后台刷新清除；支持带规范千位逗号的正整数，以 BigInt 校验 u64 范围。已有预警是历史记录，提高阈值不会删除旧记录。

总览模型分布改为占比环形图与排序条形列表，项目消耗改为横向排行，默认显示前 8 项，可展开全部。复用 Rust 聚合计数，BigInt 排序和总量保留精度，仅绘图比例使用有限小数，精确值在提示中保留；所有模型都纳入环形图，列表展开不会改变总量。按钮统一圆角、悬停、按压、禁用与键盘焦点反馈，异步操作标记忙碌状态，遵循减少动态效果设置。Schema v6 与采集口径不变。

## v0.5.14 项目悬浮气泡

总览项目消耗改为每项目一个球的气泡群，使用已有 Rust 聚合计数排序。半径随相对消耗增加，小球保留最小操作尺寸，因此面积不代表严格线性占比；精确数值与占比在悬停浮层中显示。鼠标悬停、键盘聚焦或点击将球放大并置顶，Esc 关闭；浮层挂载到页面顶层，滚动/缩放时关闭过期定位。轻微漂浮遵循减少动态效果设置；全部项目保留，密集时纵向延伸，无虚构数据。模型分布不变，Schema v6 不变。

## v0.5.15 启动诊断与本机恢复

设置与数据源的采集健康状态现在展示实际数据库位置、失败阶段和错误详情；成功重试会清除旧错误。未改变计量口径、Schema v6 或 Codex 原始日志。数据库异常不会自动删除或重建用户数据。

本机 2026-10-15 的双击失败确认来自 SQLite 结构损坏，已保留原文件备份、使用完整性检查通过的副本恢复并验证真实 Explorer 启动。此次恢复是针对本机的维护操作，安装新版不会自动修复其他损坏数据库。不同启动环境曾读取同一路径的不同文件内容，底层原因尚未确认，详见 docs/VALIDATION.md。

## v0.5.16 可见输出吞吐率与项目球体（2026-10-15）

用户确认将总览主速度指标改为近 60 秒可见输出吞吐率。旧实现每 3 秒查询，但仅接受近 15 分钟内已完成、无工具且单响应的轮次；当前真实编程工作负载多为工具/多响应轮次，因此长期显示未知。这是旧指标覆盖条件的限制，不能通过把输入消耗算作生成速度来修复。

新命令 `read_output_rate` 对当前筛选内 `(now-60s, now]` 的已确认增量计算 `output - reasoning`，跨匹配会话合计；每分钟值为窗口总量，每秒值为总量除以 60。Rust checked u128 聚合，十进制字符串跨 IPC。包含工具等待、空闲时间；按日志计量事件时间归桶，日志通常在响应结束后报告计数，不代表逐 Token 实时解码速度。没有窗口记录且采集健康时可显示 0；计量全未知、采集异常或读数过期显示未知，部分未知明确标注仅已确认部分。

界面每 3 秒查询，收到 usage-updated 或窗口重新激活时立即刷新；查询串行合并，切换筛选或卸载后丢弃旧结果。独立时钟检查读数新鲜度。旧 `read_generation_speed` 严格估算与 `read_usage_rate` 总消耗接口继续保留，主界面使用新口径。Schema v6 无迁移，不重读或改写 Codex 原始日志。

项目球体增加错峰浮动、表面呼吸和轻微光晕，使用区分更明显的八色配色及深色主题变体。用量排序变化时保留已分配的项目颜色；全部项目、精确计数和占比仍可访问。悬停/键盘聚焦暂停动效，Esc 关闭详情，减少动画模式禁用动效。半径含最小操作尺寸，面积仍不是线性消耗编码。

</details>
