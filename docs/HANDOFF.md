# TokenScope 项目交接文档

> 交接日期：2026-10-15（Asia/Shanghai）  
> 当前交付版本：**v0.5.16**；数据库：**Schema v6**。  
> 工作目录：`D:\dev-chatgpt\Token-Scope`。  
> 用途：新聊天无需完整读取旧聊天，即可理解产品、约束、代码、已做工作、验收证据和未决事项。



## 最新任务状态：实时模型输出速率（2026-10-15）

用户已明确确认此前本机数据库损坏完成修复；不再把旧根因调查作为本轮交付前置条件，历史取证记录仍保留。禁止再次执行一次性恢复脚本或自动删库。

用户在 v0.5.16 质量验证期间进一步明确：需要**响应进行中的实时模型输出 Token/s，支持多会话**。这一要求取代此前接受的 60 秒吞吐率口径。当前 v0.5.16 仍是下述吞吐率实现，不能标为满足新要求；暂停该项最终交付验收，未替换发行 EXE。

只读核查最近 8 份真实日志：1,116 条响应计数无逐响应耗时字段；68 个完成轮次中 68 个含 duration_ms、65 个含 time_to_first_token_ms，它们是整轮计时。独立 App Server 只读 thread/loaded/list 实测成功但 loaded_count=0，未创建或恢复任何用户会话。生成协议 AgentMessageDeltaNotification 只有文本 delta；不能把字符/片段个数当 Token。尚未获得桌面客户端各活跃会话的可靠逐 Token 计数流。详见 DATA_SOURCES.md 最新记录与 VALIDATION.md 的追加验证。

## v0.5.16 可见输出吞吐率与项目球体（2026-10-15，已交付行为）

用户确认将总览主速度指标改为近 60 秒可见输出吞吐率。旧实现每 3 秒查询，但仅接受近 15 分钟内已完成、无工具且单响应的轮次；当前真实编程工作负载多为工具/多响应轮次，因此长期显示未知。这是旧指标覆盖条件的限制，不能通过把输入消耗算作生成速度来修复。

新命令 `read_output_rate` 对当前筛选内 `(now-60s, now]` 的已确认增量计算 `output - reasoning`，跨匹配会话合计；每分钟值为窗口总量，每秒值为总量除以 60。Rust checked u128 聚合，十进制字符串跨 IPC。包含工具等待、空闲时间；按日志计量事件时间归桶，日志通常在响应结束后报告计数，不代表逐 Token 实时解码速度。没有窗口记录且采集健康时可显示 0；计量全未知、采集异常或读数过期显示未知，部分未知明确标注仅已确认部分。

界面每 3 秒查询，收到 usage-updated 或窗口重新激活时立即刷新；查询串行合并，切换筛选或卸载后丢弃旧结果。独立时钟检查读数新鲜度。旧 `read_generation_speed` 严格估算与 `read_usage_rate` 总消耗接口继续保留，主界面使用新口径。Schema v6 无迁移，不重读或改写 Codex 原始日志。

项目球体增加错峰浮动、表面呼吸和轻微光晕，使用区分更明显的八色配色及深色主题变体。用量排序变化时保留已分配的项目颜色；全部项目、精确计数和占比仍可访问。悬停/键盘聚焦暂停动效，Esc 关闭详情，减少动画模式禁用动效。半径含最小操作尺寸，面积仍不是线性消耗编码。

以上 v0.5.16 决策优先于下文保留的 v0.5.15 历史交接快照。当前正式产物与验证结果见 BUILD_INFO_0.5.16.json 和 VALIDATION.md 末尾；启动损坏和不同文件视图的根因仍未查明。

## 1. 给新聊天的启动提示词

将以下内容粘贴到新聊天，并附上本文件或允许其访问工作目录：

```text
你将接手 TokenScope，本地项目位于 D:\dev-chatgpt\Token-Scope。
请先完整阅读根目录 AGENTS.md 和 docs/HANDOFF.md，再按交接文档的阅读顺序核对源码与文档。

当前交付版本 v0.5.16、Schema v6。发行版仍显示近 60 秒可见输出吞吐率；用户最新要求是响应进行中、多会话实时模型输出速率，尚未实现，详见本文件顶部最新记录。真实 Codex → Rust → SQLite → Tauri → React 链路已经实现，不能重新搭建成 Demo，不能使用模拟数据冒充监控数据。其他 Provider 仅有扩展契约和未支持状态。

此前“从文件夹双击 exe 后统计、采集和额度一起失败”的本机数据库损坏已完成修复，用户确认按已修复状态继续，不作为本轮交付阻塞。历史工具与 Explorer 文件视图差异取证仍保留，不能用工具单次 integrity_check 正常否定真实故障。不要再次直接运行 .local 中的一次性恢复脚本，也不要自动删库重建。

先报告你核对到的当前版本、现有功能和最高优先级未决问题，再根据我本轮给出的具体任务继续。若我没有要求开发新功能，不要自行扩展任务。一般问题自主解决；必要的产品决策或真正缺失的信息再询问我。保留现有文件与数据，禁止批量删除。
```

