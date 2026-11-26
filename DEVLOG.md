# 开发日志（DEVLOG）

> 记录每个里程碑的**决策、踩坑、实测数据**。代码怎么写看源码，为什么这么写看这里。

## 2026-11-23 · 项目启动与 M0 骨架

- 决策：Rust + 原生 Win32（windows-sys），MIT，零第三方运行时。
- 决策：插件按**设备域**切分（mouse / keyboard / laptop / bluetooth），品牌协议放共享库 crate（ADR-0001）。
- 落地：workspace 6 个 crate、单实例、INI 配置、HKCU 自启、托盘 + 右键菜单。
- 数据：release 188 KB；文件行数门禁 400 行上线。

## 2026-11-23 · M1 真实读数

- HID 枚举（hidapi）、本机电池（GetSystemPowerStatus）、蓝牙（DEVPKEY_Bluetooth_Battery）、Logitech HID++ 2.0。
- 实测：本机电池 78%；枚举到 33 个 HID 设备；蓝牙节点 8 个但无电量属性（老设备）。
- 坑 1：`open` 只按 VID:PID 打开，命中错误的 collection（系统独占的鼠标接口 → 拒绝访问）。
- 坑 2：同一物理设备的多个 collection 需要按"插件匹配度"去重，否则读错接口。
- 数据：release 270 KB，9 个测试。

## 2026-11-23 · M2 任务栏与图标

- 目标：文字直接画进任务栏，而不是托盘图标。
- 坑 1：`CreateWindowExW(WS_CHILD | WS_EX_LAYERED)` 在本机直接失败 → 改为**顶层 layered 覆盖层**。
- 坑 2：`SetWindowPos` 漏掉尺寸标志 → 窗口一直 10×10。
- 坑 3：颜色键透明让 ClearType 文字出现**紫边**，且与 `UpdateLayeredWindow` 互斥 → 改成"亮度当 alpha"的逐像素渲染。
- 验证方式：`PrintWindow` 抓控件图 + DPI-aware 屏幕截图（普通 PowerShell 是 DPI 虚拟化的，会截错区域）。
- 数据：release 320 KB。

## 2026-11-23 · M3 历史 / 续航 / 提醒

- 历史：每设备 JSONL，变化或 15 分钟心跳落点，7 天保留。
- 估算：只取"连续放电段"，门禁 = ≥4 样本、≥2h、掉幅≥3%、速率 0.02–12%/h。
- 低电：托盘气泡，阈值 20%，同设备 6 小时或再掉 5% 才重复。
- 决策：宁可显示"数据不足"，也不给一个看起来很准的假数字。

## 2026-11-23 · M4 生态

- 中英 i18n（菜单/关于/提示）；安装与卸载脚本；`--watch` 只读抓帧。
- 决策：不做自动更新（未签名更新通道风险高于收益）。

## 2026-11-23 · M5 Rapoo 实测接入

- 现象：本机鼠标 `24AE:185A` "Rapoo Gaming Device"，被动监听 180 秒无帧；feature report 扫 256 个 ID 无响应。
- 破局：公开逆向资料给出两种布局（`07` 帧 / 旧版 `BB` 7 字节帧）；实现解析器 + 6 个单测。
- 再监听，抓到真实帧：`bb b0 42 80 0c 01 55` → state=1（放电）、percent=0x55=85%。
- 坑：标准鼠标集合 `0001:0002` 被系统独占（ReadFile 拒绝访问），必须精确打开 `FF00:0002`。
- 结果：`--once` → 本机电池 78%、鼠标 85%，耗时 1.2 秒。
- 副产品：自动抓帧（未知厂商集合常开监听）+ `--capture-report` 自动找候选电量字节。

## 2026-11-23 · 开发日志功能（本轮）

- 新日志格式：`2026-11-23T22:47:49.283+08:00 [INFO ] [poll] -> [laptop:78%, mouse:86%]`（毫秒 + 模块标签）。
- 级别：error < warn < info < debug < trace；启动可用 `--log-level`，菜单可切换"调试日志"，配置持久化。
- 滚动：单文件 1 MB，保留 `app.log.1/.2/.3`。
- 代码结构：日志升级顺手拆了 `state / runtime / commands` 三个模块，`app.rs` 从 371 行降到 184 行。
- 数据：41 个源文件，全部 ≤400 行（0 警告）；测试 24 个全绿。

