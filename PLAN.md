# BatteryHub 自研计划书（Clean-room · 插件化 · 小文件）

> 版本：v0.1 草案 ｜ 日期：2026-11-23 ｜ 目标：Windows 10/11 任务栏无线外设电量工具，自研、原生、可扩展
> 参考原则：只抄思路，不抄代码；参考对象 = 原版 BatteryHub 的架构思路 + Gitee「休息提醒 XiuxiTimer」的托盘/右键/自启交互。

---

## 0. 一页摘要

- 技术选型（建议）：Rust + Windows API（windows crate），零第三方运行时，单 exe，目标 ≤ 1.5 MB、常驻内存 ≤ 8 MB。
- 架构：核心引擎（枚举/轮询/解析）+ 三层插件体系：① 内置编译期驱动 ② 声明式驱动包（TOML，用户可加设备，不执行代码）③ 原生 DLL 插件（v2 可选）。
- 纪律：任何源文件 > **400 行**即 CI 失败；单函数 > 60 行告警；一个文件一个职责；驱动一座一文件。
- 交互：托盘图标右键 = 任务栏悬浮窗右键 = 同一套菜单构建器；首次运行自动把托盘图标从 Win11 的 ^ 溢出区固定出来（只做一次）。
- 自启：默认写 HKCU Run（不碰 HKLM、不要管理员），支持便携模式完全免注册表；升级/移动路径自动校正；卸载器负责清理。
- 里程碑：M0 骨架（1天）→ M1 首个驱动（2天）→ M2 任务栏 widget + 完整右键（3天）→ M3 历史/续航/通知（3天）→ M4 插件/i18n/打包（3天）。

---

## 1. 目标与非目标

### 目标
1. 任务栏（不是托盘）直接显示 🎧 30%  🖱 93%，低电变红、充电 ⚡、睡眠变暗。
2. 托盘图标每设备一个（可选单图标汇总），右键菜单完整可用。
3. 驱动可扩展：加一款新设备不改核心代码，理想情况只写一个 TOML。
4. 免管理员、免驱动、免服务、免运行时；断网可用；零遥测。
5. 任何单文件不超过 400 行（硬门禁）。

### 非目标（v1 明确不做）
- 不做电流/健康度/循环次数（硬件不暴露）。
- 不做厂商全家桶替代（RGB、宏、DPI）。
- 不做自动更新（v1 只做“检查到新版本→打开浏览器”，避免未签名更新通道）。
- 不支持 Linux/macOS（架构不排斥，不投入）。

---

## 2. 只抄思路：洁净室规则

### 2.1 三类信息
| 类型 | 例子 | 处理 |
|---|---|---|
| 协议事实 | VID/PID、usage page、请求帧字节、响应偏移、校验算法 | 可以自由使用，但必须记入协议笔记并注明来源 |
| 架构思路 | 驱动表、轮询调度、任务栏子窗口挂载、菜单结构 | 可以学习，用自己的方案重新实现 |
| 代码表达 | 具体函数体、结构体定义、注释、命名 | 禁止复制（GPL 传染 + 版权风险） |

### 2.2 工作流
1. 读参考源码 → 在 docs/protocol-notes/<设备>.md 只写“事实”（帧格式、时序、坑）。
2. 关掉参考源码，按笔记独立实现。
3. PR 模板强制勾选：“本 PR 未复制任何 GPL 代码；协议来源见 notes”。
4. docs/licenses.md 列出所有参考项目及许可，CI 检查清单未过期。

### 2.3 参考库与许可（已核实）
| 来源 | 许可 | 用法 |
|---|---|---|
| 原版 BatteryHub / 4 个 C++ fork | GPL-3.0 | 只读思路 |
| HeadsetControl | GPL-2.0 | 只读协议 |
| OpenRazer | GPL-2.0-or-later | 只读协议 |
| Solaar | GPL-2.0 | 只读协议 |
| rivalcfg | WTFPL | 可直接借用实现 |
| hidapi | BSD-3 | 可依赖/可借用 |
| BlueGauge | MIT | 可借用 |
| LibreHardwareMonitor | MPL-2.0 | 可用，改动文件需开源 |
| ZGMFX01A（Rust 版） | 自定义 All rights reserved | 禁止参考代码 |
| BatteryMode | EULA（非开源） | 禁止参考代码 |

本项目许可：MIT（默认）。若你决定长期走 GPL 也可以，但一旦抄了 GPL 代码就回不去了 —— 所以按 MIT 纪律写。


---

## 3. 架构总览