## 2. 任务状态与信息可信度

- 产品已经有可运行 Windows 桌面应用、安装包和真实数据验证，不是空项目。
- 本次交接前最后一项工程任务：恢复双击启动失败，并发布 v0.5.15；其后用户要求生成本交接文档。
- v0.5.15 的数据库恢复是**针对本机的一次维护操作**。安装新版并不自动修复任意损坏数据库。
- 最终验证窗口中采集、额度均为 `synced`；最后已通过资源管理器直接打开正式 exe。未来接手时必须重新确认运行状态，不能把该截面当作永久保证。
- 对话中用户多次说“额度已恢复，继续任务”，这只是继续工作的指示，不是 TokenScope 所监控账户额度的测试证据。
- 旧聊天已存在压缩摘要。本文件结合当前可见历史、摘要、源码和仓库记录整理；不能可靠给出整个聊天的上下文压缩次数，也不能声称恢复了未保留的全部对话。
- 2026-10-15 再次执行 `git status --short` 返回非 Git 仓库。不要假定存在分支、提交、远端或可回滚的 Git 快照，也不要擅自初始化仓库。

## 3. 用户目标、偏好和不可违反的规则

### 产品目标

本地优先的 AI 编程工具 Token 监控桌面应用；首个且目前唯一真实实现的 Provider 是 OpenAI Codex。监控每轮、会话、项目、模型和时间趋势，读取独立套餐额度，提供成本参考、多币别、缓存效率、规则预警和历史导出。

未来 Provider：Claude Code、DeepSeek、GLM / 智谱、Kimi。必须有独立适配能力声明；当前不能标成已支持，更不能提供伪造读数。

### 工作与安全约束

- 每次修改代码前阅读根目录 `AGENTS.md`；先检查现有结构、文档、锁文件和测试。
- 优先级：真实准确 > 稳定采集 > 体验 > 视觉 > 高级分析。
- 用户希望直接推进已授权工作，普通技术问题自主解决，避免反复确认；确实需要信息时可以询问。
- 不覆盖用户已有成果，不做无关重构。修改现有文件应小步、有针对性；备份旧发行文件。
- **禁止批量删除文件或目录**。不得使用 `del /s`、`rd /s`、`rmdir /s`、`Remove-Item -Recurse`、`rm -rf`。需要删除时只能逐个明确文件路径；批量清理交给用户。
- 不删除真实数据库来解决迁移问题，不删除或改写 Codex 日志。
- 不存储提示词、回复正文、代码内容、Cookie、访问令牌、API Key 或完整认证文件；不通过读取认证文件绕过官方接口。
- 个人用量不上传外部服务；汇率请求仅发送币种，额度使用正式 Codex App Server。
- 模拟 Fixture 必须独立数据目录且明显标为开发验证；不得混入真实账号统计。
- 不把“未知”替换为 0，不用套餐百分比反推 Token 余额。
- 测试、安装和稳定性结论必须标明版本、范围和是否实际运行。修改后同步文档。
- 当前协作规则不允许无授权主动派生子代理；不要因为任务较大就自动创建子聊天。

### 已确定的设计偏好

- 参考 Claude 桌面产品及用户提供的 Token Pulse 看板截图：暖白/暖灰、陶土橙强调、柔和圆角、克制阴影、清晰字体层级，支持深色主题。
- 保持响应快、键盘可用、窗口缩放可读、数字单位明确。
- 各导航职责去重；不要重新把相同看板塞进多个页面。
- 供应商图标来自本地矢量资源/组件；OpenAI 图标保持黑白，不统一染成陶土色。
- 不把模型生成速度误做每分钟输入加输出的消耗速率。

## 4. 当前功能与页面职责

当前五个导航：**总览、会话、时间线、预警、设置与数据源**。根目录 AGENTS 中的 Quota/Analytics 是原始推荐结构；已按用户要求合并额度导航，未实现完整 Analytics 页面。