## 2026-11-23 · 键盘支持

- 设备的真实情况（`--dump` + PnP 枚举）：雷柏接收器 `24AE:185A` 同时暴露鼠标通道 `FF00:0002` 和键盘通道 `FF00:000E`；另有一把蓝牙键盘 `05AC:024F`（Apple/Broadcom）。
- 关键改动：**每条 HID collection 独立成条目**（key 里带 usage page/usage）。以前按 VID:PID 去重会把键盘通道吞掉。
- 键盘插件：Rapoo 键盘通道接 `bh-proto-rapoo`，读不到就显示"等待设备上报（用一下设备）"；Logitech 键盘仍走 HID++。
- 蓝牙键盘：Windows 没有 `DEVPKEY_Bluetooth_Battery`（已用 PnP 逐设备验证），软件层拿不到电量 —— 这是设备/协议限制，不是代码问题。
- 实测：`--once` → 本机 78%、鼠标 85%、键盘"等待上报"；打字后下一轮轮询会读到（句柄常驻，帧被 Windows 缓存）。
- 追加：设备静默时保留**上次读数**（30 分钟内，标记为"上次"），任务栏显示 `鼠85%` 而不是变成"—"；键盘无数据时显示 `键—` 提示用户它被识别到了。

## 2026-11-23 · 图标与任务栏视觉重做

- 托盘图标：改 4× 超采样 + "遮罩/颜色"双通道合成（alpha 来自遮罩，颜色来自彩色通道），无紫边/黑底；三种样式（电池 / 数字 / 圆环），菜单可切换，配置持久化。
- 任务栏文本：从"本78%↑"纯文字改为**胶囊**：设备标签 + 彩色电池 glyph + 百分比，按 DPI 缩放（13px×缩放比）。
- 关键 bug（花了一小时）：进程不是 DPI-aware，导致 150% 缩放下算出来的坐标被系统再放大一次，窗口落到屏幕外 → 任务栏上什么都看不到。修复：启动时 SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)。
- 另一个坑：UpdateLayeredWindow 与 SetLayeredWindowAttributes 互斥；且 StretchBlt 的源必须是"选了位图的 DC"，不是位图句柄。
- 验证方法：把窗口内容 PrintWindow 成 PNG，再与真实屏幕截图比对。

## 2026-11-26 · M8 面板改「温度」：去占用、加硬盘温度

- 需求澄清：用户在面板上看到 `CPU 23% · 78°C / GPU 16% · — / 内存 90% / 系统盘 67%` 这类行，明确说
  「我是想显示温度来的，资源占用自然有任务管理器去处理」。→ 面板定位从「整机状态」收窄成「温度」。
- 决策：
  1. 三行系统信息全部只留温度：CPU 温度、GPU 温度、硬盘温度；占用百分比从面板移除
     （采样层仍保留占用，`--sysmon` 排障时照旧打印，只是不再上屏）。
  2. **内存行整行删掉**：板载 LPDDR5 没有温度传感器，而内存占用又明确不归这个面板——留着只能放占用率或 “—”，两种都是噪音。
     顺带删了 `Icon::Memory` 与它的图标绘制（避免 dead_code）。
- 新能力：**硬盘温度，依旧不需要管理员**。之前 PLAN 里写「磁盘 SMART 温度要管理员，暂不做」，这次实测推翻了：
  - `Get-StorageReliabilityCounter` 这条路确实要管理员（普通用户 `MI RESULT 2 / Access denied`）——
    所以一开始以为没戏；
  - 但直接对设备发 `IOCTL_STORAGE_QUERY_PROPERTY` + `StorageDeviceProtocolSpecificProperty(50)` +
    NVMe `SMART/Health` log page，句柄用 `\\.\C:` 或 `\\.\PhysicalDrive0`（`dwDesiredAccess = 0` 即可），
    **普通用户能读到**：本机 4TB ZHITAI，返回 322 K = 49 °C，备件 100% / 磨损 4%，与盘况自洽。
