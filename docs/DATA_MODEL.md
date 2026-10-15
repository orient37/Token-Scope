# 数据模型与统计口径

Schema v1–v4 位于 src-tauri/migrations/，启动按 schema_migrations 版本在事务中幂等执行。v2 增加响应证据与归一化版本列；保留旧原始记录并重新核对归属。providers、accounts、projects、sessions、turns、usage_events、quota_snapshots、ingest_checkpoints 均独立存储。后续 Schema 变更必须增加迁移，禁止删除真实数据库。

## 计量链路

- usage_events 保存白名单原始计量字段 raw_total/raw_last、UTC 纳秒规范化事件时间、采集时间、会话、日志上下文模型和轮次。不保存 JSONL 整行。
- 会话键为 codex + 账户哈希（缺失使用 unknown）+ 文件首条 session_meta.id。后续继承的 session_meta 不切换所有者；session_id 可能是主/子线程共享根身份，不能替代线程 id。项目键为 cwd 的 SHA-256；仅存目录名。父线程关系保留，不将子线程自动并入父线程账单。
- 快照去重键 SHA-256(session + event_time + 白名单累计值)。多路径副本、恢复重读、归档拷贝不会重复插入同一快照。同一累计值不同时间被保留，但增量为零。
- 以事件时间排序；同一时间以累计 total 排序，再以稳定 id 排序。迟到历史记录会重算受影响会话。文件顺序不是事件顺序。
- 首条 total=last 则计入；不相等时作为 baseline_unknown，后续可计算非负差值。累计下降仅在相邻快照均严格匹配 token_usage_record、响应 ID 不同、线程累计差值等于本次 usage、重置后的 total=last=usage 时恢复，并将 epoch 加 1。缺少证据、字段非法、连续线程差值不匹配时继续冻结为 uncertain_epoch，不根据“值变小”猜测重置。
- 响应证据只保留 usage/thread_token_usage 白名单计数和响应身份哈希，通过当前文件 owner 与 record.thread_id 一致、last 匹配来绑定快照。响应记录本身不与累计快照相加，避免双计数。
- 升级只在游标算法版本不匹配时重新扫描一次。旧错误归属记录保留为 superseded_assignment（不参与展示统计）；只有逐响应证据证明新归属、旧记录尚未被正确源重新验证时才替代。源文件已丢失的旧记录保留为 legacy_assignment_unverified，显示不确定，不静默删除。
- input+output=total；cached<=input；reasoning<=output。差值也校验子集约束。cached 和 reasoning 不再加到总量。
- 计数 raw 使用 u64，聚合 Rust u128；跨 Tauri 一律十进制字符串，显示使用 BigInt。图表坐标可近似，Tooltip 保持精确整数。
- 模型、轮次是日志上下文关联，不是已证实的逐问题精确归属。未关联单独列出。

## 文件与恢复

检查点保存路径哈希、已处理换行字节偏移、文件首部和游标前部锚点哈希、脱敏解析上下文、错误计数。只消费以换行结束的完整行。追加读取不反复扫描已读内容；缩短/锚点变化会重新读取，通过稳定事件键幂等恢复。每个文件的快照和游标事务提交；随后在单独事务重算受影响会话。崩溃遗留 pending 快照在下一次导入先重算。

边界：仅比较首部与游标附近锚点，无法发现保持长度与两处锚点完全不变的中间原位改写；源日志按追加模型处理。丢失元数据/时间戳的计量行跳过并计入错误提示；语法错误行不存正文。无法恢复从未观察到的原始数据。

## 聚合、时间与额度

所有卡片、会话、轮次、模型、项目和图表复用同一 Rust 增量集合。默认全部时间；今天按本地午夜转 UTC，筛选为 [from,to)。趋势可选分钟、小时、天，默认 UTC 自然日；所有区间按 UTC Unix epoch 对齐，以 [start,start+interval_seconds) 分桶。首末可确认增量间补零；零点表示该区间没有已确认增量，不能证明无实际消耗。数据范围过长时按基本粒度整数倍合并至最多 720 点，不抽样、不丢弃增量；查询返回实际 interval_seconds 和 requested_grain。只纳入筛选范围 [from,to) 内的事件，即使首末桶边界超出筛选范围也不引入其他事件。GUI 图表标签、完整起止 Tooltip 和事件明细按本地时区显示，桶边界保持 UTC 对齐。最近事件与最近轮次列表各最多 500 条；轮次截断由 turns_truncated 标识。入选轮次仍聚合全部匹配记录，卡片/趋势/项目/模型/会话总量不受上限限制。

额度快照独立保存，来源 app-server；保留服务端额度字段原语义，用账户哈希隔离。remaining=max(0,min(100,100-usedPercent))，不换算 Token 余额。缺窗口、失败、超过 5 分钟和已到重置时刻分别显示未知/不可用/过期/等待同步。80% 已使用为界面预警，不代替服务端 permission 字段。不会复用旧账户额度冒充当前账户读数。