| 页面/区域 | 当前职责与约束 |
| --- | --- |
| 总览 | Token 概况、缓存效率、365 天活跃热力图、Codex 栏、额度、成本参考、最近会话、模型分布、项目球群 |
| 会话 | 会话历史与独立会话明细、轮次和精确计量；不要重复完整时间线和导出面板 |
| 时间线 | 消耗趋势与分布、分钟/小时/天粒度、折线/柱状/K 线切换、峰值标记、计量事件表、当前筛选 CSV 导出 |
| 预警 | 真实本地规则记录、分页、类型/级别/已读/时间过滤；不是统计学异常检测，也不是系统通知 |
| 设置与数据源 | Provider 能力、隐私说明、健康状态、数据库路径/具体错误、预警规则、汇率设置 |
| 右上角账号入口 | 官方 account/read 返回的登录方式、邮箱和套餐；悬浮/键盘访问、刷新；缺少真实头像时用默认头像 |

### 不能回退的具体 UI 决策

1. 套餐额度只在总览展示，不恢复独立额度导航。
2. 完整“消耗趋势与分布”只保留在时间线。
3. “导出当前筛选的计量事件”只保留在时间线，不在总览和会话重复出现。
4. 导出必须先弹确认框，显示当时的筛选快照；取消不写文件，确认沿用该快照。
5. 输入/输出 Token 卡片使用相反方向的箭头，输出不再使用 OpenAI 供应商图标。
6. 项目/会话中的 OpenAI/GPT 供应商标识和其他位置保持一致；设置中的未来 Provider 显示各自图标，但仍标记未支持。
7. 账号资料卡已经移除“姓名/名称”和“用户名”两行。接口不提供头像、姓名、用户名时不猜测、不抓第三方头像。
8. 总览模型分布为环形与条形图展示；项目消耗为球群，不恢复为原有排行条形图。
9. 共享按钮有 hover、press、focus、disabled 及异步 busy 状态，并遵循减少动画设置。

### 项目球群（v0.5.14）

- `ProjectBubbles.tsx`：每个项目一个球，全部项目保留，密集时向下扩展，不只显示 Top N。
- 精确计数排序沿用 `distribution()` / BigInt；半径随相对消耗增大，保留最小可点击尺寸，**面积不是严格线性编码**。
- 柔和陶土、鼠尾草绿、蓝、紫等配色，缓慢漂浮；鼠标进入或键盘聚焦后暂停群体漂浮。
- 悬停、聚焦或点击时球体放大并提高层级；React Portal 浮层显示完整项目名、精确 Token 和占比。
- Esc、离开、滚动、窗口缩放关闭过期浮层；窄窗口和减少动画模式已考虑。
- 实时计数更新按项目名保留当前选中项，不因 Token 值变化或 ResizeObserver 回调立即清空。
- 测试持续漂浮元素时，Playwright `locator.hover()` 可能一直等待稳定；使用实际鼠标坐标移动，并用 `elementFromPoint` 验证真正命中顶层球体。

## 5. 技术栈与实际源码地图

Tauri 2、Rust、SQLite/rusqlite bundled、React 18 + TypeScript strict、Vite、Zustand、Tailwind/Radix Button 风格组件、模块化 ECharts。不要为了符合最初建议目录而大规模搬迁文件；当前 Rust 业务模块主要为扁平文件，不存在完整 `features/*` 目录结构。