- 坑：
  1. 输入缓冲必须带满 `STORAGE_PROPERTY_QUERY(8) + STORAGE_PROTOCOL_SPECIFIC_DATA(40)` = 48 字节，
     只给 44 字节会 `ERROR_INVALID_PARAMETER(87)`（驱动按结构体大小校验）。
  2. `ProtocolDataOffset` 的参照点各家实现不一致：本机回填 48，但实际数据在 `48 + 8`（相对 ProtocolSpecificData 起点）。
     代码里对 `offset` 和 `offset + 8` 两个位置都试一遍，谁解出 273–400 K 谁算数。
  3. 温度是开尔文、`0` 表示不支持；按 0.1 K 单位去解会把 322 K 读成 5 °C 之类的鬼数。
  4. 每 2 秒敲一次盘没必要：SMART 温度 20 秒缓存一次。
- 实测（本机）：CPU 88–100 °C（ACPI 热区，负载一上来就顶到 TjMax，红色是实情）、
  硬盘 48–50 °C（绿）、GPU “—”（Intel Arc 核显驱动就是不上报）。
- 截图验证：per-monitor aware 进程 + `FindWindowW("BatteryHubPanel")` 取窗口矩形 → `CopyFromScreen`，
  和 M2 的结论一致（不 set DPI awareness 会截错区域）。
- 门禁：64 个 `.rs` 全部 ≤400 行（`poller.rs` 337 行只报 warning）；66 个单测全绿（含 SMART 解码 2 个）；`--self-test` PASS。

## 2026-11-26 · M8 续：温度链路的性能账

- 先量再改。临时微基准（`#[ignore]` 测试，跑完即删，单独进程免得互相抢缓存锁）：

  | 环节 | 单次耗时 | 是否每帧都做 |
  |---|---|---|
  | `GetSystemTimes`（CPU 占用） | 4–12 µs | 是 |
  | `GlobalMemoryStatusEx`（内存） | 0.3 µs | 是 |
  | GPU 温度（D3DKMT 开/查/关 × 2 适配器） | ~1 µs | 是 |
  | PDH `PdhCollectQueryData`（只挂温度） | 87 µs | 是 |
  | PDH `PdhCollectQueryData`（带 `\GPU Engine(*)`） | 306 µs | 曾经是 |
  | `\GPU Engine(*)` 取值（523 个实例，两次取大小 + 逐个 parse LUID/engtype） | 595 µs | 曾经是 |
  | `GetDiskFreeSpaceExW` | 41–57 µs | 曾经是 |
  | NVMe SMART（一次 IOCTL） | **2.8 ms** | 20 秒一次 |

- 结论：面板已经不显示占用率了，采样线程还在为它付 ~0.9 ms/帧 → 直接摘掉。
  `Pdh::new(gpu_usage: bool)`：后台 `false`，`--sysmon` 走 `spawn_with_usage()` 仍然带；盘空间加 30 秒缓存
  （SMART 仍 20 秒缓存，2.8 ms 不能每帧付）。
- 效果：每帧 ~1.0 ms → ~0.2 ms；进程 CPU 0.16% → 0.05% 一个核（`Get-Process` 30 秒累计）。
  缓存命中实测 0.1 µs，`--sysmon` 输出不变。**绝对值本来就小**，但“为不看的数据付钱”没必要。
- 顺手找到真凶（与温度无关）：v1.0.0 的常驻进程 60 秒吃 59.9 秒 CPU，热线程是 capture 抓帧线程，
  加 `--no-capture` 立刻掉到 0.08%——抓帧是「读不到数据也不睡」的空转。
  另一条线同时在重写 capture（VID 白名单 + 空闲退避，默认不抓），改后整机实测 0.16%（抓帧关闭时）；
  温度链路优化后 0.05%。结论：温度采样从来没吃过 CPU，别在它身上找性能问题。

