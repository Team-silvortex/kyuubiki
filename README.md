# kyuubiki
Distributed FEM computation platform with a web-based interface and orchestration layer.

⸻

1️⃣ 系统目标
	1.	支持单机用户和跨机分布式 FEM 求解
	2.	前端轻量可视化（WebGPU / Three.js / vtk.js）
	3.	后端高性能计算（Rust Worker 独立进程）
	4.	Phoenix LiveView 管理任务、状态和 UI
	5.	IPC 统一接口（单机/跨机模式）
	6.	可扩展、跨平台（Windows / Linux / macOS）

⸻

2️⃣ 架构概览

┌──────────────────────┐
│      用户浏览器       │
│  (LiveView 页面 + JS) │
│   ┌───────────────┐  │
│   │ WebGPU/Three.js│  │
│   │   渲染可视化   │  │
│   └───────────────┘  │
└───────▲──────────────┘
        │ push_event / WebSocket
        │ PubSub
┌───────┴──────────────┐
│   Phoenix LiveView    │
│   (状态/任务管理)     │
│   ┌───────────────┐  │
│   │ Oban Job Queue │  │
│   └───────────────┘  │
└───────▲──────────────┘
        │ 拉取任务 / 状态回报
        │ IPC / TCP / NamedPipe
┌───────┴──────────────┐
│     Rust Worker(s)    │
│  (FEM 核心计算)      │
│ ┌───────────────┐    │
│ │  FEM Solver   │    │
│ │ 网格装配/迭代 │    │
│ └───────────────┘    │
│ 输出轻量化结果 / 状态 │
└─────────▲───────────┘
          │ 文件/结果
          ▼
┌──────────────────────┐
│  对象存储 / 本地文件  │
│ (大模型、检查点、VTK) │
└──────────────────────┘


⸻

3️⃣ 模块说明

3.1 Phoenix LiveView
	•	任务管理、状态广播、UI 渲染
	•	调用 Context 层 不直接做计算
	•	PubSub 发送状态到 JS Hook / LiveView
	•	单机 / 分布式统一逻辑

3.2 Oban Job Queue
	•	Job 管理：创建、调度、重试
	•	Ecto.Multi 保证事务一致性
	•	支持 checkpoint / cancel / priority

3.3 Rust Worker
	•	独立进程执行 FEM
	•	支持单机多线程或多机分布式
	•	输入：网格、材料、边界条件
	•	输出：轻量化结果 + 状态流 + VTK / Parquet / JSON

3.4 IPC / 协议
	•	单机模式：
	•	Linux/macOS → Unix Domain Socket
	•	Windows → Named Pipe
	•	分布式模式：TCP Socket
	•	消息格式：JSON / MessagePack / Protobuf
	•	流式回报状态，批量或限频发送

⸻

4️⃣ 数据结构（示例）

4.1 Job 对象

{
  "job_id": "uuid",
  "project_id": "uuid",
  "simulation_case_id": "uuid",
  "status": "queued|preprocessing|partitioning|solving|postprocessing|completed|failed|cancelled",
  "progress": 0.0,
  "residual": null,
  "iteration": null,
  "worker_id": null,
  "created_at": "ISO8601",
  "updated_at": "ISO8601"
}

4.2 ProgressEvent 消息

{
  "job_id": "uuid",
  "stage": "solving",
  "progress": 0.42,
  "residual": 1.2e-5,
  "iteration": 120,
  "peak_memory": 1024
}


⸻

5️⃣ Job 生命周期

queued → preprocessing → partitioning → solving → postprocessing → completed / failed / cancelled

	•	每个阶段 Worker 汇报状态
	•	Phoenix 通过 PubSub/LiveView 推送到前端

⸻

6️⃣ 数据流说明
	1.	用户上传 mesh/material/BC → LiveView
	2.	Phoenix 写入 Job Queue（Oban + Postgres）
	3.	Rust Worker 拉取任务
	4.	Worker 执行 FEM，流式回报状态
	5.	Phoenix 接收 → PubSub → LiveView
	6.	Rust 输出轻量化结果 → 浏览器渲染
	7.	完整结果存对象存储或本地

⸻

7️⃣ 可视化策略
	•	浏览器渲染轻中量模型（WebGPU + Three.js / vtk.js）
	•	超大模型：Rust 生成降采样/缩略网格
	•	支持旋转、缩放、切片、云图
	•	可视化与业务逻辑完全解耦

⸻

8️⃣ 单机 vs 分布式

模式	IPC	Worker	存储	状态流
单机	UDS / Named Pipe	1+ worker	本地路径	PubSub / LiveView
分布式	TCP Socket	多机 worker	对象存储	PubSub / LiveView / batch


⸻

9️⃣ 性能与稳定性原则
	•	BEAM 不做 FEM 核心计算 → Rust Worker 独立
	•	IPC 流式 / 限频 → 避免 PubSub 拖 BEAM
	•	状态可恢复、任务可重试
	•	单机/多机统一协议 → 平滑扩展
	•	数据格式统一，轻量化结果用于浏览器渲染

⸻

10️⃣ 可拓展与未来演化
	•	可以平滑替换 LiveView → Next.js 前端
	•	Rust Worker 可增加节点 / 并行任务
	•	支持云端 / HPC 扩展
	•	保持单机体验一致

⸻