| 文件/目录 | 作用 |
| --- | --- |
| `src/app/App.tsx` | 导航、筛选、看板组合、刷新编排、设置与健康状态 |
| `src/styles.css` | 双主题、布局、共享交互和球群样式 |
| `src/components/ActivityHeatmap.tsx` | 活跃日历与交互 |
| `src/components/Trend.tsx` | ECharts 趋势、三种图形和峰值 |
| `src/components/LiveRate.tsx` | 当前 UI 的生成速度估算；文件名沿用旧称，勿据此误认是总消耗速率 |
| `src/components/ProjectBubbles.tsx` | 项目球群 |
| `src/components/UsageDistribution.tsx` | 模型环形/分布展示 |
| `src/components/ProviderMark.tsx`、`src/lib/providers/` | 本地供应商标识 |
| `src/components/AccountAvatar.tsx` | 官方登录资料卡、默认头像与刷新 |
| `src/components/ExportUsage.tsx` | 当前筛选确认弹窗与导出交互 |
| `src/components/Alerts.tsx` | 预警查询、规则编辑、保存反馈 |
| `src/components/Cost.tsx` | 成本、币别、价格/汇率边界说明 |
| `src/lib/types.ts` | IPC 数据契约，大整数使用字符串 |
| `src/lib/alertRules.ts` | 阈值输入规范化、u64 范围校验 |
| `src/lib/distribution.ts` | 精确分布排序与比例展示辅助 |
| `src/lib/filters.ts`、`trendPeak.ts`、`format.ts`、`money.ts`、`monitor.ts` | 对应纯前端辅助与测试 |
| `src-tauri/src/desktop.rs` | Tauri Commands/Events、采集线程、应用数据库定位、健康报告 |
| `src-tauri/src/providers.rs` | ProviderAdapter、Codex 根目录发现、未来能力占位 |
| `src-tauri/src/ingest.rs` | JSONL 白名单提取、游标、锚点、快照持久化、受影响会话重算 |
| `src-tauri/src/domain.rs` | 增量、epoch、响应证据和未知语义 |
| `src-tauri/src/storage.rs`、`src-tauri/migrations/` | SQLite 打开、WAL、版本化迁移 |
| `src-tauri/src/services.rs` | 筛选、流式聚合、Dashboard、共用查询口径 |
| `src-tauri/src/collector.rs` | 采集状态、退避、重试时间与恢复次数 |
| `src-tauri/src/quota.rs` | Codex 程序发现、stdio RPC、额度与错误分类 |
| `src-tauri/src/account.rs` | 账号资料投影，不持久化邮箱 |
| `src-tauri/src/generation_speed.rs` | 单次响应生成速度估算与可靠性判定 |
| `src-tauri/src/usage_rate.rs` | 旧的近 60 秒消耗速率能力，仍保留，不用于冒充生成速度 |
| `src-tauri/src/visuals.rs` | 热力图和 OHLC 的后端聚合 |
| `src-tauri/src/finance.rs` | 精确成本参考、价格版本、汇率缓存 |
| `src-tauri/src/alerts.rs` | 规则、持久化预警、分页与已读状态 |
| `src-tauri/src/export.rs` | 完整筛选范围的流式 CSV/JSON 输出 |
| `tools/` | 独立验证/性能 crate；不进入桌面安装包 |
| `tests/` | Playwright + WebView2 原生桌面验证脚本 |
| `artifacts/windows/` | 便携 exe、历史版本、NSIS、实际安装验证目录和 BUILD_INFO |
| `.local/` | 独立验证数据库、Fixture、报告、截图和一次性诊断脚本；不要整体清理 |

常用 IPC：`read_dashboard`、`read_health`、`retry_ingest`、`refresh_quota`、`read_account`、`read_generation_speed`、导出和预警命令。事件包括 `usage-updated`、`ingest-health`、`quota-updated`、`alerts-updated`。完整签名以 `desktop.rs` 为准。

## 6. 核心数据口径：接手必须理解

### 采集、去重与恢复

- 只读 Codex 的 `sessions` 和 `archived_sessions` JSONL；这属于已验证版本的本地观测格式，不是永久稳定的公开协议保证。
- 正常每 3 秒检查追加，失败按 3/6/12/24/30 秒退避；手动重试通过容量 1 通道唤醒，不开并发导入。
- 文件首条 session_meta 决定线程所有者；继承的父会话元数据不能把文件归属切换走。
- 保存计量白名单字段，不保存整行正文。会话身份含 Provider、账户哈希、线程身份；项目只保存必要名称与路径哈希。
- 原始累计快照与计算增量分开。不能累加所有 total 或 last。
- 按稳定键去重，文件游标与快照事务提交；时间乱序和同时间大数有确定性排序；只重算受影响会话。
- 半行等待补齐，截断/轮转/锚点变化触发安全重读；重放不改变最终计量。
- 未知增量、继承累计历史、精确确认量分开。历史最后一条“未知本次用量”在 v0.2.0 已凭证据消除，但 **13,040,377 Token 的继承历史仍不能分配到事件**，不计入时间趋势或成本。
- 该数字只是历史验证截面，后续新来源仍可能产生未知；不能硬编码为永久 0 条未知。

### 数值、聚合和时间

- 输入包含缓存输入子集，输出包含推理输出子集；总 Token 通常为 input + output，不再额外加缓存/推理。
- Rust 使用 u64 计数和 checked u128 聚合；跨 IPC 用十进制字符串，前端不能将精确总量直接塞进 JS Number。
- 数据库存 UTC；界面本地时间。今天筛选与固定 UTC 五分钟预警窗口不是同一概念。
- Dashboard、趋势、成本和 CSV 复用同一筛选与计量集合。
- 趋势 minute/hour/day，长范围合并到最多 720 个桶；不通过丢弃事件减少总量。
- OHLC 表示桶内单条已确认计量增量的首/最大/最小/末值，不是金融价格，也不伪造请求数。
- 事件/轮次 UI 有最近 500 项上限，完整聚合和导出不受该列表上限影响。
- 长查询刷新采用串行合并，防止每 3 秒通知使尚未完成的结果不断被取消。

### 生成速度：用户已明确选择“估算，缺证据未知”