## 2026-11-25 · M7 竖排状态面板

- 需求转向：用户不再需要任务栏文本，要的是「除电量外还能看温度 / CPU / 内存 / GPU / 硬盘」，内容变多后用竖排右键菜单式面板承载，并要图标与品牌型号。
- 决策：删除任务栏 widget（`crates/bh-core/src/widget/` 整个模块与 `system.widget` 配置项），托盘左/右键统一弹出面板；原生菜单保留在面板的「更多设置…」里，不丢旧功能。
- 采样（全用户态、无管理员、无内核驱动）：
  - CPU 占用 `GetSystemTimes` 差值；内存 `GlobalMemoryStatusEx`；系统盘 `GetDiskFreeSpaceExW`；
  - CPU 温度 PDH `\Thermal Zone Information(*)\High Precision Temperature`（0.1 K，取各热区最大值）；
  - GPU 型号/温度 D3DKMT：`D3DKMTEnumAdapters2` + `KMTQAITYPE_ADAPTERREGISTRYINFO`，打开 LUID 后查 `KMTQAITYPE_ADAPTERPERFDATA`；
  - GPU 占用 PDH `\GPU Engine(*)\Utilization Percentage`，按实例名里的 LUID + engtype 汇总后取最忙引擎；
  - 品牌/型号/CPU 型号读 `HKLM\HARDWARE\DESCRIPTION\System\BIOS` 与 `CentralProcessor\0`，清洗 OEM 占位串与重复后缀。
- 独立 `bh-sysmon` 线程每 2 秒采一次，UI 只读快照，面板弹出不阻塞；面板打开时每秒重建内容。
- 实测（本机 ASUS Zenbook 14 / Intel Core Ultra 9 185H / Intel Arc）：
  - CPU 20–30% · 79–89°C；GPU 占用 16–50%；内存 ~90%；C 盘 67%；
  - GPU 温度驱动返回 0 → 面板显示 “—”（Intel 核显常见），`--sysmon` 会明确提示；
  - 鼠标/键盘“待上报”而不是 “—”，用户知道设备已被识别。
- 三个坑：
  1. PDH 速率型计数器必须采两次（先 `collect`，600ms 后再采），否则首帧全是 0；
  2. `D3DKMTEnumAdapters2` 第一次给的 `NumAdapters` 是上界（本机 34），第二次才回真实 3 块，必须按两次返回重新取；
  3. `GetWindowRect` / `GetCursorPos` 在不同 DPI 感知级别的进程里坐标会“跳”，截图与自动化最好统一成 per-monitor aware，或直接读面板日志里的 `open at x,y`。
- 面板实现：2× 超采样 GDI → 圆角背景 + 图标 + 两行文字 + 右对齐数值；颜色只表达语义（绿/琥珀/红/灰）；点外面 / Esc / 失去捕获即收起；上下键 + 回车可用。
- 门禁：57 个 `.rs` 全部 ≤400 行（面板拆成 9 个文件）；34 个单测全绿；`--sysmon` / `--self-test` 实测通过。
- 待办：Logitech 接收器多设备配对表、校准向导 UI、磁盘 SMART 温度（要管理员，暂不做）。

## 2026-11-25 · 安装版收尾：GUI 子系统 + 安装脚本编码

- 现象：`tools\install.ps1` 装完后从开始菜单/开机自启启动，会多出一个 Windows Terminal 窗口；
  `--panel` 调试时面板刚弹出就被这个终端抢走焦点，按「失活即收起」立刻隐藏，看着像面板坏了。
- 根因 1：`bh-app` 是**控制台子系统**（PE Subsystem=3），M2 计划里的「打包时改成 windows 子系统」一直没落地。
- 修复 1：`crates/bh-app/src/main.rs` 改 `#![windows_subsystem = "windows"]`；新增
  `crates/bh-core/src/console.rs`，只在「带参数启动 + std 句柄不可用」时才
  `AttachConsole(ATTACH_PARENT_PROCESS)` 并重绑 `CONOUT$`/`CONIN$`。被重定向（管道/文件）时不动句柄，
  所以 `--self-test` / `--sysmon` 的输出照样能被管道捕获。