## 已确认响应与缺失的继承历史

Schema v3 增加 carried_baseline 和查询索引。首条 total 不等于 last 时，只有 owner/响应/last 已匹配的响应证据，且 local total 与 thread total 均可减去 usage、差值合法，才计入 usage；标记 verified_response_partial_history。其余 local 累计差值保存为 carried_baseline，无法分配时间、模型或轮次，不计入事件总量/趋势/成本。缺证据或矛盾仍保留 baseline_unknown，绝不推测历史。

本机唯一此类记录已确认本次 101,297 Token；单独保留 13,040,377 Token 历史累计（输入 12,942,792、缓存 12,441,600、输出 97,585、推理 51,393）。未知快照为 0 不表示完整历史账单。相同首条快照重放/重复累计不会再次增加 usage。

## 参考价格与汇率

Schema v4 新增 app_settings、price_versions、fx_snapshots。模型价格版本为 openai-standard-short-2026-10-09，以 nanoUSD/token 整数保存。输入拆为非缓存和缓存，输出包含推理，不另加 reasoning。checked u128 计算后以 9 位小数的十进制字符串跨 IPC；前端 BigInt 舍入至显示分，不用 JS Number 计算金额。未知模型计入 uncovered_tokens，金额为空，不借用近似价格。

USD 是筛选范围已确认记录按该参考场景重估，不是按事件当日价格还原实际账单。FX 仅将 USD 参考金额换算为 HKD/CNY；保存来源、贡献方、观测日期和采集时间。状态区分 synced/disabled/stale/unavailable，失效时暂停外币金额。报价必须为 USD 基础、唯一的 HKD/CNY、合法正数且观测日期距 UTC 今天 0–7 天；采集缓存有效期 24 小时。

## Schema v5 规则预警

alerts 保存规则键哈希、类型、级别、标题、触发依据、UTC 首次/最近更新时间与已读时间；app_settings 保存 alerts_enabled 与 usage_alert_threshold。没有保存对话正文。阈值通过字符串传输并校验为正 u64，窗口累计用 u128。用量键含规则版本、会话与 UTC 五分钟起点；额度键含账户、池、窗口类型与服务端重置时间。普通更新保留已读；warning→critical 清除已读，重放不再次清除。

额度只接受 synced、采集时间不在未来且距当前不足 300 秒、usedPercent≥80 且重置时间存在并在未来的窗口。≥95 为严重。历史提示记录触发/最近检测时观测值，不自动当作当前有效状态。用量不是统计型异常检测，迟到事件进入其事件时间对应窗口；早于当前窗口的历史导入不补发预警。最近 200 条为返回上限，完整历史保留在 SQLite。


## v0.4.0 图表聚合（Schema 仍为 v5）

Dashboard 新增 candles（与 timeline 索引一致的可空 OHLC）与 activity。OHLC 输入为可确认 Counts.total，不是累计快照；SQL 按 event_at DESC、id DESC 流式读取，首个值为 close，后续较早值更新 open，min/max 为 low/high。同一时间 ID 排序只是确定性顺序，不宣称精确请求先后。空桶返回 null，真实零增量保留为零样本。IPC 值为十进制字符串；u64 样本、u128 日汇总不丢精度。

activity 返回本机今天及此前 364 天的稠密日期、星期、总量、颜色等级，以及 Rust 计算的 active_days/total/peak。事件通过 chrono::Local 转换本机时区日期，每个可确认增量进入一个日桶；早于窗口事件不进入日历但保留原有总览统计。过滤后没有消耗的日期为 0，不能推断采集完整。颜色按相对峰值四等份、正值向上分级，使用防溢出整数比较。与 UTC 对齐趋势的桶边界不同，页面明确标示；全部使用同一过滤、同一数据库读快照。没有改动原始数据或 Schema。


## v0.5.0 CSV 和预警查询（Schema 不变）

CSV 与 Dashboard 复用 usage_query 的 UTC [from,to)、项目/模型/归一化会话条件及 normalization_version>=0。CSV 按 event_at、id 升序完整流式读取，不使用 GUI 截断列表。token 数为十进制字符串，输入/缓存与输出/推理不重复相加；unknown/null 增量的五列空值、confirmed=false。carried_baseline 独立五列，meta 汇总单独保留且不计入确认用量。event_id 为去重事件键；session_id 为源日志会话标识。源时间为 UTC。公式前缀文本（忽略前置空白后 =+-@）加单引号；双引号转义，字段统一引号包裹，UTF-8 BOM、CRLF 行尾。Excel 等需将 Token 列导入为文本，CSV 不保证软件自动类型推断的整数精度。