```text
┌──────────────────────────────────────────────────────────┐
│ UI 层                                                     │
│  tray（托盘图标+右键菜单）  widget（任务栏子窗口+右键菜单）    │
│  settings（设置页）        about / 第一次运行引导            │
├──────────────────────────────────────────────────────────┤
│ 应用层                                                     │
│  AppState（设备表/配置/生命周期）  MenuBuilder（共用菜单）     │
│  Notifier（低电通知）  Autostart  SingleInstance            │
├──────────────────────────────────────────────────────────┤
│ 引擎层                                                     │
│  Enumerator（HID 枚举/接收器聚合）  Scheduler（轮询调度）      │
│  DriverRegistry（匹配/优先级）     HistoryStore（采样/压缩）   │
│  Estimator（续航估算）             PluginHost（声明式/原生）   │
├──────────────────────────────────────────────────────────┤
│ 驱动层                                                     │
│  内置驱动（每品牌一文件） 声明式驱动包（TOML） DLL 插件（v2）   │
├──────────────────────────────────────────────────────────┤
│ 平台层                                                     │
│  hid.rs（hidapi）  win32/（Shell / Tray / Menu / Registry）  │
└──────────────────────────────────────────────────────────┘
```

**线程模型**：1 个 UI 线程（消息泵）+ 1 个轮询线程（严格串行，避免抢 HID 句柄）+ 可选 1 个插件宿主线程。驱动调用永远不阻塞 UI。

---

## 4. 插件化 / 扩展化方案

### 4.1 统一接口（内核只认这个）

```rust
pub struct DeviceInfo { pub vid: u16, pub pid: u16, pub product: String, pub serial: Option<String>,
                        pub usage_page: u16, pub usage: u16, pub path: String, pub via: Transport }

pub struct Reading { pub soc: u8, pub voltage_mv: Option<u16>, pub charging: Option<bool>, pub at: i64 }

pub enum DriverError { Unsupported, Io(String), Timeout, Malformed }

pub trait Driver: Send + Sync {
    fn id(&self) -> &'static str;                       // "logitech.hidpp"
    fn match_score(&self, d: &DeviceInfo) -> u8;        // 0=不匹配, 100=精确命中
    fn read(&self, s: &mut HidSession, d: &DeviceInfo) -> Result<Reading, DriverError>;
    fn caps(&self) -> Caps;                             // has_voltage / has_charging / needs_wake
}
```

### 4.2 三层扩展

**① 内置驱动（编译期，默认）**
- 每个品牌/协议一个文件：src/drivers/logitech_hidpp.rs、razer.rs、playstation.rs …
- 注册在一个显式的 const REGISTRY: &[&dyn Driver] 里（不用宏魔法，方便审计）。
- 适合复杂协议（多步握手、Feature Report、长帧）。

