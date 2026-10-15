# 数据来源与 Phase 0 验证

验证日期：2026-10-09，Windows 11。Node 20.18.0、Rust/Cargo 1.93.0、Codex 0.162.0-alpha.2。

## 已实际验证

1. 本机默认 ~/.codex/sessions 存在 JSONL，另有 archived_sessions。初次发现当前目录 111 个文件；联合归档验证为 123 个文件。
2. session_meta.payload.id 标识线程；session_id 在主/子线程间可能共享，creator_account_id 可选，cwd 为项目；parent_thread_id 为父线程关系。只保存必要身份的哈希与目录名，不保存 base_instructions。
3. turn_context.payload.model 与 turn_id 提供事件上下文，不能证明每个内部调用与用户问题一一对应。
4. event_msg.payload.type=token_count 中 info.total_token_usage 是累计快照，last_token_usage 是最近调用计数。实际观察 input_tokens、cached_input_tokens、cache_write_input_tokens、output_tokens、reasoning_output_tokens、total_tokens。已验证样本满足 total=input+output，缓存和推理分别是子集。cache_write 保留原值，但本版不单独估价。
5. 通过本机 CLI generate-ts 生成协议（.local/protocol，未作为生产代码依赖）。initialize → initialized → account/rateLimits/read 已返回真实额度，使用 stdio JSON 消息，没有自行读取或复制认证文件。
6. 已确认 rateLimitsByLimitId、rateLimits、accountId、ordinaryUsageAllowed，窗口 usedPercent、windowDurationMins、resetsAt，Credits 等。时间戳为 Unix 秒。

首次 Rust 导入验证：123 个文件、1880 个去重快照、76 个有计量的会话；立即重复导入零新增且完整 Dashboard 序列化相同。运行中的 Codex 会继续追加，因此这些数字不是固定产品指标。

## 不确定与限制

- v0.1.1 验证：原 685 条不确定记录主要来自继承元数据使子线程误归入父线程，以及一次真实计数重置。修复所有者后，通过相邻 token_usage_record 的 thread_token_usage 连续差值严格等于 usage，证实该重置并恢复新 epoch；不确定范围降到 1 条缺失初始基线。仍不能称为完整账单。
- 新验证字段 token_usage_record.payload.thread_id、response_id、usage、thread_token_usage、turn_id。当前样本逐响应 usage 满足 total=input+output；文件首条 meta.id 与自身记录 thread_id 一致。子线程日志可能再次包含父 session_meta，session_id 在主/子线程间共享，不能用最后一条元数据覆盖文件身份。此格式以本机样本验证，官方公开 App Server 文档未完整定义 JSONL 私有实现细节。
- 首个 total=last 可确认；v0.2.0 另允许匹配的独立响应证据确认本次 usage，继承累计单独保留。无证据仍为未知。
- account/rateLimits/updated 与 thread/tokenUsage/updated 已在当前生成协议和官方文档确认，但未验证是否能旁观另一个客户端的线程通知。MVP 不依赖它们，不创建或恢复用户线程；用日志增量采集 + 额度定时读。
- 本地日志是版本相关观测接口。仅安装版本和真实样本验证，不保证未来格式不变。缺少核心计量字段时保留快照并显示不确定。
- 多设备用量、远端线程和未写入本地的调用不在本地统计范围；账户额度不可替代或相加。
- 其他 Provider 没有真实采集实现。v0.2.0 已实现参考价格与每日汇率，适用场景与覆盖范围见下。

