# v0.5.14 项目悬浮气泡验收 — 2026-10-13

ProjectBubbles 将当前筛选的每个项目绘制为立体气泡，消费较高的球更大，小球有最小操作尺寸，面积并非严格线性比例。全部项目保留，密集时纵向扩展。鼠标悬停/点击/键盘聚焦放大并提高气泡及卡片层级，页面 Portal 展示完整名称、精确 Token 与占比；Esc、离开、滚动/窗口缩放关闭浮层。鼠标进入区域暂停漂浮，减少动态效果设置关闭动画。沿用 Rust 聚合、BigInt 总量/排序，Schema v6 不变。

实际执行：npm test 17 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test 48 单元 + 10 集成，共 58 通过。npm run build:desktop 成功，最终 NSIS v0.5.14 安装至 artifacts/windows/installed-v0514-final，退出 0。

最终安装版 test:alerts:desktop 验证气泡数等于项目数、鼠标悬停、实际命中置顶球、键盘聚焦、Esc，以及阈值保存/重启回归通过。test:desktop -- --real 真实、空来源、来源恢复、数据库首次阻塞四场景通过，包含真实项目悬停详情；前端错误均为 0。真实 17 项目气泡群和放大置顶截图已目视检查，浅色/深色/窄窗口回归通过。测试改用实际坐标鼠标移动以操作持续漂浮的元素；测试发现并修复实时计数更新或布局高度通知清除悬停状态的问题。

本轮未重跑独立 CSV/账号/额度 shell 专项、百万性能或长期耐久；Windows 包未签名，其他平台未验证，既有 charts 500.23 kB 提示非阻塞。下一步继续发布稳定性验证。

---
# v0.5.13 规则保存与总览图表验收 — 2026-10-13

规则编辑此前使用原始数字字符串与 HTML pattern，带逗号输入无法提交；保存仅依赖预警事件刷新，缺少回读确认，保存错误还会被后台读取清除。本次加入规范千位逗号/空白归一化、精确 u64 正整数校验、独立草稿、保存防重入和读取版本保护。保存后回读数据库校验阈值与开关，显示实际保存值；读取错误与保存错误分开处理。提高阈值不删除已经触发的历史预警。

UsageDistribution 使用 Rust 已返回的模型/项目聚合，BigInt 排序、求总量和计算比例，模型显示环形占比与条形列表，项目显示横向排行，前 8 项及展开全部；精确数值保留在提示，绘图百分比最多两位小数。按钮统一圆角、悬停、按压、禁用、焦点与忙碌语义，减少动态效果设置有效。后端计量与 Schema v6 不变。

实际执行：

- npm test：17 通过，新增阈值格式/范围及大整数分布、空值/零值测试。typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test：48 单元 + 10 集成，共 58 通过。
- npm run build:desktop 成功，最终 NSIS v0.5.13 安装到 artifacts/windows/installed-v0513-final，退出 0。既有 charts 500.23 kB 提示非阻塞。
- 最终安装版 test:alerts:desktop：单独修改阈值、带逗号输入、后台刷新保留草稿、明确保存确认、切页回读、无效值拒绝、禁用规则与跨重启持久化通过；分布图总量与 Rust IPC 一致。只读检查隔离测试数据库确认预警 detail 使用新阈值 1,250,000，证明计算器实际应用新设置。首次测试重启连接到尚未退出的 WebView 调试端口，测试改为每次重启独立端口和 WebView 目录（同一数据库），最终通过。
- 最终安装版 test:reports:desktop：607 条 CSV、筛选、确认/取消/Esc/键盘、分页及不覆盖通过；test:account:desktop：账号、供应商图标、键盘及窄窗口通过。
- 最终安装版 test:desktop -- --real：真实、空来源、来源恢复、数据库首次阻塞四场景通过；所有通过运行前端错误 0。总览浅色/深色、窄窗口截图已目视检查，图表清晰且窗口无水平溢出。

一次测试命令的自动审批超时，重试后正常执行。本轮未重跑百万性能、长期耐久或独立额度 shell 专项；Windows 包未签名，其他平台未验证。下一步继续 Phase 4 发布质量与测试稳定性。

---
# v0.5.12 图标方向与配色统一验收 — 2026-10-13

输出 Token 使用 ArrowDownLeft，与输入的 ArrowUpRight 相反。会话入口 OpenAI 标识统一为与模型标签相同的 20 px，使用中性色；移除 Provider SVG 强调色覆盖，OpenAI 标识随主题显示黑白色系。其他品牌资源与未支持状态不变，后端计量和 Schema v6 不变。

实际执行：npm test 14 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test 48 单元 + 10 集成，共 58 通过。npm run build:desktop 成功，NSIS v0.5.12 安装至 artifacts/windows/installed-v0512，退出 0。

最终安装版 npm run test:account:desktop 验证 OpenAI 标识继承中性颜色、其他品牌图标和账号交互回归通过。npm run test:desktop -- --real 首次运行在实时采集过程中发生 RPC 快照与卡片更新的时间差（257,117,458 / 257,240,077），未改动计量代码，重跑后真实、空来源、来源恢复、数据库首次阻塞四场景通过；新检查确认相反箭头以及会话/模型图标尺寸、计算颜色一致。通过运行的前端错误均为 0。实际总览与 Provider 浅色/深色截图已目视检查。

真实数据测试的静态快照断言在持续追加时仍有竞态风险。本轮未重跑 CSV、独立预警、额度 shell 专项、百万性能或长期耐久；Windows 包未签名，其他平台未验证，既有 charts 500.23 kB 提示非阻塞。下一步继续发布质量及真实数据测试稳定性。

---
# v0.5.11 Provider 品牌标识验收 — 2026-10-13

Provider 能力列表新增官方 Claude、DeepSeek、智谱及 Kimi 本地品牌资源；原始来源见 src/assets/providers/README.md。Kimi 浅色/深色变体随主题切换，各资源保持原比例。统一前端占位 Claude Code ID 为后端的 claude-code。图标不改变未支持状态，也不新增任何采集或账号能力；Schema v6 不变。

实际执行：npm test 14 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test 48 单元 + 10 集成，共 58 通过。资源来源文档的格式提示已修复，最终 Prettier 检查通过。npm run build:desktop 成功，NSIS v0.5.11 安装至 artifacts/windows/installed-v0511，退出 0。

最终安装版 npm run test:account:desktop：四种供应商图片实际加载、四项未支持状态、主题切换、账号字段及交互回归通过，前端错误 0；Provider 卡片浅色/深色截图已目视检查。npm run test:desktop -- --real：真实、空来源、来源恢复、数据库首次阻塞四场景通过，前端错误 0。

本轮未重跑 CSV、独立预警、额度 shell 专项、百万性能或长期耐久；Windows 包未签名，其他平台未验证，既有 charts 500.23 kB 提示非阻塞。下一步继续 Phase 4 发布质量验证。

---
# v0.5.10 账号资料精简与供应商图标验收 — 2026-10-13

移除账号资料中的姓名 / 名称、用户名两行，保留登录方式、邮箱、套餐和默认头像说明。移除界面全部 Terminal 图标；工作空间、输出 Token、会话入口和 Codex 数据源设置统一复用可配置大小的 OpenAI ProviderMark。设置中尚未实现的适配器保留未知图标与未支持状态，模型图标的已验证前缀判断不变。后端采集、统计与 Schema v6 不变。

实际执行：npm test 14 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test 48 单元 + 10 集成，共 58 通过。最初的图标组件属性类型不兼容已修正，最终类型检查和构建通过。

npm run build:desktop 成功；NSIS v0.5.10 安装到 artifacts/windows/installed-v0510，退出 0。最终安装版 npm run test:account:desktop 验证账号字段精简、工作空间 OpenAI 图标、设置中一个已实现/四个未支持标识、无终端图标及悬浮/键盘/账号切换/失败清理等行为通过。npm run test:desktop -- --real 的真实、空来源、来源恢复、数据库首次阻塞四场景通过。两组前端错误均为 0；合成账号浅色/深色截图及真实总览已目视检查。

本轮未重跑 CSV、独立预警、额度 shell 专项、百万性能或长期耐久。Windows 安装包未签名，其他平台未验证；既有 charts 500.23 kB 分块提示非阻塞。下一步继续 Phase 4 发布质量验证。