metadata.json 为报告完成标记；包含实际行数、五种汇总、未知数、继承历史、筛选与口径。报告独立新目录、create_new 写入、flush/sync_all 后重命名；失败保留 partial，不返回成功；不扫描/改写源日志、不导出正文/账户 ID/源路径。导出目录路径仅用于本机定位报告。

read_alerts 增加可选过滤，返回 total/page/pages/page_size/as_of。50 条固定分页，created_at DESC/id DESC，首次触发时间筛选与 UTC 截止点；count/items/global unread/settings 同一事务。无截止点时查询实时首屏，并返回该数据库读快照中最大的 created_at 作为后续翻页锚点。新创建记录在固定截止点之后不改变分页范围；已有记录的等级/已读改变可改变匹配数，因此页码由服务端夹到最后有效页。刷新重新进入实时首屏。kind 仅 usage_high/quota_risk，severity 仅 warning/critical；无效筛选拒绝。已有创建时间索引保留，没有 Schema 改动。


## v0.5.4 Token 消耗速率

Codex 信息栏增加 tokens/s、tokens/m，按当前筛选下最近 60 秒已确认的输入 + 输出增量计算。tokens/m 为窗口总量，tokens/s 为总量 / 60（保留两位小数），是消耗速率而非模型输出生成速度。每 3 秒刷新，过期事件自然离开窗口；未知增量提示且排除，缓存与推理子集不重复相加。采集未就绪/异常/过期或查询失败显示 —，不以零冒充不可用。读取最近时间索引，不刷新全历史图表；Schema v5 不变。


## v0.5.6 模型生成速度估算

总览 Codex 栏改为模型可见输出生成速度（估算），替换此前的输入 + 输出消耗速率。可见输出 = output_tokens − reasoning_output_tokens；耗时 = task_complete.duration_ms − time_to_first_token_ms；tokens/s = 可见输出 × 1000 / 耗时毫秒，tokens/m = 可见输出 × 60000 / 耗时毫秒。两种单位分别从同一精确整数比率四舍五入至两位小数，避免先舍入每秒再乘 60 的误差。

仅接受 task_started / task_complete 完整、有且只有一个独立响应证据、无工具/压缩或其他非消息项、模型一致且输出计数与已归一化增量一致的轮次；拒绝缺失/非正耗时、未知计量及没有可见输出的样本。展示当前筛选匹配的最近完成轮次（最多近 15 分钟），多个并发会话取最近一条，不相加；最近轮次不合格时显示未知，不退回较早的高速度。不是实时流式测量，扣除首 Token 等待仍可能包含收尾开销，故明确标注估算。

Schema v6 新增 generation_samples 和索引，仅保存耗时、计数和响应身份哈希，不保存消息正文或工具参数。游标版本 3 首次升级只读重放一次以回填，之后继续增量采集；原计量和额度记录保留。旧版本拒绝 Schema v6，升级后应继续使用新版。read_usage_rate 保留为独立消耗统计，UI 改用 read_generation_speed。


## v0.5.16 可见输出吞吐率与项目球体（2026-10-15）

用户确认将总览主速度指标改为近 60 秒可见输出吞吐率。旧实现每 3 秒查询，但仅接受近 15 分钟内已完成、无工具且单响应的轮次；当前真实编程工作负载多为工具/多响应轮次，因此长期显示未知。这是旧指标覆盖条件的限制，不能通过把输入消耗算作生成速度来修复。

新命令 `read_output_rate` 对当前筛选内 `(now-60s, now]` 的已确认增量计算 `output - reasoning`，跨匹配会话合计；每分钟值为窗口总量，每秒值为总量除以 60。Rust checked u128 聚合，十进制字符串跨 IPC。包含工具等待、空闲时间；按日志计量事件时间归桶，日志通常在响应结束后报告计数，不代表逐 Token 实时解码速度。没有窗口记录且采集健康时可显示 0；计量全未知、采集异常或读数过期显示未知，部分未知明确标注仅已确认部分。

界面每 3 秒查询，收到 usage-updated 或窗口重新激活时立即刷新；查询串行合并，切换筛选或卸载后丢弃旧结果。独立时钟检查读数新鲜度。旧 `read_generation_speed` 严格估算与 `read_usage_rate` 总消耗接口继续保留，主界面使用新口径。Schema v6 无迁移，不重读或改写 Codex 原始日志。

项目球体增加错峰浮动、表面呼吸和轻微光晕，使用区分更明显的八色配色及深色主题变体。用量排序变化时保留已分配的项目颜色；全部项目、精确计数和占比仍可访问。悬停/键盘聚焦暂停动效，Esc 关闭详情，减少动画模式禁用动效。半径含最小操作尺寸，面积仍不是线性消耗编码。