- 根因 2：`tools/install.ps1`、`tools/uninstall.ps1` 是 UTF-8 **无 BOM**，Windows PowerShell 5.1 按 ANSI
  解码，`$sc.Description` 那行的中文会把后面的引号吃掉 → 整个脚本解析失败。加 BOM 后 5.1 / 7 都能跑。
- 验证：`Subsystem=2`；`--self-test` 全 PASS，`--sysmon` 正常；`cargo test` 45 项全绿；
  `install.ps1` 装到 `%LOCALAPPDATA%\Programs\BatteryHub` + 开始菜单快捷方式 + HKCU Run（指向安装路径）；
  启动后无终端窗口，托盘图标注册且 tooltip 随轮询更新
  （`BatteryHub — 本机 78%⚡ · 鼠标 — · 键盘 —`），面板截图正常（CPU 30%·87°C / GPU 63% / 内存 85% / C: 66% / 电池 78%）。
- 注意：Win11 默认把新的托盘图标收进「显示隐藏的图标」溢出区，想常驻要手动拖到任务栏。

## 2026-11-26 · 左右键分工：左键看信息，右键做功能

- 反馈：托盘左键和右键都弹同一个面板，面板里又塞着「立即刷新/开机自启/关于/退出」，
  “看信息”和“做功能”混在一起，分不清哪个键是干什么的。
- 决策：**左键 = 只读信息面板，右键 = 原生菜单**，两边不混。
  - 左键：品牌型号 + 系统状态（CPU/GPU/内存/磁盘）+ 设备电量，纯展示。
  - 右键：立即刷新 / 打开日志目录 / 开机自启 / 托盘图标开关 / 图标样式 / 调试日志 / 关于 / 退出。
- 面板瘦身：删掉整块动作区（含「更多设置…」），连同行悬停高亮、`↑`/`↓`/`Enter` 键盘导航、
  `Row.action` / `Row.checked`、`CMD_MORE`、`panel.more` 文案、Refresh/Power/Gear/Info/Exit/Check
  六个图标和 `Tone::Neutral` 一起下线；面板里不再有任何可点的行。
- 收起策略改为“失活即收”为主：拿得到前台就靠 `WM_ACTIVATE` 收，拿不到前台才退回 `SetCapture`。
  这一步顺带修掉了「面板开着时点托盘右键要按两次」——以前鼠标捕获会把那次右键吃掉。
- 坑：真实点击是“按下”先让面板失活收起、“抬起”才回到托盘，同一次点击会把面板关了又开。
  用 400ms 的 `CLICK_GRACE` 吸收那次抬起（`toggle` 里比对最近一次自动收起的时刻）。
- 验证（真鼠标点托盘图标，不是投消息）：左键开 → 再点收 → 再开；面板任意处左键收；
  失活收；面板上右键 → 面板收 + 原生菜单在光标处；右键不再弹面板。8 项断言全 PASS，
  截图见 `tmp/menu-1-panel.png` / `tmp/menu-2-right-menu.png`，README 配图已换成新版面板。
- 门禁：58 个 `.rs` 全 ≤400 行（`panel/mod.rs` 332 → 244 行、`model.rs` 349 → 279 行）；
  46 个单测全绿；0 警告。

## 2026-11-26 · 托盘悬停提示只留标题

- 反馈：鼠标悬停托盘图标时，提示里塞着电量（`BatteryHub — 本机 78%⚡ · 鼠标 — · 键盘 —`）。
  电量图标上已经能看出来，提示里再报一遍纯属重复，还和左键面板打架。
- 决策：tooltip **只显示标题 `BatteryHub`**，电量 / 续航 / 缺数据全部进左键面板，提示里不出现任何数字。
  - `i18n` 新增 `tray.title`（中英同值 `BatteryHub`）；`tooltip.none`（「未发现设备」）和启动期的
    「正在扫描设备」两种临时文案一起下线——提示不再随读数变化。
  - `runtime::refresh()` 只保留“取最差电量 → 画图标”，`tip` 恒为标题；托盘新建（`app.rs` 启动、
    右键「显示托盘图标」重建）也走同一个 `tray.title`，三处口径统一。