**② 声明式驱动包（TOML，用户可加，不执行代码）** ← **v1 必须做，这是扩展性的关键**
- 位置：%APPDATA%/BatteryHub/drivers/*.toml（只读加载，拒绝路径穿越）
- 覆盖“一问一答”型设备（原版 136 个驱动里约七成属于此类）。
- 示例：

```toml
schema = 1
id = "atk.mercury"
name = "ATK Mercury I"
match = { vid = 0x373B, pid = 0x107C, usage_page = 0xFF22 }
request = { report_id = 0x00, bytes = [0x02, 0x05, 0x00, 0x00], size = 64, kind = "feature" }
response = { report_id = 0x00, offset = 3, width = 2, endian = "little", scale = 0.5, clamp = [0, 100] }
charging = { offset = 5, mask = 0x02, when = 1 }
sleep = { no_reply_secs = 120, retry = 2 }
notes = "帧格式参考 docs/protocol-notes/atk_mercury.md"
```

- 校验器：加载时 schema 校验 + 边界检查（offset+width ≤ size），失败只记日志、不崩溃。
- 不支持的字段组合 → 拒绝加载并提示用户。

**③ 原生 DLL 插件（v2 可选，默认关闭）**
- ABI：extern "C" fn bh_plugin_v1() -> *const PluginVTable
- 只从 %APPDATA%/BatteryHub/plugins/ 加载，文件名白名单；设置里必须显式打开“允许原生插件”。
- 加载前计算 SHA-256 并记录到日志；不做自动下载。
- 用途：极复杂设备（需要长连接、异步事件的耳机）。

### 4.3 匹配与优先级
1. 声明式包与内置驱动统一算 match_score；同分时：内置 > 声明 > DLL。
2. 用户可“强制指定驱动”（设置里选），用于同名设备冲突。
3. 匹配结果缓存到设备 key，重连先走缓存，失败再全量匹配（省时间）。

### 4.4 安全边界（插件）
- 声明式包：无代码执行、无文件/网络 API，纯数据 → 最安全。
- DLL：进程内执行、无沙箱 → 默认关闭、需显式授权、日志留痕。
- 两种插件都不得让主程序崩溃：调用包 catch_unwind；DLL 超 2 秒熔断不等待。


---

## 5. 目录结构与文件预算

```text
batteryhub/
├─ Cargo.toml
├─ src/
│  ├─ main.rs                 ≤120   参数解析 / 单实例 / 启动
│  ├─ app.rs                  ≤250   生命周期、事件循环
│  ├─ config.rs               ≤250   INI 读写 + 迁移 + 默认值
│  ├─ autostart.rs            ≤120   HKCU Run + 便携模式
│  ├─ single_instance.rs      ≤80    命名互斥体 + 唤起旧实例
│  ├─ menu/
│  │  ├─ mod.rs               ≤120   菜单项定义（纯数据）
│  │  ├─ builder.rs           ≤250   托盘 / 悬浮窗共用的构建器
│  │  └─ handler.rs           ≤250   命令分发
│  ├─ tray/
│  │  ├─ mod.rs               ≤180   图标管理、气泡
│  │  ├─ icons.rs             ≤220   数字图标绘制（GDI）
│  │  └─ pin.rs               ≤120   首次运行固定到任务栏
│  ├─ widget/
│  │  ├─ mod.rs               ≤200   生命周期
│  │  ├─ attach.rs            ≤300   找 Shell_TrayWnd / 桥接类、建子窗口
│  │  ├─ layout.rs            ≤280   托盘 / 时钟 / 多显示器 / 竖排坐标
│  │  └─ paint.rs             ≤200   文本与徽标绘制
│  ├─ hid/
│  │  ├─ mod.rs               ≤150
│  │  ├─ enumerate.rs         ≤220   枚举 + 接收器聚合
│  │  └─ session.rs           ≤260   打开 / 重开 / 超时 / 独占处理
│  ├─ polling/
│  │  ├─ mod.rs               ≤180
│  │  ├─ scheduler.rs         ≤250   每设备节奏、退避
│  │  └─ state.rs             ≤150   在线 / 睡眠 / 离线判定
│  ├─ drivers/
│  │  ├─ mod.rs               ≤120   REGISTRY
│  │  ├─ logitech_hidpp.rs    ≤320
│  │  ├─ razer.rs             ≤300
│  │  ├─ playstation.rs       ≤260
│  │  ├─ xbox.rs              ≤180
│  │  ├─ bluetooth.rs         ≤200   Windows 蓝牙电量
│  │  └─ declarative.rs       ≤300   TOML 解释器
│  ├─ model/                  ≤120/文件
│  ├─ history/                ≤250/文件
│  ├─ estimate/               ≤250/文件
│  ├─ notify.rs               ≤150
│  ├─ i18n/                   ≤80/语言
│  ├─ ui/                     ≤200/页面
│  └─ win32/                  ≤200/模块
├─ plugins/examples/*.toml
├─ docs/protocol-notes/*.md
└─ tools/check-size.ps1
```

**门禁脚本** tools/check-size.ps1：
- 任何 src/**/*.rs > 400 行 → 退出码 1；
- 单目录 > 15 个文件 → 警告（提示拆分模块）；
- 单函数 > 60 行 → 警告（自写扫描器，30 行脚本搞定）。
挂到 CI，本地 pre-commit 也跑。

> 为什么坚持这条：原版单文件 76KB（widget.cpp）+ 92KB（main.cpp），改一行要读一小时；我们的目标是“任何一个文件 5 分钟能读完”。

---

## 6. 右键菜单方案（对齐 XiuxiTimer 的交互）

### 6.1 入口
- **托盘图标右键**（每设备图标 / 汇总图标）
- **任务栏 widget 右键**（整个区域）
- **设备卡片右键**（设置页/弹窗里）

三处共用同一个 MenuBuilder，按上下文注入不同项 → 不会出现“这里能点那里不能点”的不一致。

### 6.2 菜单结构

| 位置 | 菜单项 | 说明 |
|---|---|---|
| 1 | 标题行（禁用） | “电池 3 台 · 最低 18%” |
| 2 | 立即刷新 | 触发现有轮询 |
| 3 | 显示窗口 / 隐藏窗口 | 切换 |
| 4 | 设备子菜单 ▸ | 每设备：隐藏/显示、重命名、设图标、低电阈值、查看历史、校准 |
| 5 | 布局子菜单 ▸ | 靠左/靠右/时钟左侧；竖排；仅图标 / 图标+数字；多显示器选择 |
| 6 | 开机自启 | 勾选项，切换即写/删 Run 键 |
| 7 | 设置… | 打开设置窗口 |
| 8 | 检查更新… | 打开浏览器到 Releases（v1 不做自动更新） |
| 9 | 关于 | 版本 / 许可 / 日志目录 |
| 10 | 退出 | 确认一次（可配置“不再询问”） |