- 不是近一分钟总 Token 消耗，也不是同时累加所有活跃会话。
- 可见输出 = `output_tokens - reasoning_output_tokens`。
- 估算耗时 = `task_complete.duration_ms - time_to_first_token_ms`。
- tokens/s = 可见输出 × 1000 / 耗时毫秒；tokens/m = 可见输出 × 60000 / 耗时毫秒，两者分别从整数比率舍入，避免先舍入再乘 60。
- 仅接受有完整开始/结束证据、恰好一个响应、模型一致、无工具/压缩等干扰、计数匹配且耗时为正的样本。
- 取当前筛选最近完成轮次、限定近 15 分钟；最近轮次不合格时显示未知，不回退到更早的漂亮数字。
- 必须标注“估算”，不能声称精确解码速度或实时流式速度。

### 套餐、账户与成本

- 套餐额度与本地 Token 统计严格分离，不跟随本地项目/时间过滤、不换算 Token 余额。
- 官方 RPC：初始化后读 `account/rateLimits/read`；资料读 `account/read`，`refreshToken=false`。
- 已核实部分通知协议，但未证实可旁观其他客户端线程，因此用量仍依赖本地日志轮询。
- `usedPercent` 是已使用比例；剩余为 clamp(100-used, 0, 100)。保留多个 limit_id、窗口和重置时间。
- 额度约每分钟同步，RPC 总超时 20 秒；失败、未授权、不支持、过期不能展示伪造 0%。
- Codex 可执行文件优先显式 `TOKENSCOPE_CODEX_BIN`，其次 PATH，再找 Windows 桌面安装目录 `LOCALAPPDATA/OpenAI/Codex/bin` 下版本化 exe；显式非空错误路径不悄悄绕过，空覆盖值要回退。
- 创建 App Server 禁用 analytics，使用 stdin/stdout，退出清理子进程，不创建推理会话。
- 当前账户接口不提供真实头像、姓名、用户名；UI 保留邮箱/套餐，邮箱只在内存中，不写数据库或 localStorage。
- 成本是公开模型价格下的 API 等价参考估算，不是 Codex 套餐账单，也不保证实际 API 账单一致。
- 未知模型不给别的模型价格；显示覆盖率。使用版本化价格、精确整数金额，明确不含工具、长上下文/加速/地区等差价。
- USD/HKD/CNY；Frankfurter 仅查询币种，每日缓存、后台每小时检查。过期、失败、关闭时遵守既有暂停换算语义。
- 公开价和汇率是历史核实结果；涉及新定价或接口变更时重新查官方来源，不能把旧日期数据称作当前价格。

### 预警与导出

- 高用量规则按每会话 UTC 固定五分钟窗口，默认阈值 1,000,000；不是滚动五分钟，也不是统计学异常。
- v0.5.13 修复输入规范化与保存：空白/合法千位分隔归一化，正 u64 校验，保护草稿，保存后回读服务端值，避免后台刷新覆盖用户编辑。
- 本机恢复副本中的设置：预警启用、汇率启用、阈值 **10,000,000**。这不是产品默认值，也不保证未来未被用户修改。
- 额度阈值历史实现为 80% 警告、95% 严重；不对未知/过期/重置窗口触发。
- 预警幂等持久化，已读跨重启保留，严重升级可重新提示；每页 50 条，提供筛选。
- CSV 全范围流式写入，输出精确整数、UTC 时间、未知留空，并附口径 JSON；继承历史单独说明。
- 导出仅写应用 exports 目录，每次新报告，不覆盖老报告；导出按钮只在时间线且有确认弹窗。

## 7. 环境、运行与测试命令

### 路径与环境变量

| 项目 | 当前约定 |
| --- | --- |
| 系统 | Windows，PowerShell，时区 Asia/Shanghai |
| 工程要求 | Node >=20.18、Rust stable、MSVC 工具链、WebView2 |
| Codex 数据 | `CODEX_HOME`，默认 `%USERPROFILE%\.codex` |
| 数据库 | 通常 `C:\Users\stx\AppData\Roaming\local.tokenscope.desktop\tokenscope.sqlite3`；以运行实例 read_health 为准 |
| 覆盖数据库目录 | `TOKENSCOPE_DATA_DIR` 必须绝对路径 |
| 覆盖 Codex exe | `TOKENSCOPE_CODEX_BIN` |
| 桌面测试程序 | `TOKENSCOPE_TEST_EXE` 指向要测试的实际 exe |
| Fixture 模式 | `TOKENSCOPE_TEST_MODE=1`，必须配合隔离数据库和来源 |
| 自动化 WebView | 每任务独立 `WEBVIEW2_USER_DATA_FOLDER` 与本地调试端口 |

不要把本机曾安装的 Codex 0.153.4 / 0.162.0-alpha.2 或 Node 20.18 当成接手时必然仍然相同的版本；需要时重新检查。