---
# v0.5.9 登录账号资料卡验收 — 2026-10-13

右上角支持鼠标悬浮、点击及键盘访问 Codex 账号资料，支持手动刷新和定期刷新。复用 App Server stdio 通道调用 account/read（refreshToken: false），只返回允许的登录方式、邮箱、套餐和读取状态。资料仅存在内存，刷新或失败清除旧资料，不读取认证文件、不持久化邮箱、不请求外部头像服务。Schema v6 不变。

当前实测 Codex 0.162.0-alpha.17.2 的账号字段仅有 type、email、planType；头像、姓名和用户名无正式来源，界面明确显示默认头像与“未提供”。这里对应 TokenScope 启动的 Codex CLI 环境，其他 CODEX_HOME 的登录可能不同。

实际执行：

- Rust 48 单元 + 10 集成，共 58 通过；npm test 14 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过。
- npm run build:desktop 成功；最终 NSIS v0.5.9 安装至 artifacts/windows/installed-v059-final，退出 0。
- 最终安装版 npm run test:account:desktop：独立 TEST_ONLY Rust stdio fixture 覆盖无 CLI、悬浮/点击/键盘/Esc、真实协议形状的合成账号、切换账号、API Key、退出登录、RPC 失败清除旧邮箱、无浏览器持久化和窄窗口；前端错误 0。浅色/深色截图仅含合成邮箱，已目视检查。修复刷新按钮禁用时焦点丢失导致弹层关闭的问题。
- 最终安装版 npm run test:quota:shell：真实 account/read 邮箱与界面一致（仅输出比较结果），空覆盖变量及无 Codex PATH 的 shell 启动仍同步真实额度；前端错误 0。未将真实邮箱写入截图或日志。
- 最终安装版 npm run test:desktop -- --real：真实、空来源、来源恢复、数据库首次阻塞四场景通过，前端错误 0；原有计量、生成速度、热力图、峰值、筛选及主题回归通过。

真实头像/姓名/用户名仍需未来官方接口提供；本轮未重跑 CSV、独立预警、百万性能或长期耐久。Windows 安装包未签名，其他平台未验证；既有 charts 500.23 kB 分块提示为非阻塞。下一步为 Phase 4 发布质量与稳定性验证。

---
# v0.5.8 GUI 风格应用图标验收 — 2026-10-11

source.svg 保留 TokenScope 脉冲线，换为暖白 #faf9f5 底板、陶土橙 #b45c40 线条和中性 #dedbd3 边框，圆角外角透明。侧栏及 favicon 复用同一 SVG；bundle.icon 明确指定 PNG、ICO 和 ICNS。旧资源备份到 .local/ICON_BACKUP_0.5.7_*；采集、计量及 Schema v6 不变。

实际执行：

- npm run tauri -- icon src-tauri/icons/source.svg --output src-tauri/icons --ios-color '#faf9f5' 成功生成资源；ICO 二进制目录检查确认包含 16、24、32、48、64、256 px 的 32 位图层。生成器对系统 mstmc.ttf 有非阻塞解析提示，图标不使用字体。
- npm test 14 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test 45 单元 + 10 集成，共 55 通过。
- npm run build:desktop 成功，Vite favicon 已转换为实际构建资产引用；NSIS v0.5.8 安装至 artifacts/windows/installed-v058，退出 0。charts 500.23 kB 的既有提示为非阻塞。
- 使用 Windows ExtractAssociatedIcon 从最终安装 exe 读取的实际图标与新资源一致，32 px 和主图已目视检查；安装版的浅色、深色 GUI 截图均确认侧栏图标正常显示。
- 最终安装版 npm run test:desktop -- --real：真实、空来源、来源恢复、数据库首次阻塞四场景通过，前端错误 0；原有真实计量、生成速度、筛选、主题与窄窗口回归通过。

其他平台仅生成图标资源，未验证安装；本轮未重跑 CSV/独立预警/额度 shell 专项、百万性能或长期耐久。Windows 安装包未签名。下一步仍为真实流式速度可观测性与发布质量。

---
# v0.5.7 导出入口集中验收 — 2026-10-11

App.tsx 仅在时间线页渲染 ExportUsage，总览和会话页移除导出卡片。导出组件、筛选口径、后端计量和 Schema v6 不变。README 与 ROADMAP 已同步。

实际执行：npm test 14 通过；typecheck、Prettier、core fmt、core/tools clippy（all-targets、core desktop、-D warnings）通过；cargo test 45 单元 + 10 集成，共 55 通过；npm run build:desktop 成功，NSIS v0.5.7 安装至 artifacts/windows/installed-v057，退出 0。

最终安装版 npm run test:reports:desktop 验证总览与会话导出按钮均为 0、时间线为 1；确认/取消/Esc/焦点循环和模态阻止导航、607 条完整 CSV、303 条筛选 CSV、分页、不覆盖及窄窗口回归通过。npm run test:desktop -- --real 的真实、空来源、来源恢复、数据库首次阻塞四场景通过。两组前端错误均为 0。

本轮未重跑独立预警/额度 shell 专项、百万性能或长期耐久。既有 charts 500.23 kB 分块提示为非阻塞；安装包未签名。下一步仍为真实流式速度可观测性与发布质量。

---
# v0.5.6 模型生成速度估算验收 — 2026-10-11

按用户确认，将总览速率改为单次响应的可见输出生成速度估算。分子为 output − reasoning，分母为 duration_ms − time_to_first_token_ms；每秒和每分钟分别从同一整数比率换算并四舍五入到两位小数。仅接受无工具、单响应、模型一致且计量证据与增量一致的完成轮次。当前筛选选最近完成的一轮，限制近 15 分钟；不满足条件显示未知，不汇总并发速度，也不以输入 + 输出消耗速率替代。

Schema v6 无损增加 generation_samples 和近期/会话轮次索引。游标版本 3 只读回填一次，计量计数不变；采集事务保存游标和脱敏时间样本。查询从近期完成样本索引选取，不每 3 秒扫描全部计量历史。只保留时间、计数及响应哈希，无正文/工具参数。

实际执行：

- cargo test --manifest-path src-tauri/Cargo.toml：45 单元 + 10 集成，共 55 通过。新增精确比率及大整数、单响应与子集口径、无效/缺失耗时、工具、多响应、未知计量、混合模型、筛选、未来/过期、重放/重启/旧游标回填与部分完成行的断点续读测试；既有迁移与计量回归通过。
- npm test：14 通过；npm run typecheck、Prettier 和 core fmt 检查通过；core clippy（all-targets、desktop、-D warnings）及 tools clippy（all-targets、-D warnings）通过。实现中发现的格式与 Clippy 提示已修复。
- npm run build:desktop 成功，Windows Release / NSIS v0.5.6 安装到 artifacts/windows/installed-v056，退出 0。charts 分块 500.23 kB 的既有 Vite 提示仍存在，非阻塞。
- 最终安装版 npm run test:alerts:desktop：独立 TEST_ONLY 日志的 120 输出 − 20 推理、3000 ms − 500 ms，界面精确显示 40.00 tokens/s 与 2,400.00 tokens/m；提示说明已核对。追加工具调用轮次后两种单位均显示未知。原消耗接口、预警、重放和跨重启回归通过，前端错误 0。截图已目视检查。
- 最终安装版 npm run test:desktop -- --real：真实、空来源、来源恢复、数据库首次阻塞四场景通过；真实日志生成速度显示与 Rust IPC 一致，估算标识和真实截图已目视检查。三种趋势峰值、筛选、额度、主题与窄窗口回归通过，前端错误 0。
- 最终安装版 npm run test:reports:desktop：完整 607 条 CSV、筛选、确认/取消/键盘、分页及不覆盖回归通过；npm run test:quota:shell：空覆盖变量/无 Codex PATH 启动仍同步真实额度；前端错误均为 0。

这是最近完成响应的估算平均速度，非实时解码测量；扣除首 Token 等待仍可能包含网络或收尾开销。缺少可靠源字段时不承诺数值，复杂工作轮次通常显示未知。首次升级回填会读历史日志一次；百万性能与长期耐久本轮未重跑。安装包未签名，其他平台/卸载未验证。Schema v6 不能由旧版打开，升级后继续使用新版。下一步是验证独立流式计量的可观测性与发布质量。