### 6.3 借鉴参考项目的细节
- **首次运行把托盘图标从 ^ 溢出区固定出来**：Shell_NotifyIcon + 读写通知区域设置；只做一次，之后永不覆盖用户手动改动（用 tray_pinned_v1 标记）。
- **调试开关**：--no-tray、--config <path>、--no-autostart、--debug（参考项目同款，便于排障）。
- **同一套菜单**：托盘图标与悬浮窗右键结果一致（参考项目就这么做，体验统一）。
- 不引入密码锁（那是家长控制场景）；但提供“隐藏所有托盘图标（只留任务栏）”来减少杂乱。

### 6.4 实现要点（Win32）
- CreatePopupMenu + TrackPopupMenuEx(TPM_RETURNCMD | TPM_RIGHTBUTTON)，在 WM_APP+1 处理命令。
- 高 DPI：菜单用系统默认字体；坐标用 GetCursorPos + MonitorFromPoint 保证贴边不越界。
- 键盘可达：菜单自带；托盘图标需 NOTIFYICON_VERSION_4 才能收到 WM_CONTEXTMENU。
- 暗色：菜单跟随系统（Win11 自动）；不做自绘，避免兼容性坑。
- 菜单项状态（勾选/禁用）每次弹出前重建，不做常驻菜单句柄。

---

## 7. 开机自启方案

### 7.1 默认行为
- **默认开启**（与 XiuxiTimer 一致），但首次运行欢迎页里明示并提供开关。
- 写入：HKCU/Software/Microsoft/Windows/CurrentVersion/Run，值名 BatteryHub，
  数据 = "C:/path/batteryhub.exe" --minimized。
- **绝不写 HKLM**、绝不请求管理员。

### 7.2 便携模式
- 检测顺序：命令行 --portable → 同目录存在 portable.flag → 配置 [system] portable=1。
- 便携模式下：不写注册表、配置落 exe 同目录、不自动检查更新。

### 7.3 生命周期
| 场景 | 处理 |
|---|---|
| 升级/移动 exe | 启动时比对 Run 值里的路径，不一致且当前是自启状态 → 自动修正 |
| 用户手动取消 | 菜单勾选状态读注册表为准，不自作主张 |
| 卸载 | 卸载器删除 Run 值 + 配置目录 + 插件目录（询问是否保留历史） |
| 写注册表失败 | 回退到启动文件夹快捷方式；再失败则提示并保持关闭 |
| 多用户 | 只影响当前用户（HKCU 天然如此） |

### 7.4 调试与排障
- --no-autostart：本次运行不改注册表（参考项目同款参数）。
- 启动日志记录：是否自启、Run 值原文、便携模式判定结果。


---

## 8. 核心数据与算法（摘要）

- **DeviceKey**：vid:pid:serial（有 serial 用 serial，否则用接收器路径哈希），保证“接收器换口 / 设备重连”仍是同一台。
- **采样**：只在“电量变化 或 15–20 分钟心跳 或 状态变化”落点；每设备保留 7 天；存储用 JSONL 追加 + 定期压缩（每设备一个文件，单文件 ≤ 2 MB 滚动）。
- **续航估算**（详见 docs/estimate.md）：
  - 前置过滤：剔除充电段、离线段、>4h 空洞、跳变（+1 抖动、-25 损坏）；
  - 估计器：RLS（λ=0.99）+ 台阶 Theil-Sen 双估计，两者分歧 >2 倍时标记“数据不足”；
  - 门槛：≥4 样本、≥2h、累计下降 ≥3%、速率落在 [0.02, 12] %/h；
  - 展示：只取整小时/天，附“最近 12 小时放电 X%/天 · N 个样本”；≤3 个变化点不显示。

---

## 9. 测试与验收

| 层级 | 做法 | 门禁 |
|---|---|---|
| 单元 | 解析器 / 估算器 / 坐标计算等纯函数 | core 模块覆盖率 ≥ 70% |
| 金帧 | 每个驱动一组“请求帧 → 响应帧 → 期望电量”固定用例（tests/frames/） | 新增驱动必须带 1 组 |
| 声明式包 | 非法 TOML / 越界 offset / 超长帧 → 必须优雅拒绝 | 注入测试 |
| 集成 | 无设备启动、拔插、睡眠唤醒、多显示器切换 | 手动清单 |
| 稳定性 | 72 小时常驻，内存增长 ≤ 5 MB | 发布前必跑 |
| 界面 | 截图对比：100% / 125% / 150% / 200% DPI、竖排、自动隐藏 | 人工比对 |
| 安全 | 无网络（v1）、无提权、插件越界拒绝、路径穿越拒绝 | 代码审查清单 |

---

## 10. 里程碑