常用命令（在项目根目录；执行前仍需读 package.json / Cargo 清单）：

```powershell
npm run desktop                 # Tauri 开发模式
npm run dev                     # 仅浏览器 UI；无真实 Tauri 后端
npm run typecheck
npm test
npm run build
npx prettier --check src
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features desktop -- -D warnings
cargo clippy --manifest-path tools/Cargo.toml --all-targets -- -D warnings
npm run build:desktop           # 正式 Release + NSIS

$env:TOKENSCOPE_TEST_EXE = (Resolve-Path 'artifacts/windows/installed-v0515/tokenscope.exe').Path
npm run test:desktop            # 空来源、恢复、数据库阻塞
npm run test:desktop -- --real   # 另含真实数据场景；会访问本地真实数据与官方额度
npm run test:alerts:desktop
npm run test:reports:desktop
npm run test:quota:shell
npm run test:account:desktop
```

验证工具入口在 `tools/src/verify.rs`，不是 `tools/src/bin/verify.rs`，也不在桌面 crate 的 bin 里：

```powershell
cargo run --manifest-path tools/Cargo.toml --bin verify -- .local/NEW-verification.sqlite3 --quota --fx
npm run test:soak -- 30 9231
# 百万测试先阅读 README、测试脚本和 VALIDATION 中的路径要求。
# benchmark / ingest_benchmark 通常要求新路径，禁止覆盖既有测试库。
```

注意：测试脚本会创建独立 Fixture/报告/浏览器配置目录；不要批量删它们。正式 exe 不依赖 Python；部分验证脚本用 Python 标准库 sqlite3。本机曾使用 `D:\anaconda3\python.exe`。

## 8. 验证证据与边界

### 最近 v0.5.15 工作中实际执行

- 17 项前端测试通过；58 项 Rust 测试通过（48 单元 + 10 集成）。这是本轮诊断改动的测试记录，并不表示每次仅版本号修改都重跑全部测试。
- TypeScript/Vite、Prettier、Rust fmt、desktop clippy `-D warnings` 通过。
- 正式 NSIS 构建成功，独立安装目录 `artifacts/windows/installed-v0515`，安装退出码 0。
- 实际安装版 empty/recovery/blocked 三个桌面场景通过，前端运行错误 0；新增验证阻塞时返回真实数据库路径与错误，恢复后 last_error 清空。
- 恢复后通过真实 Explorer 启动默认配置，collector/quota 均 synced、150 日志文件、读取/解析错误 0。
- 最后直接资源管理器启动正式 exe 成功；不将当时 PID、Token 数和额度百分比写成长期事实。
- 未在 v0.5.15 重跑百万性能、长期耐久或其他平台；独立 CSV/账号/额度专项不应视为本轮全部重跑。
- Vite 仍有约 500 kB 图表 chunk 提示，构建通过；安装包未签名。

### 历史验收不等于当前版本无限保证

- v0.5.14：项目球群真实数据、浅/深色、860px 窄窗口、实际 hover 命中置顶、键盘/Esc、实时更新，以及规则保存回归已验证。
- v0.2.0 百万真实格式合成事件：100 会话 × 10,000 事件，首次导入约 74.6 秒；重启重放约 3 ms、0 新增；补全半行追加约 108 ms。
- 历史百万全范围查询 p50 约 4.62 秒、p95 约 5.25 秒；安装版 IPC 查询约 4.50 秒，导航仍可响应。全范围仍 O(N)，不是预聚合架构。
- 历史 30 分钟耐久约 1802 秒、506 次追加、5 次重启、10 次来源断开恢复，计数一致、前端错误 0。
- 上述测试是当时构建、本机、特定数据分布；不能推断数周稳定、单个百万事件会话追加性能或其他机器表现。
- 更详细的原始数字、失败后修复记录和限定条件见 `docs/VALIDATION.md`。

## 9. 2026-10-15 双击启动故障：必须保留的交接信息

### 现象与初期误判风险

用户从文件夹双击 `artifacts/windows/TokenScope.exe` 后，总览、会话、时间线、预警、汇率都读不到数据，顶部“采集异常·将重试”，额度“服务请求失败”。不能仅把它当作 Codex 登录或网络问题。

初期独立工具检查默认路径 SQLite 正常，隔离/脚本启动也能采集；曾观察到一次额度成功、一次 Codex Rejected。**这些检查不足以覆盖用户真实 Explorer 失败环境**。

### 取证结论