---
# v0.5.5 时间线峰值标记验收 — 2026-10-11

消耗趋势与分布新增峰值圆点、数值标签及精确值/时间桶范围提示。折线和柱状取当前筛选中最高桶总量；K 线取最高单条已确认增量。直接复用 Rust 返回的聚合数据，用 BigInt 比较，避免大整数取峰精度损失；同值保留最早桶，空数据和全零不标峰。计量算法及 Schema v5 不变。

实际执行：

- npm test：14 通过，新增大整数、并列峰、K 线独立口径、零值/缺失值和筛选变化测试；npm run typecheck、npx prettier --check src 通过。
- cargo test --manifest-path src-tauri/Cargo.toml：40 单元 + 10 集成，共 50 通过；core fmt、core clippy（all-targets、desktop、-D warnings）与 tools clippy（all-targets、-D warnings）通过。
- npm run build:desktop 成功，NSIS v0.5.5 安装到 artifacts/windows/installed-v055，退出 0。Vite charts 分块为 500.23 kB，有超过默认 500 kB 的非阻塞提示。
- 最终安装版 npm run test:desktop -- --real：真实、空数据、来源恢复和数据库首次阻塞四场景通过，前端错误 0；三种模式峰值及索引均与 Rust IPC 聚合结果精确一致。三种图表截图已目视检查，峰值圆点与标签清晰可见；原有筛选、主题、窄窗口回归通过。
- 最终安装版 npm run test:reports:desktop：607 条 CSV、筛选导出、确认/取消/Esc/键盘、分页及不覆盖回归通过；npm run test:alerts:desktop：预警、精确速率、重放及跨重启通过；npm run test:quota:shell：空覆盖变量与无 Codex PATH 的 shell 启动仍同步真实额度。三组前端错误均为 0。

峰值属于当前筛选与图表有效粒度；合并时间桶后随桶口径变化。图形坐标仍使用 Number 近似绘制，取峰与精确提示保留整数语义。本轮未重跑百万性能、长期耐久或独立全历史校验；安装包未签名，其他平台未验证。下一步仍为 Phase 4 发布质量。

---
# v0.5.4 Token 消耗速率验收 — 2026-10-11

在总览 Codex 信息栏加入 tokens/s 和 tokens/m。Rust usage_rate.rs 复用全局筛选口径与现有时间索引，流式读取 (now-60s, now] 内已确认输入 + 输出增量，未知记录排除并提示；tokens/m 为该分钟总量，tokens/s 为除以 60 后两位小数的平均消耗。u128 累加、整数运算与字符串 IPC，缓存与推理子集不重复计数。LiveRate 每 3 秒串行刷新，过滤/卸载丢弃旧请求；采集未就绪、异常或过期、查询失败显示 —。空窗口在采集正常时为真实零。它不是模型生成速度，不根据单次调用完成事件虚构流式速度。Schema 仍为 v5。

实际执行：

- cargo test --manifest-path src-tauri/Cargo.toml：40 单元 + 10 集成，共 50 通过。新增窗口首末边界、未来事件排除、未知排除、子集不重复计数、模型/项目/历史时间筛选、无新事件的窗口过期归零及 u64::MAX 合并后的精确 u128 / 两位小数。
- core cargo fmt --check、core clippy（all-targets、desktop、-D warnings）、tools clippy（all-targets、-D warnings）通过；npm test 10 通过，typecheck 和 Prettier 通过。
- npm run build:desktop：Vite、Windows Release / NSIS v0.5.4 成功；最终安装到 artifacts/windows/installed-v054，退出 0。
- 最终安装版 npm run test:alerts:desktop：隔离 Fixture 2000000 Token 实际展示 33,333.33 tokens/s / 2,000,000 tokens/m，截图已目视检查；预警生成/重放/规则与已读跨重启回归通过，前端错误 0。
- 最终安装版 npm run test:desktop -- --real：真实、空数据、来源恢复、数据库首次阻塞四场景通过，前端错误 0。真实速率栏两种单位及空来源的 —、860×640 无全局溢出、原有筛选/图表/热力图/额度/主题验证通过。
- 最终安装版 npm run test:reports:desktop：CSV 确认、取消、键盘、完整/筛选导出与分页回归通过；npm run test:quota:shell：空覆盖变量与无 Codex PATH 的 Windows shell 启动仍成功同步真实额度，前端错误 0。

未重跑百万性能、长期耐久、卸载或其他平台；实时窗口依据本地日志中的计量事件时间，受源日志写入和采集延迟影响，已确认值可能是未知记录存在时的下界。历史筛选不覆盖最近一分钟时速率为零，不代表全账户实时消耗。下一步仍为 Phase 4 发布质量。

---
# v0.5.3 桌面空配置修复验收 — 2026-10-11

用户截图确认 v0.5.2 仍启动失败。通过 PowerShell Start-Process 实际复现，补充底层错误后得到 program path has no file name：清空后的 TOKENSCOPE_CODEX_BIN 在 shell 子进程中仍以空值存在，被解析成显式空程序路径。原 Node 用例删除覆盖变量，因此上一轮成功只代表该测试环境，遗漏了桌面空值分支。

resolve_executable 将空值视为未配置，继续 PATH / Windows 桌面目录发现；非空无效覆盖仍明确失败。启动失败补充系统启动错误及可用系统代码，仅来自本机进程启动，不回显服务端 RPC 原始错误或认证内容。CSV 确认和 Schema v5 保留。

实际验证：

- cargo test --manifest-path src-tauri/Cargo.toml：39 单元 + 10 集成，共 49 通过；空 PathBuf 覆盖变量新增断言。core fmt、core clippy（all-targets、desktop、-D warnings）、tools clippy（all-targets、-D warnings）通过。
- npm test：10 通过；typecheck 与 Prettier 通过。npm run build:desktop 成功，NSIS v0.5.3 安装到 artifacts/windows/installed-v053，退出 0、注册版本 0.5.3。
- 最终安装版 npm run test:quota:shell：PowerShell 7 Start-Process、空覆盖变量、不含 Codex 的 PATH，真实官方额度接口成功，synced、1 个限额池，前端错误 0；截图已目视确认 5 小时和 7 天窗口为真实服务端返回。测试数据/快照写入独立目录；仅 Codex 自身管理登录，不读取认证文件。
- 已将经过验收的安装版复制为当前可运行程序并备份 v0.5.2；以同样空变量/无 Codex PATH 的方式启动可见窗口后，独立只读查询用户实际数据库，确认启动后产生新的官方额度快照。本轮同时验证测试版和当前实际运行程序。
- 最终安装版 npm run test:desktop -- --real：真实、空数据、来源恢复、数据库首次阻塞四场景通过，前端错误 0。
- 最终安装版 npm run test:reports:desktop：确认/取消/Esc/焦点循环、607 条完整 CSV、303 条项目/模型筛选、精确整数、未知/继承口径、不覆盖报告及预警分页通过，前端错误 0。
- 最终安装版 npm run test:alerts:desktop：生成、重放、已读/规则跨重启与非法规则拒绝通过，前端错误 0。

中间诊断构建曾因探查进程占用 release 可执行文件而构建失败；停止该确切探查进程后重新构建成功，最终验收均使用 v0.5.3 安装版。未删除历史数据库、日志或测试目录。未重跑百万性能/长期耐久；安装包未签名，其他安装布局和平台仍未验证。下一步：继续发布质量工作。

---
# v0.5.2 额度同步与 CSV 确认验收 — 2026-10-11

本机 Codex 桌面 CLI 已安装，但普通 Windows 用户/系统 PATH 未包含其目录；旧实现只启动 codex.exe。增加显式覆盖、PATH 与 LOCALAPPDATA/OpenAI/Codex/bin 发现，版本目录按可执行文件修改时间选择，显式配置失败不回退。错误分类提供可操作的脱敏提示，不读取认证文件、不返回服务端原始报错。空/格式异常 RPC 结果不能冒充同步成功。

CSV 导出新增原生模态确认框，展示捕获的项目、模型和时间范围以及文件与计量说明，确认后才调用现有异步导出；取消、Esc 或遮罩关闭不写文件。弹窗内焦点循环、初始取消焦点、关闭后恢复，以及模态期间导航快捷键限制已实现。Schema v5 与计量/导出算法不变。