| 阶段 | 交付物 | 验收标准 | 估时 |
|---|---|---|---|
| **M0 骨架** | 单实例、配置、托盘静态图标、菜单、自启开关、调试参数 | 双击即跑，托盘有图标，菜单能退出，自启可开可关 | 1 天 |
| **M1 首个驱动** | HID 枚举、会话管理、Logitech HID++ 或 Razer 之一、声明式 TOML 加载器 | 有线/无线各一台设备能读到百分比并刷新 | 2 天 |
| **M2 任务栏** | widget 子窗口挂载、多显示器/竖排、完整右键菜单、图标渲染 | 任务栏显示 🎧 30%，四种 DPI 不糊，三处菜单一致 | 3 天 |
| **M3 历史/估算/通知** | HistoryStore、Estimator、低电通知、校准向导 | 连续运行 3 天能给出“约 X 天”且不胡报 | 3 天 |
| **M4 生态** | 内置驱动 8–10 个、i18n（中/英）、安装器 + 便携包、文档 | 新设备只需加 TOML；安装包 ≤ 3 MB | 3 天 |

**总估时：约 12 天（单人，含自测）**。M0–M2 做完即可日常使用。

---

## 11. 风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| 任务栏内部结构随 Windows 更新变化 | widget 失效 | 类名/坐标集中在 attach.rs + layout.rs，启动时版本探测，失败降级到托盘 |
| 驱动无限膨胀导致大文件 | 违背纪律 | 一座一文件 + 声明式包优先 + CI 行数门禁 |
| 协议逆向错误导致误写设备 | 设备异常 | 只发“查询帧”；写入仅在用户显式“校准”时发生；先只读试探 |
| DLL 插件崩溃 | 主程序挂 | 默认关闭 + 超时熔断 + 日志 |
| 无签名安装包被 SmartScreen 拦 | 用户流失 | 文档说明 + SHA256 + 后续可选签名 |
| 单人维护被 issue 淹没（原版教训） | 项目停摆 | 声明式包让用户自助加设备；issue 模板自动带设备清单 |

---

## 12. 需要你拍板的 4 件事

1. **语言**：Rust（建议：内存安全、长期驻留更稳、单 exe 约 0.5–1.5 MB）还是 C++（单文件更小、Win32 示例更多）？目录结构与分层两者通用，接口换成纯虚类即可。
2. **许可**：MIT（建议）还是 GPL-3.0？选 MIT 就必须严守第 2 节的洁净室纪律。
3. **插件范围**：v1 只做“内置 + 声明式 TOML”（建议），还是连 DLL 插件一起做？
4. **首个驱动**：先做你手上哪台设备？告诉我型号或 VID:PID，我把 M1 的验收目标直接定为它。

---

*本计划书为 v0.1 草案。确认以上 4 点后，我按 M0 开工，并把任务拆成可勾选的 todo。*

---

## 13. 实施记录

### M0（2026-11-23）✅

**已落地**

| 内容 | 位置 |
|---|---|
| 工作区（6 个 crate） | Cargo.toml |
| 插件 API（无平台依赖） | crates/bh-api/src/lib.rs |
| 宿主：配置 / 日志 / 单实例 / 自启 / 托盘 / 菜单 / 消息循环 | crates/bh-core/src/*.rs |
| 三个设备域插件：鼠标 / 键盘 / 笔记本 | plugins/bh-plugin-*/ |
| 架构决策记录 | docs/adr/0001-plugin-architecture.md |
| 文件大小门禁 | tools/check-size.ps1 |

**实测结果**

| 指标 | 结果 |
|---|---|
| cargo build（debug） | 通过，0 error |
| cargo build --release | 通过，**188 KB** |
| cargo test | 4 passed（配置往返、INI 解析、日期换算） |
| 自检 batteryhub.exe --self-test | SELFTEST OK |
| 冒烟：启动 → 托盘创建 → 常驻 | 进程存活，日志 "tray icon added" |
| 首启配置生成 | 自动写 config.ini（autostart=1, tray=1） |
| 文件行数门禁 | 18 个 .rs，最大 280 行，0 超限 |

**本阶段确定的两件事**

1. 插件粒度 = 设备域（mouse / keyboard / laptop），不是"每个品牌一个插件"；
   品牌协议放在协议库 crate，多个域共用（详见 ADR-0001）。
2. v1 用编译期 crate，v2 再上 sidecar（JSON-RPC）；不引入 WASM / dylib ABI。

**下一步（M1）**

- HID 枚举（hidapi）与 HidIo 实现；
- 第一个真实驱动：按你手上设备的 VID/PID 定；
- 声明式驱动包（TOML）+ 校验器；
- 轮询调度与"睡眠/离线"状态机。


### M1（2026-11-23）✅

**已落地**