- 保留：`Monitor::estimate_for` / `tooltip_suffix`、`DeviceReading::summary` 没删，续航估算仍按 M3
  逻辑落库，留给以后的面板行用（当前不在提示里展示）。
- 验证：`cargo test` 46 项全绿、0 警告；`--self-test` PASS；`tools/check-size.ps1` 58 文件 0 超限；
  以 `--log-level debug` 实跑，日志里是 `[ui] refresh: BatteryHub`（不再带百分比）；
  `install.ps1` 重装到 `%LOCALAPPDATA%\Programs\BatteryHub` 并重启，新进程已接管托盘。

## 2026-11-26 · 状态色：80 / 20 分级 + 充电 / 上次 / 无数据

- 需求：状态栏图标想引入颜色——所有上报设备都 >80% 一个颜色、任何一台 <20% 一个颜色，
  并把其它状态的颜色也研究补齐。
- 调研：Windows 11 新电池图标（绿 = 充电且状态良好、黄 = 节能 ≤20%、红 = 危急、正常中性）、
  Android 15 同一思路；WCAG 非文本对比度 3:1。结论：颜色只表达“要不要管”，正常档故意不上色。
- 决策：六态 + 汇总优先级：
  - 单设备：`Good >80%` 绿、`Normal 20–80%` 中性、`Low <= low_battery` 红、
    `Charging 20–80% 且在充` 蓝、`Stale 只有上次` 灰、`Unknown 读不到` 灰。
  - 托盘汇总：**低电 > 全部充足 > 充电 > 正常**；没有新鲜读数才退到上次 / 无数据。
  - 低电优先于充电（15% 还在充也先红），充足优先于充电（都 >80% 时插电也保持绿）。
- 两个口径修复：
  1. 阈值从 15/40 改成 80 / `low_battery`（默认 20）——原来低电提醒响了，图标还是琥珀；
  2. 托盘图标改读 `SystemUsesLightTheme`（原读 `AppsUseLightTheme`）——应用浅色 + 任务栏深色会画反。
- 细节：低电整壳变红（16px 下填充只剩一两像素）；无数据画空心斜线 / “—”，不再冒充 0%；
  面板设备行复用同一个 `status` 判定，充电行蓝字 + `⚡`。
- 验证：`cargo test` 54 项全绿（含阈值 / 优先级 / 对比度 6 个）；clippy 0 error（warning 为既有技术债）；
  60 个 `.rs` 0 超限；实机 79%⚡ → 面板蓝字 + 托盘蓝图标（`tmp/panel-status.png`、`tmp/tray-zoom.png`）；
  6 状态 × 3 样式 × 明暗预览见 `docs/status-icons.png`；方案与备选见 `docs/status-colors.md`。

## 2026-11-26 · 厂商协议注册表 + 修掉 100% CPU 空转

- 两个问题一起收口：①`capture` 线程吃满一个 CPU 核；②"说好的插件化"只做到域，没做到厂商。
- **空转事故（实测数据）**：进程 852 秒里单线程吃 842 秒（**99.6% 单核**）、`ReadFile` 累计 **3.28 亿次 ≈ 38.6 万次/秒**，
  而 `capture/` 目录一帧都没落盘。根因是 `capture` 循环体**没有 sleep**，且把读错误用 `if let Ok(n)` 吞掉，
  失败立即重试 → 死循环；即使 `handles` 为空也照样空转（结构性缺陷）。
  - A/B 对照（同一 exe，20 秒窗口）：默认 `20.12s CPU / 28.4 万次读/秒` ↔ `--no-capture` `0.03s / 0 次`。
  - 修复后同条件实测：capture 关 **0 秒 / 0 次读**；capture 开（白名单 `vids=093a`）**0.02s/12s ≈ 0.1% 单核**。
  - 三条硬约束写进模块注释：循环必睡、失败即摘句柄并隔离 60s、只抓显式白名单 VID。