实际执行：

- npm test：10 通过；npm run typecheck、npx prettier --check src、core cargo fmt --check 通过。
- cargo test --manifest-path src-tauri/Cargo.toml：39 单元 + 10 集成，共 49 通过。新增桌面无 PATH 发现、显式配置优先、PATH 优先及缺程序、RPC 错误脱敏/类型与异常结果测试。core clippy（all-targets、desktop、-D warnings）与 tools clippy（all-targets、-D warnings）通过。
- npm run build:desktop：Vite、Release 与 NSIS v0.5.2 成功，最终安装到 artifacts/windows/installed-v052-final，退出 0，注册版本 0.5.2，旧目录保留。
- 最终安装版 npm run test:desktop -- --real：四场景通过，前端错误 0。真实模式主动移除 Codex 注入的 PATH 和覆盖变量，额度仍成功同步；真实 Token 与 IPC 一致，热力图、图表、筛选、导航、主题、窄窗口与异常恢复回归通过。
- 最终安装版 npm run test:reports:desktop：确认框项目/模型范围、取消/Esc 零文件、焦点与 Shift+Tab 循环、恢复焦点、模态阻止导航通过。确认后 Python 独立解析完整 607 条 CSV，大整数、未知留空、继承分离、公式保护及 303 条项目/模型筛选导出正确，既有文件不覆盖；预警分页及窄窗口通过，前端错误 0。确认框截图已目视检查。首次 Shift+Tab 验收发现 WebView2 焦点边界不稳定，补充显式循环后通过。
- 最终安装版 npm run test:alerts:desktop：生成、重放、已读/规则跨重启及非法规则拒绝通过，前端错误 0。首次重启时测试在 CDP 可连接但页面尚未创建时失败；增加等待页面创建后复测通过。

本次未重跑独立全历史工具、百万性能、长期耐久、卸载、签名与跨平台验证。程序发现只验证当前 Windows 桌面安装布局；其他目录可使用覆盖变量。失效登录或服务端故障仍需在 Codex 中处理，应用展示真实失败状态。下一步：Phase 4 托盘与可选系统通知。

---
# v0.5.1 页面职责与导航验收 — 2026-10-09

总览保留核心指标、365 天活跃热力图、完整套餐额度、成本与摘要；完整消耗趋势及粒度选择集中到时间线。移除独立额度导航，将额度口径说明折叠到总览额度面板。总览的时间线入口保留模型、项目及时间筛选；五个导航对应 Alt+1 至 Alt+5。时间线事件表局部滚动并固定表头，避免 500 条事件撑长整页。计量、聚合与数据库 Schema v5 不变。

实际执行：

- `npm test`：10 通过；`npm run typecheck`、`npx prettier --check src` 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：37 单元 + 10 集成，共 47 通过。core fmt、core clippy（all-targets、desktop、-D warnings）及 tools clippy（all-targets、-D warnings）通过。
- `npm run build:desktop`：Vite、Windows Release 及 NSIS v0.5.1 成功；最终安装到独立 `artifacts/windows/installed-v051-verified`，注册版本 0.5.1，保留既有目录。
- 最终安装版 `npm run test:desktop -- --real`：真实、空数据、来源断开恢复、数据库首次阻塞四场景通过，前端错误 0。验证五个导航、总览唯一额度、时间线唯一趋势、筛选保留、三种图表、各粒度汇总一致、365 日真实热力图、事件表高度上限、860×640 无全局横向溢出及键盘快捷键；总览和最终 K 线截图已目视检查。
- 最终安装版 `npm run test:reports:desktop`：607 条完整 CSV、精确大整数、未知与继承口径、303 条筛选导出、不覆盖报告、预警分页及窄窗口通过，前端错误 0。
- 最终安装版 `npm run test:alerts:desktop`：生成、重放、已读跨重启、非法规则拒绝及有效规则跨重启通过，前端错误 0。首次回归暴露测试在异步保存完成前关闭应用的竞态；改为轮询后端已保存规则再关闭，复测通过。

本次真实数据验证为原生 GUI 只读采集与 IPC 数值一致性；未重新执行独立全历史验证工具、百万性能或长时耐久测试。先前性能记录保留于下文，不代表本版本重复测量。安装包未签名，卸载和跨平台仍未验证。下一步：Phase 4 托盘常驻与可选系统通知。

---

# v0.5.0 本地 CSV 与预警分页验收 — 2026-10-09

## 实现

CSV 与看板共用前后端筛选口径，从同一数据库读快照完整流式导出；u128 汇总、u64 事件十进制原样保留。独立新目录中生成 usage.csv / metadata.json，partial 完成协议，不覆盖既有报告。未知增量空值、继承历史独立排除、UTC 时间、公式前缀文本保护、CSV 引号转义与 UTF-8 BOM。GUI 可导出并打开固定本机目录，没有网络上传或正文导出。

预警每页 50 条，完整历史可翻页；类型/级别/已读/首次触发时间筛选，查询计数与记录同一事务。首屏实时跟随新预警，进入分页后固定到数据库返回的 created_at 截止点；已读/等级为最新状态。回归发现首屏过早固定客户端时间会隐藏随后生成的预警，已改由数据库读快照提供锚点并通过生成/重启回归。Schema 仍为 v5，未修改 AGENTS.md 或删除用户数据。

## 实际执行

- cargo test --manifest-path src-tauri/Cargo.toml：37 单元 + 10 集成，共 47 通过。新增 602 条完整导出与看板五种汇总一致、模型/会话/时间筛选、u64/u128、未知/继承历史、CSV 引号/换行/Unicode/公式前缀、空数据、重复报告不覆盖、无效筛选写入前拒绝、写入故障不返回成功，以及 123 条分页/同时间稳定 ID/新创建记录隔离/已读/筛选/最后页回退。
- npm test：10 通过。typecheck、Prettier、core fmt/clippy --all-targets --features desktop -- -D warnings、tools clippy --all-targets -- -D warnings 均通过；Vite 与 Windows Release / NSIS 构建成功。
- 最终 NSIS 实际 /S /UPDATE 安装到 artifacts/windows/installed-v050-final，退出 0，注册版本 0.5.0。旧安装目录和报告保留，无卸载或批量删除。
- 最终安装版 npm run test:reports:desktop：独立 REPORTS_TEST_ONLY，607 条 CSV 经 Python csv.DictReader 独立解析；u64::MAX + 605 精确，未知一条留空、继承 7 单独排除、公式文本保护；模型筛选导出 303 条，两次导出独立目录。123 条预警三页无重复、类型/级别/未读组合、已读后匹配数缩减、空时间范围及 860×640 无全局溢出通过；0 前端错误。分页截图已目视检查，含明确 Fixture 横幅。
- 最终安装版 npm run test:alerts:desktop：首次生成、重放、已读跨重启、无效阈值、规则关闭与阈值跨重启保留通过，0 前端错误。
- 最终安装版 npm run test:desktop -- --real：真实、空数据、采集断开恢复、数据库首次阻塞四场景通过；热力图/三种图表/模型筛选/会话详情/额度/财务/设置/主题与窄窗口回归通过，0 前端错误。
- 真实只读验证：cargo run --release --manifest-path tools/Cargo.toml --target-dir src-tauri/target --bin verify -- .local/verification-v050.sqlite3 --quota --export。128 文件、112 会话、2363 快照、192581218 已确认 Token，未知 0；重放新增 0，各粒度总量一致，quota=synced。独立 Python CSV 解析逐行复核全部五种汇总与 JSON/看板一致，13040377 未分配累计仍独立排除。仅验证当时截面，日志持续增长。
- 最终安装版百万合成事件：npm run test:million:desktop。查询 6885.3 ms、查询中导航 39 ms、连续刷新新值显示 5917 ms；完整 CSV 1000000 行导出 3630 ms（含落盘同步），Python 流式 CSV 解析行数与 110000110 Token 精确匹配，0 前端错误。单次机器时间不是性能分位数或长期保证。

## 限制与下一步