| 内容 | 位置 |
|---|---|
| HID 枚举 + 读写适配（hidapi） | crates/bh-hid |
| 蓝牙电量（SetupAPI + DEVPKEY_Bluetooth_Battery） | crates/bh-core/src/bt.rs |
| 本机电池（GetSystemPowerStatus） | crates/bh-core/src/system.rs |
| 轮询线程 + 设备去重 + 强弱匹配 | crates/bh-core/src/poller.rs |
| Logitech HID++ 2.0 客户端（含 5 个单测） | crates/bh-proto-logitech |
| 蓝牙设备域插件 | plugins/bh-plugin-bluetooth |
| 调试命令 --once / --dump | crates/bh-core/src/probe.rs |
| 协议笔记（洁净室） | docs/protocol-notes/logitech-hidpp.md |

**实测（本机）**

| 项目 | 结果 |
|---|---|
| cargo test | 9 passed（核心 4 + HID++ 5） |
| --once | 本机电池 78%（充电中）✅ 端到端打通 |
| --dump | 33 个 HID 设备；识别到 Rapoo 24ae:185a 的 vendor collection ff00:0002 |
| 蓝牙节点 | 枚举正常（BTHENUM 节点 8 个）；本机设备未上报电量属性 |
| release 体积 | 270 KB |
| 行数门禁 | 27 个 .rs，全部 ≤400 行 |
| GUI 冒烟 | 托盘创建成功，4 个插件注册 |

**已知限制**

1. 蓝牙电量依赖 Windows 上报；老设备（如 2007 年 Apple 键盘）没有该属性。
2. Logitech 接收器多设备场景只取第一个应答槽位（M1.5 补配对表）。
3. 本机鼠标是 Rapoo（24ae:185a），协议未逆向 → 显示为未支持。
4. 托盘目前只有 tooltip 文本，数字图标在 M2。

**M2 计划**
- 任务栏文本 widget（Shell_TrayWnd 子窗口，多显示器/竖排/自动隐藏）
- 托盘数字图标（GDI 绘制）
- Rapoo 协议逆向（用 --dump + 帧捕获）


### M2 / M3 / M4（2026-11-23）✅

**M2 任务栏与图标**

| 内容 | 实现 |
|---|---|
| 任务栏文本 widget | 顶层 layered 弹窗 + 颜色键透明 + 点击穿透，覆盖在通知区左侧 |
| 位置计算 | 找 Shell_TrayWnd + TrayNotifyWnd，横排/竖排分别计算；自动隐藏与前台全屏时自动隐藏 |
| 托盘数字图标 | GDI 绘制：百分比 + 底部电量条，绿/琥珀/红/充电色 |

> 踩坑记录（三个）：
> 1. 本机 CreateWindowExW(WS_CHILD | WS_EX_LAYERED) 直接失败 → 改为顶层 layered 覆盖层。
> 2. SetWindowPos 必须带尺寸标志，否则窗口停在 10x10。
> 3. 颜色键（SetLayeredWindowAttributes）会让 ClearType 文字出现紫边，且与 UpdateLayeredWindow 互斥 → 改为"亮度当 alpha"的逐像素渲染（UpdateLayeredWindow + AC_SRC_ALPHA），文字干净无色边。

**M3 历史 / 续航 / 提醒**

| 内容 | 实现 |
|---|---|
| 采样历史 | 每设备一个 JSONL，变化或 15 分钟心跳才落点，保留 7 天（crates/bh-core/src/history.rs） |
| 续航估算 | 连续放电段 + 门禁（≥4 样本、≥2h、掉幅≥3%、速率 0.02–12%/h），输出"约 N 小时/天"（estimate.rs，5 个单测） |
| 低电提醒 | 托盘气泡，阈值可配（默认 20%），同设备 6 小时或再掉 5% 才重复 |

**M4 生态**

| 内容 | 实现 |
|---|---|
| i18n | 中/英两套字符串，按配置 lang 切换（menu/about/tooltip/提醒） |
| 安装/卸载脚本 | tools/install.ps1（复制 exe、开始菜单快捷方式、HKCU 自启）、tools/uninstall.ps1 |
| 排障命令 | --once / --dump / --watch vid:pid（只读抓报文，用于逆向新设备） |

**最终实测**

| 项目 | 结果 |
|---|---|
| cargo test | 16 passed（核心 11 + HID++ 5） |
| release 体积 | 320 KB |
| 行数门禁 | 36 个文件，0 超限（app.rs 359 行，触发 300 行警告） |
| 运行 | 托盘图标 + 任务栏文本 + 本机 78% 读数 + 历史落盘 全部验证 |
| widget 渲染 | PrintWindow 抓图确认绘制了电量文本 |

**仍未做（需要硬件配合）**

1. Rapoo（24ae:185a）私有协议逆向：需要 watch 时开关鼠标或正常使用，抓到帧后写 bh-proto-rapoo。
2. Logitech 接收器多设备配对表（HID++ 1.0 寄存器）。
3. 多显示器任务栏（现在只挂主任务栏）。
4. 校准向导 UI（现在只有 CLI 抓帧）。