1. 失败进程确由 Explorer 启动、用户为 stx，路径类环境变量正常。
2. 原代码吞掉具体采集错误，因此新增 read_health.database_path / last_error 和设置页显示。
3. 用户反馈：`打开数据库失败：malformed database schema (...)`，括号中出现会话标识而非正常表名。
4. 临时取证构建在失败进程内读取并导出其看到的数据库；导出副本在独立 Python SQLite 下也复现结构错误，确认不是单纯 UI 错报。
5. 同一绝对路径在不同启动环境下读到不同内容：

| 内容 | 字节数/指纹 |
| --- | --- |
| Explorer 失败实例看到的损坏内容 | 7,143,424 字节；SHA-256 `65d99109f4bd2cd82ce0d22c0bb9aa425ab5b8bdbce00e2e76268c48f178e941` |
| 当时独立工具看到的正常内容 | 8,077,312 字节；SHA-256 `eb4cc5eca8f2dfd4cb987eefcf2013312f2d660b78c2d23bd536501f1c590c11`；完整性正常 |
| SQLite backup API 生成的恢复源 | SHA-256 `bcf94229177ae96ab22657b77cf8cde780303405d7e451da349c675e93385fe3`；Schema v6、3449 usage_events、integrity_check=ok |

这些指纹仅标识当时文件；正常采集后数据库会变化。**尚未证明具体为何出现不同文件视图，也未定位最初的损坏写入**。不能擅自断言是 UAC、杀毒软件、沙箱、磁盘或 SQLite bug。

### 实际恢复步骤（历史记录，不是可直接重跑的操作指南）

- 停止明确路径的 TokenScope 实例，未批量杀其他程序。
- 生成并校验正常恢复源：`.local/startup-recovery-20261015.sqlite3`。
- 通过真实 Explorer 启动一次性 Python 恢复脚本，核对损坏文件和恢复源的预期指纹；检查无活动 WAL/journal。
- 先完整备份真实损坏文件，再创建恢复暂存文件并原子替换，之后再 integrity_check。
- 真实损坏文件备份：
  `C:\Users\stx\AppData\Roaming\local.tokenscope.desktop\tokenscope.pre-recovery-20261015-132851.sqlite3.bak`
- 恢复结果：`.local/explorer-recovery-result.json`；status=restored，3449 个源事件。
- 恢复源保留 `fx_enabled=true`、`alerts_enabled=true`、`usage_alert_threshold=10000000`。
- 原始 Codex 日志没有修改，后续继续增量采集/重放。
- 恢复后真实 Explorer 启动采集与额度均成功。

**严谨边界**：没有完成坏文件逐表取证，不能证明损坏副本绝对不存在正常副本未涵盖的设置、已读状态或历史记录；完整坏文件已保留，必要时继续比较。不要把“保留备份并恢复”夸大为已证明任何数据都零损失。

### 保留的诊断产物

- `.local/startup-diagnosis-20261015.sqlite3`：初期独立诊断副本。
- `.local/startup-recovery-20261015.sqlite3`：校验通过的恢复源。
- `.local/recover-explorer-database.py` / `.cmd`：一次性、带固定指纹与路径的恢复脚本，**不要直接重跑**。
- `.local/explorer-recovery-result.json`：实际恢复记录。
- `.local/diagnose-startup.mjs`、`diagnose-default-startup.mjs`、`read-*-health.mjs`、`read-startup-env.py` 等：临时诊断，不属于产品 API 或常规测试入口。
- `%TEMP%\tokenscope-startup-36372.sqlite3` 及同名 JSON：失败进程的取证副本；临时目录文件可能被系统清理，以持久备份为准。
- `artifacts/windows/TokenScope_0.5.14-original.exe`：原版备份。
- `artifacts/windows/TokenScope-startup-capture.exe`：临时取证程序，**不要分发或当日常应用**。
- `BUILD_INFO_0.5.14-diagnostic.json` 属于中间诊断构建，不代表当前正式 exe。

### v0.5.15 产品代码实际变化

- `desktop.rs`：采集保留 last_error；目录创建/数据库打开阶段标明；read_health 返回实际 database_path 和 last_error；成功后清除错误。
- `types.ts` / `App.tsx`：设置页展示实际路径及具体原因。
- `tests/desktop-smoke.mjs`：阻塞场景验证错误路径与详情，恢复后错误为空。
- 正式源码已移除临时数据库读取/指纹/复制逻辑，不会遇错自动复制整个库、删除数据库或自动恢复。
- Schema 仍为 6，计量算法未变。版本更新和文档同步已完成。

## 10. 正式交付物与打包注意事项

| 产物 | 位置/说明 |
| --- | --- |
| 日常便携程序 | `artifacts/windows/TokenScope.exe`，当前 v0.5.15 |
| 安装包 | `artifacts/windows/TokenScope_0.5.15_x64-setup.exe` |
| 实际安装验证 | `artifacts/windows/installed-v0515/tokenscope.exe` |
| 发布记录 | `artifacts/windows/BUILD_INFO_0.5.15.json` |