目前导出是完整计量事件及口径 JSON，不是 XLSX/账单，也不含额度历史或预警 CSV；Token 列需要电子表格按文本导入才能避免其自动类型推断。固定应用数据目录，无自选保存位置、进度百分比或中途取消。不完整报告保留而不清理。分页范围固定首次触发截止点，已有记录的等级/已读改变仍可改变匹配数。长时耐久、卸载、签名与跨平台本次未验证；系统通知和托盘尚未实现。下一步：Phase 4 托盘常驻与可选系统通知。

---

# v0.4.0 活跃热力图与趋势图验收 — 2026-10-09

## 实现和口径

新增真实增量派生的本机时区 365 天热力图、折线/柱状/K 线切换、供应商标识及卡片/间距/字号优化。热力图与趋势遵循当前筛选；K 线描述单条增量的首末低高，与桶总量不同，空桶 null，未知/未分配累计不参与。Schema 仍为 v5。参考用户上传图片调整布局，没有复制图片中的虚构实时数值、预算、未支持工具或请求次数。

## 实际执行

- `cargo test --manifest-path src-tauri/Cargo.toml`：33 单元 + 10 集成，共 43 通过。新增 OHLC 顺序/零值/u64 精度、闰日/本地午夜/u128 日汇总/窗口排除、空桶、筛选和重放一致性。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`、core `cargo clippy --all-targets --features desktop -- -D warnings`、tools `cargo clippy --all-targets -- -D warnings`：通过。`npm test` 9 通过，`npm run typecheck`、`npx prettier --check src`、Vite 生产构建通过。
- `npm run build:desktop`：Windows Release 与 NSIS v0.4.0 构建成功。核实旧版本安装路径和 MainBinaryName 后，以 `/S /UPDATE` 安装至独立 artifacts/windows/installed-v040，退出 0；仅主程序和卸载器，旧目录与测试产物保留。
- 最终安装版 `npm run test:desktop -- --real`：真实、空数据、采集断开/恢复、数据库首次阻塞四场景通过，前端运行错误 0。新增 365 日单元、日汇总整数一致、方向键/Enter 选择、三种图表及 aria-pressed、OpenAI 来源标识检查；浅色/深色/860×640 窄窗口截图已目视检查，无全局横向溢出，日历和表格局部滚动。修复 K 线总量图例歧义、长排行另一列留白及截图中的跳转链接。
- 只读真实日志验证：`cargo run --release --manifest-path tools/Cargo.toml --target-dir src-tauri/target --bin verify -- .local/verification-v040.sqlite3 --quota`：128 文件、112 会话、2291 快照、已确认 186266618 Token、未知 0；立即重放新增 0、Dashboard 一致、各粒度总量一致，quota=synced。未分配累计 13040377 Token 仍明确排除；数据继续增长，该数值仅为验证截面。
- 最终安装版 `npm run test:million:desktop`：隔离百万合成事件、精确总量、有界列表/图表、查询中导航及连续刷新通过，前端错误 0；IPC 9400.3 ms、导航 46 ms、连续刷新显示新总量 5910 ms。当时同时运行 tools 编译与其他测试，时间不作为性能分位数；先前相同聚合 Release 单次为 7060.2 ms。
- 最终安装版 `npm run test:alerts:desktop`：隔离规则生成、重放、已读重启、非法阈值、规则跨重启保留全部通过，前端错误 0。

## 限制

K 线样本为计量增量，不是请求或金融价格；同时间事件使用确定性 ID 排序。热力图空白不证明来源覆盖完整，日历仅近 365 天并遵循筛选，不代表全历史。绘图用浮点近似，提示和存储保留精确整数。仅 Codex 已支持；未知模型供应商保持未知。本次没有重跑长时耐久、跨平台或卸载测试，安装包未签名。下一步：本地 CSV 导出与预警分页/筛选。

---

# v0.3.0 界面与本地预警验收 — 2026-10-09

## 按阶段实现

先完成暖灰/炭灰双主题、陶土色强调、字体层级、侧栏和可访问导航、财务明细展开、图表随主题更新；在真实原生四场景与最小窗口检查通过后，进入预警开发。参考 [Claude Code 官方桌面设计说明](https://claude.com/resources/articles/claude-code-desktop-redesign)的侧栏、信息分组与键盘导航方向，未复制品牌或虚构 Provider 支持。

预警采用可配置的 UTC 固定五分钟会话高用量阈值（默认 1,000,000），服务端额度 80%/95% 风险等级。排除未知/未分配历史与过期/未来/缺重置/已重置额度。稳定键去重、事务写入、已读重启保留、严重升级重新未读，规则关闭不清空历史。预警页面只有实际生效的控件，最近 200 条是查询上限，不是删除范围。

## 实际执行检查

- cargo test --manifest-path src-tauri/Cargo.toml：31 单元 + 9 集成，共 40 通过。新增未知/旧/未来事件与子集排除、重放、已读/重启、额度新鲜度/账户/重置/等级升级、阈值正 u64 和无效配置不写入。
- cargo fmt --check、core/tools clippy --all-targets（core 加 desktop）-- -D warnings：通过。前端 typecheck、9 项 Vitest、Prettier、Vite 生产构建通过。
- Debug 与优化 Release 独立构建、NSIS 包构建成功。实际安装至 artifacts/windows/installed-v030，当前用户 /S /UPDATE，退出 0，只有主程序与卸载器；没有执行卸载或批量删除。
- 原生真实/无数据/源断开恢复/数据库首次阻塞四场景通过，0 前端运行错误。新增双主题、Alt+2、860×640 最小窗口无全局横向溢出、表格局部滚动检查。截图 final-overview.png、ui-dark.png、ui-compact.png 已目视检查，未混入 Fixture。
- 独立 ALERTS_TEST_ONLY 原生预警：生成、重复导入/重试去重、已读跨两次重启、非法阈值拒绝、关闭规则和新阈值跨重启保留，0 前端错误。截图 alerts-test-only.png 含明确开发验证横幅；不代表真实账号消耗。
- 本次真实只读复核：127 文件、111 计量会话、2,234 快照；未知 0，partial_histories=1，13,040,377 Token 未分配累计继续排除。立即重放 0 新增、Dashboard 一致、三种趋势一致，quota=synced。用量随持续采集增长，上述是验证截面。
- 安装版百万合成事件回归：全范围 IPC 6,055.5 ms、查询中导航 46 ms、每 500 ms 连续刷新后新总量 5,394 ms 显示，精确计数/有界列表/趋势与 0 前端错误。单次时间受同时查询与机器负载影响，不代替 v0.2.0 的 10 次分位数基准。

## 边界与后续

只完成可解释规则提示，没有统计学异常基线、系统通知或 CSV 导出。当前预警无分页，只显示最近 200 条，完整记录保留。历史是检测时观测值，不作为当前额度状态。Schema v5 保留真实用量与额度数据；旧版本拒绝更高 Schema，须运行新版。

本次未重跑 30 分钟耐久；前次 v0.2.0 耐久只证明当时构建。签名、卸载、磁盘满和跨平台仍未验证。最值得继续推进的是带精确十进制计数的本地 CSV 导出与预警筛选/分页。

---

# v0.2.0 验收记录 — 2026-10-09

## 数据准确性和参考估算

最后一条缺初始基线的记录有匹配的逐响应证据，确认本次 101,297 Token；累计减去本次后的 13,040,377 Token 历史无法分配时间/模型/轮次，保存 carried_baseline 并明确排除。真实验证未知快照为 0，partial_histories=1；这不等于已找回完整历史。

独立真实库复核：127 个文件、111 个计量会话、2,129 快照、已确认总量 170,514,863；立即重放 0 新增、完整 Dashboard 相同，分钟/小时/天总量一致。官方 quota=synced。日志持续增长，这些数值仅为验证截面。

参考价覆盖 139,872,167 Token，30,642,696 未覆盖；未知内部模型没有套用公开模型价格。真实 Frankfurter 响应解析贡献方对象，2026-10-09 USD→CNY 6.6993、HKD 7.8541，三币金额均可计算。价格/档位/不包含项见 DATA_SOURCES.md；不是实际账单。

## 检查与实际安装

- cargo test --manifest-path src-tauri/Cargo.toml：28 单元 + 9 集成，共 37 通过。覆盖响应基线、拒绝矛盾证据、重放/重启/迁移、u64 同时间排序、大整数、缓存计价、未知模型与汇率过期/关闭/非法数据。
- cargo fmt --check、core clippy --all-targets --features desktop -- -D warnings、tools crate fmt/clippy：通过。
- npm run typecheck、npm test（9 项）、Prettier、前端生产构建：通过。
- npm run build:desktop：优化 Release 和 x64 NSIS 构建通过，安装包约 3.78 MiB。
- 最终 NSIS 静默安装至 artifacts/windows/installed-final，退出 0；仅 tokenscope.exe 与 uninstall.exe，无验证/基准二进制。安装使用 /UPDATE 保留此前测试安装文件，未运行卸载器，没有批量删除。
- TOKENSCOPE_TEST_EXE 指向实际安装后的 Release，npm run test:desktop -- --real：真实日志/额度/成本与币别切换、未知模型覆盖提示、筛选/会话弹窗键盘、三种粒度五项计量一致；无源/无 CLI；断开/恢复/手动重试/半行重放；首次数据库路径被目录阻塞后恢复。四场景均 0 前端运行错误。隔离场景显示开发验证标识，Fixture 不进入用户库。
- 实际截图 final-overview.png 已目视检查，成本/汇率/继承历史警告正常展示，无面板溢出。

## 百万事件机器实测

测试数据全部为隔离的 TEST_ONLY 合成数据，不冒充账号用量。原始日志导入与查询压力是两套不同 Fixture。

| 验证 | 规模与结果 |
| --- | --- |
| 实际 JSONL 导入 | 100 会话 × 10,000 累计事件；首次导入 74,642 ms；精确 110,000,000 Token，缓存 50,000,000、推理 2,000,000，未知 0 |
| 重启重放/追加 | 重新打开后重放 3 ms，0 新增；半行不计入，补齐后追加 1 条 108 ms，仅增加 110 Token，重复重放不变 |
| 查询压力 | 1,000,000 事件、1,000,000 轮次；10 次全范围分钟聚合 p50 4,620 ms / p95 5,248 ms；会话+模型过滤 54 ms，精确 3,334 事件/366,740 Token |
| 返回规模 | 最近 500 事件与 500 轮次、明确 truncated、趋势 ≤720 点；总量仍覆盖全范围 |
| 安装版实际 GUI | 最终同一百万查询经 IPC 4,497 ms；查询期间导航到设置 40 ms；每 500 ms 持续刷新时，新累计 110,000,110 在 5,409 ms 内显示，精确计数且 0 前端错误 |

查询库约 624 MB、原始日志导入库约 1.24 GB；CLI 进程采样工作集峰值分别约 8.27/11.22 MB，不包括系统文件缓存、WebView2 和其他进程。测量受本机/并行任务影响，不承诺其他机器同等速度。全范围查询仍 O(N)，会话追加仍重算该会话；100 会话分布实测不证明单一百万事件会话的追加耗时。

复现：tools crate 的 benchmark/ingest_benchmark 要求新的测试路径（存在时拒绝覆盖）；仅 benchmark 的 --reuse 可复用指定 Fixture。原生大数据脚本 npm run test:million:desktop 默认使用本次 installed-final 验证安装目录，也支持 TOKENSCOPE_TEST_EXE 指定已安装的 Release exe；要求先生成 .local/million-test-20261009.sqlite3，复制至独立目录，明确测试模式并关闭汇率，不访问真实 Codex。

## 耐久与限制

隔离 WebView2 耐久测试通过：1801.904 秒、506 次追加、5 次进程重启、10 次源目录断开恢复；完整行/半行/重放计数逐次校验一致，前端错误 0。Rust 工作集采样 32.85–35.05 MB；渲染器 JS 堆 6.04–32.78 MB，末次 7.41 MB。采样不包含全部 WebView2 进程内存。原报告位于 .local/SOAK_TEST_ONLY_1791523534042/report.json，构建散列随报告保留。该测试使用复制的 Debug 采集器构建，与最终 Release 的 Rust 采集逻辑一致；测试开始后的前端串行刷新变更另由最终安装版四场景及百万持续刷新回归覆盖。

先前共用 WebView2 用户目录的并行自动化出现 0x8007139F，且旧耐久任务在约 21.5 分钟被其他浏览器连接关闭；修复为每任务独立 WEBVIEW2_USER_DATA_FOLDER，重新运行最终测试。百万查询期间连续刷新可能取消旧结果，已改为串行合并，并增加每 500 ms 持续通知下的新累计展示回归。汇率复选框保存期间曾回弹，已加入待保存状态；原生测试导航名称包含 01，选择器已按实际可访问名称修正。初版安装夹带开发工具，现独立 tools crate 且检查最终安装清单，未把失败版本当作通过。

签名、卸载流程、无 WebView2 主机上的下载安装、磁盘满、Linux/macOS、数日/数周无人值守仍未验证。安装测试主机已有 WebView2。默认系统代理/网络可影响汇率与官方额度，失败显示真实状态。

---

# v0.1.2 验收记录 — 2026-10-09

## 已实现与核对

- 分钟/小时/天趋势由 Rust 同一增量集合聚合，空档补零、UTC 边界与负 epoch 可处理。跨度过长按粒度整数倍合并到最多 720 点，大整数保持 u128 精确总量；Tooltip 显示本地起止时间与精确字符串。
- 采集器明确记录目录缺失、部分文件失败、数据库异常和最后完整成功；3–30 秒退避，恢复后回到 3 秒，容量 1 通道允许手动重试而不并发写游标。上次成功保留作历史信息，不用作当前健康状态。
- 延用 Schema v2，未重建或删除真实用户数据库。

## 实际运行检查

- `cargo test --manifest-path src-tauri/Cargo.toml`：22 个单元 + 8 个集成通过，共 30。新增跨午夜/本地筛选边界、补零、百年跨度和 u128、默认/非法粒度、源目录出现、SQLite 写锁解除后的补采、退避与新鲜度。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` 与 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features desktop -- -D warnings`：通过。
- `npm run typecheck`、`npm test`（7 项）、`npx prettier --check src tests/desktop-smoke.mjs`、`npm run build`：通过。
- `npm run build:desktop -- --debug --no-bundle`：Windows 独立桌面构建通过。
- `npm run test:desktop -- --real`：实际 WebView2 真实日志/额度/三种粒度、无源/无 CLI、隔离合成日志恢复三种场景通过，均为 0 前端运行错误。恢复场景以 TEST_ONLY 独立数据库和源目录运行，不与用户数据混排；验证目录消失、历史保留、重新出现、手动触发、半行续写与重复重试不增量。
- 实际截图 `.local/final-overview.png` 已目视检查，无筛选控件或图表布局溢出。

## 本机真实数据

`cargo run --manifest-path src-tauri/Cargo.toml --bin verify -- .local/granularity-verification.sqlite3 --quota`：126 个文件、110 个有计量会话、2011 条快照。第二次导入新增 0，Dashboard 相同；三种趋势粒度的 total 与卡片完全一致。独立原生 GUI 进一步校验 input/cached/output/reasoning/total 五项均一致。仍有 1 条未知基线；后续日志持续追加，快照和总量会变化。官方 quota status=synced。

初次锁恢复测试在文件未追加时无法触发写锁，随后改为锁期间追加计量行，验证部分文件失败→解除锁→仅补采一次后通过。一次 Cargo 命令误在仓库根目录运行，改用 crate 目录/显式 manifest 后检查通过；未把这些失败列为通过。

## 限制与下一步

当前运行期恢复经过短时故障验证，未证明长期无人值守、不可写数据库首次启动、磁盘满、百万条事件和跨平台运行。趋势点数有上限，但查询仍加载筛选范围所有快照；采集状态不跨进程持久化。未增加通知/托盘或安装包。成本估算和每日汇率尚未实现，下一步先核实模型单价、价格版本与套餐/API 等价估算边界。

---

# v0.1.1 验收记录 — 2026-10-09

## 计数恢复与归属修复

- 确认子线程日志可能包含继承的父 session_meta；修复每次覆盖会话身份的问题，文件首条 meta.id 保持为所有者。共享 session_id 不代替线程身份。
- 新读取 token_usage_record 白名单计数、响应 ID 哈希及线程/轮次身份；仅将与所有者和 last 严格匹配的证据绑定到累计快照，不把逐响应用量另行相加。
- 累计下降必须满足两个不同响应、线程累计差值等于本次 usage、新 local total=last=usage，才恢复并持久化 epoch。错误证据、同响应重复、last 不匹配仍拒绝恢复。
- Schema v2 保留原始快照、采集时间和额度历史，旧误归属行保留为 superseded_assignment；检查点版本不匹配时只重读一次。
- 时间线展示计数区段，并明确标记“重置已核实”。

## 已运行检查

- Rust 测试：17 个单元 + 6 个集成通过（共 23）；包含证据拒绝条件、继承元数据、迁移留存、重启/重复副本。
- Rust clippy --all-targets --features desktop -- -D warnings：通过；cargo fmt --check：通过。
- 前端 4 项测试、TypeScript strict、Prettier：通过。
- npm run build:desktop -- --debug --no-bundle：通过，独立 Windows GUI，不依赖 Vite。
- npm run test:desktop -- --real：最终通过真实数据、精确数值展示、筛选、详情键盘、时间线、额度及设置；隔离无数据环境通过，两种场景前端运行错误均为 0。

## 真实数据与迁移核对

首次本轮恢复验证：125 个文件、109 个有计量会话、1956 条可展示快照；旧库升级和全新导入的 1956 条记录逐项完全一致，比较含原始计数、响应证据、会话/模型/轮次、增量、质量与 epoch。旧库另保留 276 条被纠正的误归属审计行，不计入统计。

不确定快照从此前 685 条降到 1 条（缺失初始基线），一次真实重置得到确认，后续 96 条记录位于 epoch=1。连续两次导入均确认第二次 0 新增，Dashboard 不变。之后 Codex 继续追加，因此记录和总量数字会变化。

独立官方额度读取在本轮再次成功。第一次原生 GUI 冒烟因启动时额度读取不可用而未通过“必须已同步”的外部依赖检查，界面如实显示不可用；独立接口复查成功、迁移完成后重跑原生验证通过。未把这次暂时失败隐藏为成功，也未修改或绕过额度接口。

## 剩余限制

仍有 1 条未知基线；缺少严格响应证据的其他重置/缺字段场景仍冻结为未知。日志格式以本机 0.153.4 / 0.162.0-alpha.2 样本验证，未宣称官方承诺的跨版本日志契约。分钟/小时粒度、持续断连自恢复与大数据性能、成本与每日汇率仍待后续。

---

# v0.1.0 验收记录（历史）— 2026-10-09

## 已实际通过

- `cargo test --manifest-path src-tauri/Cargo.toml`：13 个单元测试 + 4 个集成测试通过。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features desktop -- -D warnings`：通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`：通过。
- `npm test`：4 个前端精度与额度状态测试通过。
- `npm run typecheck`：TypeScript strict 通过。
- `npx prettier --check src`：通过。
- `npm run build:desktop -- --debug --no-bundle`：前端生产构建与 Windows Tauri 独立 exe 构建通过；未运行 Release 优化构建或安装包构建。
- `npm run test:desktop -- --real`：真实 WebView2 连接 `http://tauri.localhost/`，不依赖 Vite；真实历史列表、Rust IPC 与精确数值展示一致、模型筛选、轮次详情、Escape 关闭、时间线、官方额度、设置均通过，前端运行错误为 0。另通过独立 `TOKENSCOPE_DATA_DIR` 与不存在的 `CODEX_HOME`/CLI 验证无数据、额度不可用状态，不显示虚构读数。
- 真实导入验证命令：`cargo run --manifest-path src-tauri/Cargo.toml --bin verify -- .local/verification.sqlite3 --quota`。最近一次 123 个文件、1912 条去重快照、76 个会话；第二次导入新增 0，Dashboard 完全相同；quota status=synced。Codex 持续工作，数量会随时间增加。