### M5（2026-11-23）✅ Rapoo 驱动接入

| 内容 | 结果 |
|---|---|
| 协议 | `bb b0 42 80 0c 01 55`（7 字节被动帧），state=1/2，percent=0x55 |
| 实现 | crates/bh-proto-rapoo（6 个单测用公开捕获帧） |
| 实测 | `--once` → Rapoo Gaming Device mouse **85%**，耗时 1.2s |
| 两个坑 | ① 标准鼠标集合被系统独占（拒绝访问），必须开 `FF00:0002`；② 同 VID:PID 多 collection 必须按 usage 精确匹配 |
| 抓帧工具 | `--watch vid:pid` 抓到真实帧；`--capture-report` 自动分析候选电量字节 |

**仍未做**：多显示器任务栏、Logitech 接收器多设备配对表、校准向导 UI。


### M6（2026-11-23）✅ 开发日志功能

| 内容 | 说明 |
|---|---|
| 分级 | error / warn / info / debug / trace |
| 标签 | [app] [poll] [tray] [widget] [capture] [config] [menu] [battery] [ui] |
| 时间 | ISO8601 + 毫秒 + 时区 |
| 滚动 | 1 MB × 4 个文件 |
| 控制 | `--log-level`、菜单开关、配置持久化 |
| 结构 | 顺带拆分 state / runtime / commands，app.rs 371 → 184 行 |
| 校验 | 41 个文件 0 超限；24 个测试全绿 |

### M7（2026-11-25）✅ 竖排状态面板（替代任务栏文本）

**需求**：除电量外显示温度（CPU/内存/GPU/硬盘这类整机指标）；任务栏文本不再需要；
内容变多改成类似右键菜单的竖排面板；引入图标；显示品牌/型号。

**决策**

1. 任务栏文本 widget 整个下线（`widget/` 模块与 `widget` 配置项删除），所有信息进面板。
2. 入口：托盘左键/右键都弹面板；旧的原生右键菜单保留在面板「更多设置…」里，
   托盘图标开关、日志级别、图标样式等老功能不丢。
   - **2026-11-26 修正**：左右键拆开——**左键只开只读信息面板，右键直达原生菜单**；
     面板里的动作区（含「更多设置…」）已整体移除，面板不再有可点行。详见 DEVLOG 同日条目。
3. 面板是自绘分层窗口：圆角、图标、两行文字、右对齐数值、悬停高亮；
   点外面 / `Esc` / 失焦收起，`↑`/`↓` + `Enter` 可用，行为对齐原生菜单。
4. 温度/占用拿不到就显示 “—”，不为了读数引入内核驱动或提权。

**数据源（全部用户态、无需管理员）**

| 指标 | 实现 |
|---|---|
| CPU 占用 | `GetSystemTimes` 两次采样差值 |
| CPU 温度 | PDH `\Thermal Zone Information(*)\High Precision Temperature`（0.1 K，取最大热区） |
| 内存 | `GlobalMemoryStatusEx` |
| 系统盘 | `GetDiskFreeSpaceExW` |
| GPU 名称/温度 | D3DKMT `D3DKMTEnumAdapters2` + `KMTQAITYPE_ADAPTERREGISTRYINFO` / `_ADAPTERPERFDATA` |
| GPU 占用 | PDH `\GPU Engine(*)\Utilization Percentage`，按 LUID + engtype 汇总 |
| 品牌/型号 | `HKLM\HARDWARE\DESCRIPTION\System\BIOS` / `CentralProcessor\0`（读注册表，不走 WMI） |
| 外设电量 | 已有设备域插件 |

**代码**

| 内容 | 位置 |
|---|---|
| 系统采样线程（2s，UI 只读快照） | `crates/bh-core/src/sysmon/`（cpu/mem/disk/pdh/gpu/kmt） |
| 品牌型号清洗 | `crates/bh-core/src/machine.rs` |
| VID→品牌 | `crates/bh-core/src/brand.rs` |
| 面板（生命周期/布局/绘制/图标/配色） | `crates/bh-core/src/panel/`（9 个文件） |
| 探针 | `--sysmon` 打印一次采样；`--panel` 直接弹面板 |

**实测（ASUS Zenbook 14 / Core Ultra 9 185H / Intel Arc）**

| 项目 | 结果 |
|---|---|
| CPU | 20–30% · 79–89°C（ACPI 热区） |
| GPU | Intel Arc Graphics，占用 16–50%；温度驱动不报 → “—” |
| 内存 / C 盘 | ~90% / 67% |
| 设备 | 本机 79%⚡、Rapoo 鼠标/键盘待上报（识别到但暂无电量） |
| 面板交互 | 悬停/点击/Esc/托盘重开/点内边距收起/退出 全部实测通过 |
| 门禁 | 57 个 `.rs` 全 ≤400 行；34 个单测全绿；`--self-test` OK |