- **抓帧改成 opt-in**：配置 `[capture] vids=09da,24ae` 才启动，默认一个句柄都不开（旧行为对全系统厂商集合"顺便"开句柄）。
- **架构（ADR-0002）**：新增厂商协议注册表 `bh-core::registry` + `bh-api::protocol`（`VendorProtocol` / `Endpoint` / `Mode` / `DeviceTable`）。
  宿主一次枚举产出快照统一下发；`VID → 协议` 索引 O(1)；加品牌 = 加一个 `bh-proto-*` crate + 注册表一行。
  `bh-plugin-mouse` / `bh-plugin-keyboard` 里写死的 `0x046D` / `0x24AE` 分支全部删掉（两个 crate 也不再依赖协议库）。
- **轮询改造**：一轮只枚举一次（旧实现每设备再枚举一次）；`std::thread::scope` 并行读（8.3s → 6.6s，且与设备数无关）；
  `ProbeState` 退避 1/2/5 分钟，`立即刷新` 解除退避。
- **幽灵行抑制**：判定依据从"枚举到了"改成"成功读到过"（`DeviceReading::ghost`）。雷柏接收器插着但两条通道都无帧时，
  `--once` 会标「幽灵」，面板与托盘汇总都不再显示——用户插回接收器后确认"这次没有多出来的鼠标/键盘行"。
- 雷蛇协议（`bh-proto-razer`）：90 字节 feature report / class `0x07` / cmd `0x80`·`0x84` / 字节 2..88 异或校验，6 个单测。
  实测有线（1532:007a）不可读：MI_02 控制接口被厂商驱动占用（`0x1 函数不正确`），鼠标集合能写但一律回 `0x1F`；
  且**有线模式电量本身无意义** → 只认无线 dongle（1532:007b），复核工具 `--example razer_probe`。
- 门禁：`cargo test` 66 项全绿；clippy 0 error；64 个 `.rs` 0 超限（`poller.rs` 337 行进警告区）；`--once` 6.6s（旧 8.3s）。

## 2026-11-26 · 幽灵行可发现性补丁（不打版本号）

- 用户反馈：雷柏接收器还插着，但面板里没有雷柏键盘/鼠标了 —— 这是幽灵行抑制的预期行为。
- 但复查时发现一个体验缺口：**打开面板不会触发重试**，只有右键菜单「立即刷新」会。
  而被动协议已退避到 5 分钟封顶，等于每 5 分钟只有 6 秒读取窗口（约 2% 命中率），
  用户"把雷柏鼠标开机点两下再打开面板"大概率还是看不到。
- 两个补丁：
  1. `panel::show()` 里加一行 `poller::request_refresh()` —— 打开面板 = 强制重试一轮并解除退避；
     面板自身那个 1 秒 `WM_TIMER` 会自动把新读数画出来，所以最多 6 秒就能看见设备冒出来。
  2. 退避按 `Mode` 分开：`Feature` 仍 1→2→5 分钟阶梯；`Passive` 固定 60 秒
     （配合默认 `refresh_secs=60`，等于每轮都试一次，代价只是后台线程每轮多等 ≤6 秒）。
- 顺带：探测状态机抽到 `probe_state.rs`（含 `ok_once`/`fails`/`next_at` 与退避口径，4 个单测），
  `poller.rs` 回到 333 行（原先 408 行会撞 400 行门禁）；删掉无调用方的 `DeviceReading::summary()`。
- 实测：`--once` 6.2s（不变）；常驻 20 秒 CPU 0.000s；`cargo test` 71 项全绿；行数门禁 65 文件 0 超限 0 警告。
- 待观察：14:46 起的安装版实例首轮之后 7 分钟没再跑出第二轮（CPU 0.30s、线程 7）。同一二进制两次 160 秒复现
  （`--no-tray` / `--tray --panel`）都是稳定 66 秒一轮，未能复现；推测是 hidapi 超时后的取消未返回导致读卡在驱动里。
  再出现就把「一轮」放独立线程 + 硬超时放弃。详见 `gits/20261126-ghost-ux/decision-chain.md` D4。