## 测试覆盖

历史导入、稳定快照重放、重复累计值、乱序、部分写入、文件替换与副本恢复、应用重启、迁移幂等、多会话、多模型、未知字段、未知计数区段、子集不重复计算、大整数完整链路、时区边界、模型与时间筛选、同名项目隔离、原始正文最小化、额度多池与缺失窗口，以及无数据源/额度服务不可用的真实 GUI。

## 排查中修复的问题

初次桌面构建缺少 Windows 图标，已补齐图标及可编辑 SVG 源。严格 lint 发现测试字符串字节切片写法，已修复。一次测试期间 exe 正在运行导致 Windows 文件锁，关闭本次测试进程后重跑通过。隔离测试发现 Windows Known Folder 不采用 APPDATA 环境变量重定向，改用应用专用数据目录后重新验证通过。

## 未验收与已知限制

- 归档中一次计数下降之后 685 条快照仍标记不确定并排除；当前汇总不代表完整账单。明确 epoch 恢复是下一步最优先的数据准确性工作。
- 账户额度在线同步失败的具体原因目前合并为“不可用/未授权/网络失败”提示；尚未独立构造每一种网络/账户错误。没有通过读凭据或私有接口绕过。
- 不能证明每个模型调用精确对应某个用户问题；turn_id 与 model 是日志上下文关联。
- 未验证跨客户端 Token 通知，使用 3 秒日志轮询；未做百万事件性能、原位中间改写、长期断连、完整无障碍或跨平台发布测试。
- 成本估算、多币别、每日汇率、异常消耗检测、系统通知、安装包、托盘未实现。