最终便携/实际安装 exe SHA-256：
`A39E73D530C0766914361961ADFE1EC370C8974E925012D0C4680322B84286B3`

安装包 SHA-256：
`646524A147489EA7DEFCBF5B50730F4D483FB0BA6FF447D2C328402E5E53DED6`

不要只凭文件名判断版本，核对 BUILD_INFO 和哈希。实际构建目录 exe 与安装后 exe 曾出现不同指纹，因此最终将**实际安装并验证的 exe**复制到便携路径；不能把两者未经核对视为同一二进制。

版本涉及 package.json/package-lock、桌面 Cargo.toml/Cargo.lock、tools Cargo.toml/Cargo.lock、tauri.conf.json、App footer 和 quota initialize.clientInfo；更新时针对这些文件，别全仓库替换历史文档版本号。

发布时确认没有运行中的目标 exe 占用；退出后可能需稍等才可替换。只关闭确认过路径的目标实例，保留旧文件备份，不递归清理安装验证目录。启动后台 helper 使用隐藏窗口；用户要看到的主 GUI 可正常显示。

## 11. 已知限制与下一步优先级

### 优先级 1：把本次恢复提升为可重复验证的稳定性保障

- 再次确认用户后续真正双击、关闭重开仍正常，尤其在机器/环境重启后。
- 追踪不同文件视图和原始损坏原因；单次工具进程正常不能代替用户进程证据。
- 设计只读、脱敏、受控大小的诊断导出；不能重新引入每次失败复制整个数据库。
- 如需自动备份/恢复，先制定明确保留策略、完整性检查、并发/锁与 WAL 一致性、失败回滚和用户可审查恢复范围；当前没有该产品能力。
- 补充磁盘满、不可写、突发退出、多实例和重启后的验证，避免以删库“修好”。

### 优先级 2：Phase 4 发布质量

- 托盘常驻及可关闭的系统通知是此前路线图下一项，但尚未实现；先保证稳定性，再由用户任务决定推进。
- 签名、卸载流程、跨平台、数日/数周耐久和更多无障碍验证未完成。
- 全范围仍 O(N)，超大范围可考虑预聚合；单会话超大日志需独立基准，不能直接套用 100 会话百万事件结果。

### 后续能力

- 统计学异常检测与高级 Analytics 未实现。
- Claude Code / DeepSeek / GLM / Kimi 仍只占位，接入前逐一验证真实数据与能力。
- 自动更新、开机启动、云同步、团队权限和账号托管不是当前已交付能力；不要默认增加。

## 12. 阅读顺序与防遗忘清单

建议顺序：

1. 根目录 `AGENTS.md`：强制约束。
2. 本文件：当前交接快照。
3. `README.md`、`docs/ROADMAP.md`：运行方式与阶段进度。
4. `docs/VALIDATION.md`：按日期/版本看，**v0.5.15 记录追加在文件末尾**，不是只读文件头。
5. `docs/DATA_MODEL.md`、`DATA_SOURCES.md`、`ARCHITECTURE.md`、`DECISIONS.md`：口径和历史取舍。
6. 当前任务涉及的源码、测试、锁文件和 BUILD_INFO。

老文档保留历史段落：例如“尚未实现成本/百万测试”“仍有一条未知基线”“账号姓名显示未提供”等可能只适用于旧版本。以较新明确决定和源码为准；不要只看到一个关键词就回退已完成工作。

新聊天开始时核对：

- [ ] 仍是正确工作目录、版本和 Schema；是否出现新的 AGENTS 或用户改动？
- [ ] 是否真有 Git？没有就不假定可用 git diff/reset。
- [ ] 当前打开的是正式 exe 还是 diagnostic/capture/旧安装版本？
- [ ] 新任务是否要求功能开发，还是只要求解释/检查？不自行扩大范围。
- [ ] 修改统计前是否理解缓存/推理子集、未知历史与估算速度？
- [ ] 处理启动故障时是否取得真实失败进程的路径与错误，而非仅工具自己的检查？
- [ ] 未把历史测试数字冒充本轮结果，未把本机恢复宣称为通用自动修复？
- [ ] 结束时说明完成内容、主要文件、实际验证、限制和一个明确下一步。

补充：聊天中曾有一次 Java `/user/login` 的 `@NotBlank` 用于 Integer 的问答，用户当时要求只解释、不修改。那是旁支问题，与 TokenScope Rust/React 工程无关，不要在本项目里寻找或修复 Java 登录接口。
