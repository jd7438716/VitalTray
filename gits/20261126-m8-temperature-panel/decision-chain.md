# 决策链（decision-chain）

## D1. 整树一个提交，不按「两条线」拆

- 备选：只提交本会话的文件 / 拆成两个提交 / 整树一个提交
- 选择：**整树一个提交**
- 理由：两条线在**同一批文件的同一批 hunk 里**交叉，拆不开：
  - `probe.rs`：我的「磁盘温度输出」和对方的「幽灵设备标记」在同一个函数里；
  - `panel/model.rs`：我的 `temp_text`/温度行与对方的 `Tone::Neutral|Accent`、`status::DEFAULT_LOW`、`device_rows(readings, low)` 交织；
  - `bh-core/Cargo.toml`：我的 `Win32_System_IO` 特性与对方的三个 `bh-proto-*` 依赖；
  - 文档（README/CHANGELOG/DEVLOG/PLAN）已被对方合并成同一份叙述。
  任何「只提交一部分」的做法都会产出**提交即编译不过**的中间态（例如 `probe.rs` 用了 `DeviceReading.ghost`，而该字段在 `poller.rs`）。
- 后果：本次提交同时含两条线的内容；另一会话若还要提交自己的后续改动，需从本提交之后继续。

## D2. 未跑 `butler-git` 五阶段 workflow，改单 agent 直执行

- 备选：`workflow(butler-git.js)` 编排 / 手执行
- 选择：**手执行**（与 `20261126-publish-initial` 的 D5 同一处理思路）
- 理由：另一会话正在同一工作树连续落盘（14:02–14:24 多次写入），编排型 pipeline 的子代理会做广域 `git add`，且多 agent 并发写 index 风险更高；用户诉求是「把这次改动提交掉」，手执行即可满足守门。
- 保留的守门：`todo_write` 清单、写操作先请许可（用户明确回「提交」）、决策文档落 `gits/`、提交后只读验证（`git log` / `git status`）。

## D3. 提交前先停掉常驻进程

- 现象：`cargo build --release` 报 `failed to remove target\release\batteryhub.exe`。
- 处理：`Get-Process batteryhub | kill` → 重新 build → 验证 → 提交后再启动。
- 理由：exe 被运行中的进程占用时无法链接，门禁等于没验证。提交前必须让门禁真的跑通。

## D4. 不 push

- 用户只说了「提交」。push 是对外动作，另需明确许可。
- 后果：本地领先 origin/main 一个提交，可由用户随后 `git push`。

## D5. 核显温度不做「代理数字」

- 备选：保持 “—” / 用 CPU 温度冒充核显温度 / 隐藏 GPU 行
- 选择：**保持 “— ”**
- 理由：Intel 核显与 CPU 同 die，驱动不在 `KMTQAITYPE_ADAPTERPERFDATA` 上报核显温度；厂商 EC 路径要内核驱动（`AsusWinIO64.sys`），与「不为读数提权」原则冲突。
- 后果：核显机器上该行恒为 “—”；有独显（NVIDIA/AMD）的机器自动有数字，逻辑无需改动。已写入 CHANGELOG 已知限制。

## D6. 硬盘温度用协议专有查询，而不是 `Get-StorageReliabilityCounter`

- 备选：`Get-StorageReliabilityCounter`（要管理员）/ 直接发 NVMe SMART log page
- 选择：**直接发 NVMe SMART log page**（`IOCTL_STORAGE_QUERY_PROPERTY` + `StorageDeviceProtocolSpecificProperty(50)`）
- 理由：前者普通用户直接 `Access denied`；后者对 `\\.\C:` 只要 `dwDesiredAccess = 0` 就能读（本机实测 322 K = 49 °C，与备件 100% / 磨损 4% 自洽）。
- 后果：只覆盖 NVMe；SATA/机械盘读不到就 “—”。这是能力边界，不是 bug。

## D7. 采样不再采集面板不展示的数据

- 备选：保留 `\GPU Engine(*)` 采集（`--sysmon` 排障仍要）/ 彻底删掉
- 选择：**按调用方开关**（`Pdh::new(gpu_usage: bool)`；后台 `false`，`--sysmon` 走 `spawn_with_usage()` 仍带）
- 理由：`\GPU Engine(*)` 有 523 个实例，`collect` 306 µs + 取值 595 µs，占每帧 0.9 ms 的九成；面板已经不看占用率。
- 后果：面板侧不再有占用数据（本来也不显示）；排障命令保持原样。