AGENTS.md 保持原文件，没有删除 Codex 源文件或用户数据库；目录最初尚未初始化 Git，未创建提交。

## 2026-10-15 启动故障诊断（未结案）

用户从 Explorer 双击 v0.5.14 时，统计、采集、预警、额度均不可用。新增 read_health.database_path / last_error 和设置页诊断显示；成功采集后清除旧错误。用户诊断版反馈 SQLite malformed database schema（错误中出现会话标识），故不能归因于 Codex 登录或筛选。默认数据库只读 quick_check 为 ok，Schema v6 对象正常；数据库副本导入 150 文件、重放及跨粒度一致性验证通过。隔离启动和通过 Shell 的脚本启动可正常采集；额度分别出现正常返回与 Rejected。具体启动差异仍需复现，未删除、迁移或重建用户数据库。

诊断修改：17 前端测试、58 Rust 测试、Prettier、TypeScript/Vite 构建、desktop clippy -D warnings 通过。桌面 empty/recovery/blocked 三场景通过，新增断言验证数据库阻塞时报告实际路径/错误、解除后清除错误。当前便携 exe 为诊断构建，原文件备份 TokenScope_0.5.14-original.exe；既有 NSIS 和 BUILD_INFO_0.5.14 仍对应之前正式构建，不能视为本轮修复产物。

## 2026-10-15 v0.5.15 本机恢复验证（接续上述诊断）

实际 Explorer 启动的临时取证版导出 7,143,424 字节文件，SHA-256 65d99109f4bd2cd82ce0d22c0bb9aa425ab5b8bdbce00e2e76268c48f178e941；该导出副本用 Python SQLite 同样报 malformed database schema。独立工具通过同一绝对路径读取 8,077,312 字节文件且 integrity_check 正常。这个差异已实测，但底层文件视图/重定向机制未确认，早先的独立检查不能代表失败实例。

使用独立正常数据库通过 SQLite backup API 生成恢复源，integrity_check=ok、Schema v6、3449 个 usage_events。恢复源 SHA-256 bcf94229177ae96ab22657b77cf8cde780303405d7e451da349c675e93385fe3。通过 Explorer 启动恢复脚本，停掉已知应用实例，核对损坏文件与恢复源指纹、检查无活动 WAL/journal，先用独占创建保留完整坏文件为 tokenscope.pre-recovery-20261015-132851.sqlite3.bak，再原子替换并复核完整性。未删除原始 Codex 日志；恢复源保留汇率启用、预警启用和 10000000 阈值。损坏文件保留以便后续检查其是否有正常副本未涵盖的非日志设置或历史。

恢复后通过真实 Explorer 启动默认配置：collector=synced、150 日志文件、io_errors=0、parse_errors=0、last_error=null，quota=synced。本机故障已恢复；并非安装新版即可自动恢复任意损坏数据库。正式源码保留 read_health 的数据库位置/失败详情与设置页显示；删除临时取证逻辑。临时捕获 exe、诊断副本与脚本保留为维护产物，不属于正式发行文件。