**踩坑**

1. PDH 速率计数器要采两次才有效，首帧 0 值不能当真。
2. `D3DKMTEnumAdapters2` 第一次返回的是上界（34），第二次才给真实数量（3），必须按两次结果重取。
3. 不同 DPI 感知级别下 `GetWindowRect`/`GetCursorPos` 坐标会缩放；截图与自动化要统一 DPI 感知，
   或直接看日志里的 `panel open at x,y`。

**仍未做**：Logitech 接收器多设备配对表、校准向导 UI、磁盘 SMART 温度（需要管理员，暂不做）。

### M8（2026-11-26）✅ 面板改「温度」：去占用、加硬盘温度

**需求**：用户反馈「面板上 GPU / 内存 / 系统盘显示的是资源占用，我想看的是温度；占用有任务管理器」。
→ 面板定位从「整机状态」收窄成「温度」，占用百分比不上屏。

**决策**

1. CPU / GPU / 硬盘三行只留温度；采样层仍保留占用，只在 `--sysmon` 排障时打印。
2. **内存行整行删除**：板载内存没有温度传感器，内存占用又明确不归这个面板。
3. 硬盘温度补上，且仍然不需要管理员（推翻 M7 「要管理员，暂不做」的判断）。

**数据源变化**

| 指标 | 实现 |
|---|---|
| 硬盘温度 | `IOCTL_STORAGE_QUERY_PROPERTY` + `StorageDeviceProtocolSpecificProperty(50)` + NVMe `SMART/Health` log page，句柄 `\\.\C:`（`dwDesiredAccess = 0`）；20 秒缓存 |
| 其余 | 同 M7（CPU 温度 PDH 热区、GPU 温度 D3DKMT、外设电量插件） |

**实测（ASUS Zenbook 14 / Core Ultra 9 185H / Intel Arc / ZHITAI TiPlus7100 4TB）**

| 项目 | 结果 |
|---|---|
| CPU | 88–100 °C（ACPI 热区，负载上来就顶 TjMax → 面板常红） |
| 硬盘 | 48–50 °C（NVMe SMART 直接读，普通用户） |
| GPU | Intel Arc，驱动不上报 → “—” |
| 设备 | 本机 79%⚡、Rapoo 鼠标/键盘待上报 |

**踩坑**

1. 协议查询输入缓冲要 48 字节（8 + 40），给 44 字节报 `ERROR_INVALID_PARAMETER(87)`。
2. `ProtocolDataOffset` 参照点各家不同：本机回填 48，实际数据在 56；两个位置都试，谁解出 273–400 K 谁算数。
3. `Get-StorageReliabilityCounter` 要管理员，但直发 NVMe 日志页不要——别把两条路混为一谈。

**仍未做**：Logitech 接收器多设备配对表、校准向导 UI、GPU 核显温度（用户态无源，除非装内核驱动/读厂商 EC，明确不做）。

### M9（2026-11-26）✅ 托盘与面板状态色

**需求**：状态栏图标引入颜色——所有上报设备都 >80% 一个颜色、任何一台 <20% 一个颜色，
其余状态（充电、上次读数、无数据）一起定色。

**规则**（颜色只表达“要不要管”，不表达设备）

| 状态 | 判定 | 颜色 | 非颜色线索 |
|---|---|---|---|
| 充足 | 电量 >80% | 绿 | 长填充 / 数字 |
| 正常 | 20–80%，未充电 | 中性 | 中等填充 |
| 低电 | <= 低电阈值（默认 20%） | 红 | 短填充 + 整壳变红 |
| 充电 | 20–80%，在充 | 蓝 | 面板行 `⚡` |
| 上次 | 只有上次读数 | 灰 | 面板 `· 上次` |
| 无数据 | 读不到 | 灰 | 斜线 / `—` |

托盘汇总优先级：**低电 > 全部充足 > 充电 > 正常**；没有新鲜读数才退到上次 / 无数据。

**顺带修复**

1. 电量阈值从 15/40 统一到 80 / `low_battery`（默认 20），低电提醒与颜色同源。
2. 托盘图标改读 `SystemUsesLightTheme`，不再跟应用主题串色。
3. 无读数画斜线 / `—`，不再冒充 0%。

**落点**：`crates/bh-core/src/status.rs`（状态 + 色板 + 对比度单测）、`tray/icons.rs`、`runtime.rs`、
`theme.rs`、`panel/model.rs`、`panel/theme.rs`、`examples/icons.rs`；方案见 `docs/status-colors.md`，
预览见 `docs/status-icons.png`。

**验证**：54 项单测全绿、clippy 0 error、60 个 `.rs` 0 超限；实机 79%⚡ 面板蓝字 + 托盘蓝图标。