- 按用户要求**不发新版**（版本仍是 1.1.0，未打 tag、未动 Release）；本机已重装同一版本。

## 2026-11-26 · exe 身份补齐：图标 / 版本信息 / 清单 + 托盘健壮性

- 起因：用户问“应用是不是没有图标”。实测确认：`target\release` 与安装版 exe 的**资源目录 RVA = 0**——
  一个资源都没有：无图标、无版本信息、无应用清单。`SHGetFileInfo` 取到的就是 Windows 通用白窗图标；
  `(Get-Item).VersionInfo` 全空；运行中的进程压根没加载 comctl32 v6（原生菜单因此是经典样式）。
- 根因：仓库里没有 `build.rs` / `.rc` / `.ico`，Cargo 默认不给 exe 编任何资源；发布流水线只算 SHA256，从不检查资源。
- 修复（身份层）：
  - `assets/batteryhub.ico`：竖向绿色电池（`#6CCB5F` = 状态色板里的 Good 深色），16/20/24/32/40/48/64/128/256 九档。
  - `crates/bh-app/build.rs`：零第三方依赖，自己找 Windows SDK 的 `rc.exe` 把 ICON + VERSIONINFO + MANIFEST 编进 exe；
    版本号从 `CARGO_PKG_VERSION` 生成，不会再和 `Cargo.toml` 漂移。找不到 `rc.exe` 只警告不失败（源码构建不受影响）。
  - 清单：comctl32 v6（菜单跟随系统主题）、PerMonitorV2、Win10/11 supportedOS、longPathAware、asInvoker。
  - 坑 1：`.rc` 里的路径必须转义反斜杠，否则 `\b` `\t` 会被当转义序列；坑 2：中文 `FileDescription` 必须 `#pragma code_page(65001)`。
- 修复（运行时）：
  - `TaskbarCreated` → 重新 `NIM_ADD`：资源管理器重启 / 崩溃恢复后，托盘图标不再永久消失。
  - 第二个实例不再静默退出，而是 `FindWindowW` + `PostMessage` 把面板叫出来——既是“托盘被关掉”之后的恢复入口，
    也修掉“双击 exe 毫无反应”的老毛病。关托盘图标时会明确提示怎么找回来。
  - 窗口类 `hIcon` 与托盘兜底图标改成内嵌图标（`LoadIconW(hinst, 1)`）。
- 字形：托盘电池由横向改竖向，与应用图标同构；状态色语义完全不变（低电红壳、无数据斜线）。
  按用户反馈又**加宽了一版**：壳宽 35% → 46%、触点 17% → 22%、壳厚 5.5%——
  35% 那版 16px 下像根细管子，46% 在任务栏和资源管理器里都更结实。应用图标与托盘字形共用这套比例。
  `docs/status-icons.png` 已按新字形重新生成。
- 门禁：`tools/check-exe.ps1` 读 PE 资源目录，缺图标 / 版本信息 / 清单或版本号为空即失败；CI 与 release 都跑。
- 数据：release 470,528 → 500,224 B（+29.7 KB，主要是 24 KB 图标）；`cargo test` 71 项全绿；
  clippy 0 error（18 项既有风格 warning，未新增）；66 个 `.rs` 0 超限；`check-exe.ps1` 对新 exe 5 项 PASS、对旧 exe 5 项 FAIL。
- 实机验证：新 exe 起托盘后广播 `TaskbarCreated` → 日志 `taskbar recreated; re-adding icon`；
  再运行一次 exe → `wake from second instance: show panel`，面板在 (527,40) 弹出；
  `tools/install.ps1` 覆盖安装后，开始菜单快捷方式取到新图标（绿电池 + 快捷方式角标）。
- 注意：已发布的 v1.1.0 exe 不会自己变，要重新打 tag 才会更新下载页。

## 待办

- 多显示器任务栏（当前只挂主任务栏）。
- Logitech 接收器多设备配对表（HID++ 1.0 寄存器）。
- 校准向导 UI（当前只能 CLI 抓帧）。