最终 v0.5.15 NSIS 安装到 artifacts/windows/installed-v0515，退出码 0。实际安装版三场景桌面回归（empty/recovery/blocked）通过、前端错误 0；真实 Explorer 默认数据库启动 collector/quota 均 synced、last_error=null、150 文件无读取/解析错误。最终构建通过 TypeScript/Vite、Rust fmt/desktop clippy；本轮 17 前端和 58 Rust 测试通过。未重跑百万性能、耐久、其他平台；安装包未签名。恢复仅针对本机，底层文件视图差异仍未定。


## 2026-10-15 v0.5.16 输出吞吐率与球体验证

### 实现与口径

用户确认改为近 60 秒可见输出吞吐率（output − reasoning），每 3 秒及采集事件/重新激活后刷新。排除输入和缓存，跨当前筛选会话合计，包含工具等待与空闲时间；不称为模型解码速度。原严格单响应估算接口保留。Schema v6、去重与原始计量规则不变。项目球体增加错峰呼吸/浮动/光晕、八色配色和双主题，实时排序变化保持项目颜色，交互暂停与减少动画模式保留。

### 本轮实际运行

- `npm test`：17 项通过；`npm run typecheck` 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：49 单元 + 10 集成，共 59 项通过。新测试覆盖无单响应完成样本的吞吐率、输出/推理子集、多会话、边界、过滤、未知、u64/u128 和窗口过期。
- `npx prettier --check src`、Rust fmt 检查、desktop 与 tools clippy `-D warnings` 通过。首次 Prettier 检查报告 App.tsx 格式问题，修正后通过。
- `npm run build:desktop`：TypeScript/Vite、Release、NSIS 构建通过；独立安装目录 installed-v0516，安装退出码 0。Vite 保留约 500 kB 图表 chunk 提示。
- `npm run test:output:desktop` 对实际安装版验证：真实 Rust 导入隔离合成日志，页面无需手动刷新，从 0→2.00→6.00 tokens/s；半行等待补齐，工具/多响应工作负载无需生成速度样本，静默期间窗口到期归零。球体浅/深色、860px、色彩区分、呼吸动画、悬停暂停、键盘/Esc、减少动画均通过，前端运行错误 0。产物：`.local/OUTPUT_RATE_TEST_ONLY_1792056232103/`，这些截图明确属于 Fixture 验证。
- `npm run test:desktop` 对实际安装版执行 empty/recovery/blocked 三场景通过，前端错误均为 0；恢复后错误清除。
- 两套桌面脚本首次在受限工具环境中无法连接 WebView 调试端口，未进入页面断言；经受控本机环境启动重跑后通过。未将失败启动记作测试通过。
- `cargo run --manifest-path tools/Cargo.toml --bin verify -- .local/output-rate-real-0516.sqlite3 --output-rate`：只读真实日志、写独立验证库；152 文件、首次 3259 快照，重放新增 0，三种粒度汇总一致。观测窗口内 3 条确认事件、509 可见输出 Token、8.48 tokens/s，与独立 Python SQLite 只读计算一致。这些数字仅代表工具环境当时的验证截面。
- 已保留旧便携 exe 为 TokenScope_0.5.15.exe，将实际安装并验证的 v0.5.16 exe 复制到日常 TokenScope.exe 与版本化便携路径。便携/安装版哈希一致，详见 BUILD_INFO_0.5.16.json。
- 通过 Explorer 打开日常正式程序，Computer Use 核对 v0.5.16 页面：本地采集已连接，设置页显示真实默认数据库路径且无失败原因，额度区域有新鲜服务端快照；主指标明确显示输出吞吐率及口径。该窗口检查不等同于长期稳定性或原数据库损坏根因已修复。

### 限制

日志计数在响应结束后才可观测；60 秒指标是按计量事件时间入窗的墙钟平均，不能用于精确测量单模型解码速度。未知记录不会伪装成已确认用量。未重跑百万、长时耐久、独立账号/导出专项或跨平台；安装包未签名。未执行旧恢复脚本、删库、批量清理或改写源日志。下一步仍以真实启动环境的持续稳定性取证为优先，后续功能按用户任务推进。

## 2026-10-15 v0.5.16 追加发布质量验证

本轮以已安装交付 EXE（SHA-256 FA72ECD46C2AB5ED0DB92BF20E93A9F3BB9CE901E9208B53D3B8B8D576DD1D63）为基线，未修改生产源码、Schema 或发行 EXE。数据库损坏按用户确认已修复；本轮没有执行恢复、删库或修改原始日志。

- 调整 `tests/soak.mjs` 支持 TOKENSCOPE_TEST_EXE，默认选交付 EXE；`tests/million-desktop.mjs` 支持 TOKENSCOPE_TEST_MILLION_DB，使用新生成的隔离百万库；`tests/alerts-desktop.mjs` 按现有 60 秒吞吐率 UI 验证 105/205 输出，保留旧严格生成速度 API 的 40.00 与 complex_turn 回归。
- `node tests/alerts-desktop.mjs`、`node tests/reports-desktop.mjs`、`node tests/account-desktop.mjs` 全部通过，前端错误 0。包含预警重放/已读与设置重启持久化、大整数 CSV 精度/未知空值/公式保护/筛选/防覆盖/取消、账号状态切换/旧身份清除/键盘与窄窗口。最初预警断言误期望分钟整数有 .00 后缀，按实际整数格式纠正后通过。
- `node tests/quota-shell-desktop.mjs` 通过：空覆盖、无 CLI PATH 的真实 Shell 启动，官方账户读取匹配、额度 synced、1 个限额池，前端错误 0。仅该项使用真实官方账户；其余数据均为 TEST_ONLY 隔离 Fixture。
- `cargo run --release --manifest-path tools/Cargo.toml --bin benchmark -- .local/million-v0516-quality.sqlite3`：100 万事件精确总量 110000000；10 次查询 p50 5497 ms、p95 5617 ms，筛选 60 ms；数据库 666714112 字节。无严格性能 SLA，不把通过精度断言称为查询足够快。
- `node tests/million-desktop.mjs`：交付版读取新百万库，查询 7202 ms、同时导航 59 ms、持续刷新 5925 ms；100 万行 CSV 导出 3966 ms，精确计数通过，前端错误 0。报告 `.local/MILLION_GUI_TEST_ONLY_1792065609396/report.json`。百万规模聚合延迟仍是性能限制。
- `cargo run --release --manifest-path tools/Cargo.toml --bin ingest_benchmark -- .local/ingest-v0516-quality`：100 会话/100 万事件，导入 78043 ms、重放 3 ms、新追加 122 ms，重放零重复、部分行与重启后重放通过，数据库 1294983168 字节。
- 三个修改后的 JS 测试文件已 Prettier 格式化、`node --check` 通过。产品源码未变，本轮不重复声明运行之前的单元测试或重建安装包。

最新需求与交付结论：用户改为要求响应进行中、多会话模型输出速率；当前发行版不满足。只读能力探针已运行，见 DATA_SOURCES.md 和 `.local/realtime-capability-v0516.json`。本轮专项通过不能消除该数据源缺口，因此暂停速度指标的最终验收，不将现有吞吐率替代需求。安装包仍未签名，其他平台与长期耐久未验证。

耐久补充：`node tests/soak.mjs 10 9230` 对同一交付 EXE 完成 **603.52 秒、170 次追加、1 次重启、3 次数据源断开恢复**；部分写入、缓存/推理子集与时间线精确计数均通过，前端错误 0。Rust 工作集采样约 26.3–28.0 MiB，渲染器 JS heap 约 5.3–28.2 MiB（不等于总进程内存）；只代表 10 分钟检查，不能据此证明无长期泄漏。报告 `.local/SOAK_TEST_ONLY_1792065432661/report.json`，EXE SHA-256 与交付清单一致。

最终状态：本轮测试脚本、能力探针、交接/来源/路线/验证文档及 BUILD_INFO 已更新；新增 SHA256SUMS_0.5.16.txt。未替换发行文件、未升级版本；实时多会话模型速率仍未实现，最终验收待可靠流式计量源。