官方依据：[Codex App Server](https://learn.chatgpt.com/docs/app-server)。生产实现只调用已在本机成功验证的读额度方法。

## v0.2.0 再验证与公开价格 — 2026-10-09

真实只读重放：127 文件、111 个计量会话、2,129 快照；立即重放零新增，三种粒度一致。未知 0，1 条有已确认本次响应且缺继承历史的记录；13,040,377 历史累计明确排除。数据持续追加，以上为一次验证截面。

核实官方模型页的 Standard/短上下文 USD 每百万 Token 输入/缓存输入/输出：

| 模型 | 输入 | 缓存输入 | 输出 | 来源 |
| --- | ---: | ---: | ---: | --- |
| gpt-6.1-sol | 2.00 | 0.10 | 10.00 | [官方模型页](https://developers.openai.com/api/docs/models/gpt-6.1-sol) |
| gpt-6-luna | 0.10 | 0.01 | 0.50 | [官方模型页](https://developers.openai.com/api/docs/models/gpt-6-luna) |
| gpt-5.6-luna | 0.20 | 0.02 | 1.20 | [官方模型页](https://developers.openai.com/api/docs/models/gpt-5.6-luna) |

[官方价格说明](https://developers.openai.com/api/docs/pricing)定义其他档位与费用。内部模型/未核实模型不估价，长上下文和其他溢价未纳入；本版只是明确场景的 API 等价参考。

[Frankfurter](https://frankfurter.dev/) 公开汇率接口已实测：v2/rates?base=USD&quotes=HKD,CNY&expand=providers。2026-10-09 观测值 CNY 6.6993、HKD 7.8541；这不是交易成交汇率。providers 实际是包含 key/date/rate 的对象列表，解析保存 key；也兼容字符串来源字段，不把缺明细伪装为确定来源。请求不包含账户、项目、日志或 Token 数据。


## v0.5.2 桌面程序发现 — 2026-10-11

本机 Codex CLI 0.162.0-alpha.2 位于 LOCALAPPDATA/OpenAI/Codex/bin 的版本目录。Codex 开发终端注入了该 PATH，但 Windows 用户及系统 PATH 未包含它；仅通过 codex.exe 启动导致普通桌面进程找不到程序。额度读取现增加安装目录回退，每次请求重新发现，不固定版本目录。覆盖变量优先且严格保留。仍只调用 initialize / initialized / account/rateLimits/read；启动子进程由 Codex 自己处理已有登录，不读取或复制凭据。未在其他 Windows 安装布局或平台验证自动发现。


## v0.5.3 补充启动验证 — 2026-10-11

用户反馈 v0.5.2 仍启动失败。以 PowerShell Start-Process 重现，补充系统错误后实际收到 program path has no file name。Windows shell 启动保留了被清空的 TOKENSCOPE_CODEX_BIN，Some(空路径) 被当成显式程序。原 Node 测试删除变量，未覆盖这一情况，不能据其成功推断桌面启动成功。现空值按未配置处理，并新增 shell 启动的真实只读额度测试；非空覆盖仍权威。不读取认证文件。


## v0.5.6 耗时字段验证 — 2026-10-11

本机 Codex 0.162.0-alpha.2 的当日日志只读检查发现 task_started.turn_id，task_complete 的 duration_ms、time_to_first_token_ms、turn_id，以及 token_usage_record 的响应身份与输出计数。任务完成事件包含 last_agent_message，但本应用不保存或输出其正文。观察到的样本有完整耗时，但没有每次模型调用独立解码耗时。

本机生成协议 Turn.durationMs 注释是整轮开始到完成的耗时；TokenUsageBreakdown 只含计数。[官方 App Server 文档](https://learn.chatgpt.com/docs/app-server)确认活跃线程的 tokenUsage 更新与消息 delta，但不能据此假定旁观另一个客户端的完整 Token 时间序列。没有创建或恢复用户线程。现有估算仅用于单响应、无工具轮次，排除推理输出并扣除首 Token 等待；不承诺精确解码速度或流式实时速度。


## v0.5.9 账号读取验证 — 2026-10-13

本机 CLI codex-cli 0.162.0-alpha.17.2，经 initialize / initialized / account/read（refreshToken=false）只读实测，account 仅返回 type、email、planType；登录方式为 chatgpt，邮箱存在。未记录邮箱具体值，未读取认证文件。没有头像、姓名、用户名字段，不能用邮箱前缀冒充姓名或用户名。

[官方 App Server 账号接口](https://learn.chatgpt.com/docs/app-server#auth-endpoints)说明 account/read 读取当前账号，ChatGPT email 可为 null；API Key、Bedrock 和无需 OpenAI 登录的配置需分别呈现。已复用现有受控 stdio 传输；仅允许已验证字段进入页面，额外响应字段丢弃。登录配置来自启动的 Codex CLI，与任意其他独立 Agent 实例的账号是否一致不能推断。

## 2026-10-15 多会话实时模型输出速率可行性核查

用户要求响应进行中显示多会话模型输出 Token/s，取代 v0.5.16 的 60 秒吞吐率需求。本次仅核查元数据，不保存或输出提示词、回复、认证信息。

- 最近 8 份真实日志有 1,116 条 token_usage_record；未发现 time/duration/latency 类逐响应字段。68 条 task_complete 中 68 条有 duration_ms、65 条有 time_to_first_token_ms。整轮耗时不能覆盖工具/多响应时的独立生成耗时。
- 当前生成协议 AgentMessageDeltaNotification 为 threadId/turnId/itemId/delta（文本），TokenUsageBreakdown 为累计计数字段，无逐 Token 时间戳。内部 rawResponse/completed 的确切用量来自响应完成，不能当作实时 Token 流。
- 官方文档 https://learn.chatgpt.com/docs/app-server 说明 thread/read 不订阅、thread/loaded/list 列出内存线程、消息 delta 是文本追加。没有据此确认可从独立监控进程旁观另一个桌面进程的所有流式会话。
- `python tests/realtime-capability-probe.py` 在受限环境首次初始化超时；在本机受控环境重跑成功，独立 App Server 的 loaded_count=0。该验证只调用 initialize/initialized/thread/loaded/list，未创建、恢复或修改线程。结果 `.local/realtime-capability-v0516.json`。

结论范围：当前已验证数据链路尚不能满足响应中、多会话实时模型 Token/s。不能用 60 秒墙钟平均、字符数、文本片段数或整轮耗时冒充。下一步技术前提是获得受支持且经验证的各会话流式 Token 计量来源；当前未实现，未宣称接口永远不可能支持。

本轮重新运行 `codex --version` 确认实际 CLI 为 **0.162.0-alpha.17.2**，并使用 `codex app-server generate-ts --out .local/protocol-realtime-v0516-quality` 生成当前版本协议；以上字段结论已对新生成文件复核，不仅依赖旧协议快照。生成命令成功退出，但受限环境报告无法创建 PATH aliases 的警告。
