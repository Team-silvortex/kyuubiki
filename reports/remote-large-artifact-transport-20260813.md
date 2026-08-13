# 远程大工件传输闭环（2026-08-13）

## 目标

验证 `KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES` 作为 Headless、Orchestra 与 Rust
Agent 共享的 cross-process transport contract，而不是仅在单一进程中放宽
限制。实验在 Installer 风格的隔离运行根目录中完成，不复用或修改服务器旧
仓库，也不记录主机地址、账号、凭证或机器专用绝对路径。

## 拓扑与约束

- Ubuntu x86_64，16 个逻辑 CPU，14 GiB 内存。
- Orchestra：Docker `elixir:1.19`，仅绑定 loopback，4 GiB 内存和 4 CPU 上限。
- Rust Agent 与 Headless：本轮隔离源码构建的 release 二进制。
- Headless 进程地址空间上限：7 GiB。
- 三端模型工件上限：`700000000` bytes。
- 默认负对照上限：`536870912` bytes。
- 输入文件：`600001051` bytes。
- 序列化模型工件：`600000389` bytes。
- 模型本体是一个可快速求解的热三角形；600 MB 扩展字段只用于强制覆盖工件
  上传、持久化、下载、摘要验证和解码链，避免把传输测试误变成稀疏求解器
  容量测试。

## 正向闭环

当 Headless、Orchestra 和 Agent 都配置为 `700000000` 时：

1. Headless 将大模型流式写入临时文件并提交 `POST /api/v1/model-artifacts`。
2. Orchestra 在 1.040 秒内返回 `201`，随后作业提交返回 `202`。
3. Agent 请求模型内容端点并获得 `200`，完成长度和 SHA-256 验证后解码。
4. `solve_heat_plane_triangle_2d` 完成，结果以内容寻址工件回传并返回 `201`。
5. Headless 三步工作流 `solve -> job_wait -> result_fetch` 最终为 `status=ok`。

观测结果：

- Headless 总墙钟时间：5.87 秒。
- Headless 最大 RSS：1,175,648 KiB，swap 为 0。
- 求解作业：`completed`，执行 2.025 秒，总耗时 2.051 秒。
- Orchestra 稳态内存：93.52 MiB / 4 GiB。
- 模型工件 SHA-256：
  `c02d3f9ca5077ba4ad701f8e42d1196fe7f1cecba7db8e78c0774cb376d406b3`。
- 结果工件：957 bytes，SHA-256：
  `d5ce1a19ed86f5fe95117e7e6f3810d40f7d1cbce2f63b3b8ccd33f18df13f54`。
- 两个摘要都与内容寻址文件名一致。
- Headless 临时模型目录在完成后为空，未遗留 600 MB 中间副本。

## 默认上限负对照

移除 Headless 的显式上限、保持 Orchestra 与 Agent 为 700 MB 后，同一输入按
默认 512 MiB fail-closed：

- 状态：`failed`，`executed_step_count=0`。
- 明确诊断：`size_bytes=600000389 limit_bytes=536870912`。
- 退出码：1，墙钟时间 1.37 秒。
- 最大 RSS：1,175,604 KiB，swap 为 0。
- Orchestra 没有收到第二次模型上传，证明客户端契约没有被服务器配置旁路。

该负对照同时暴露出原 failure receipt 会把这类错误归入通用
`kyuubiki.headless.runtime_failure`。随后本地分类链已增加稳定的
`kyuubiki.headless.model_artifact_limit_exceeded` 错误码，阶段固定为
`artifact_upload`、不可直接重试，并保留旧报错文案兼容分支。这个分类修复由
Rust 单元测试覆盖；上面的远程性能数据仍是修复前原始实测，不伪装成重跑结果。

## Installer 对齐

Installer 远程 Agent 面板现在公开 `remote-model-artifact-max-bytes`，节点注册表
持久化 `model_artifact_max_bytes`。Rust 原生启动器对零值和平台地址空间做校验，
并在 orchestrated 与 offline Mesh 两种启动命令中注入
`KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES`。旧 camelCase 节点配置可读，保存后统一输出
UI 使用的 snake_case 契约。

## 依赖安全复核

远程构建期间 Hex 审计发现锁文件版本落后。本轮将 Cowboy `2.17.0 -> 2.18.0`、
Cowlib `2.18.0 -> 2.19.0`、Postgrex `0.22.3 -> 0.22.4` 和 Ranch
`2.2.0 -> 2.2.1`。升级后已清除高危 Cowlib 内存耗尽、Cowboy 重复头绕过和
Postgrex 注入通告；Orchestra 完整测试为 `479 tests, 0 failures`。

截至本轮，Cowlib 2.19.0 仍被 Hex 标记有两个尚无更高稳定版本可升级的低/中危
通告。它们保留为显式残余风险，不应被写成“审计全绿”；后续依赖门禁需要在
上游发布修复版后再次复核。

## 结论

600 MB 级模型已经完成真实的 Headless -> Orchestra -> Agent -> Orchestra ->
Headless 全链路，而不仅是单元测试或上传探针。原报告中的 512 MiB 阻断点已
从硬编码边界变为可见、可验证、三端一致的部署契约。下一阶段应继续用真实
大型网格验证解码和求解容量，但不得把本轮传输闭环等同于 3M 网格求解能力。
