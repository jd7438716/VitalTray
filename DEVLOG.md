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

## 待办

- 多显示器任务栏（当前只挂主任务栏）。
- Logitech 接收器多设备配对表（HID++ 1.0 寄存器）。
- 校准向导 UI（当前只能 CLI 抓帧）。
