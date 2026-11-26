# Battery Hub 深度评估：Windows 任务栏无线设备电量工具

> 评估对象：网传「把鼠标/键盘/耳机/手柄电量显示在 Windows 任务栏」的开源小工具
> 评估日期：2026-11-23 ｜ 评估方式：开源软件七维分析法（溯源消歧 → 代码审计 → 依赖与供应链 → 发布可信度 → 权限与隐私 → 社区健康度 → 口碑三角验证）
> 评估结论时效性：强（GitHub 状态、源码、依赖库均为当日实测）

---

## 0. 一页结论（TL;DR）

**确实有这个工具，但"Battery Hub"不是一个项目，而是至少三个互相独立、同名同功能的项目。** 媒体报道的那个（也是你大概率听说的那个）现在**在 GitHub 上已经 404**。

| | A：媒体报道原版 | B：同名 Electron 版 | C：同名 Rust 版（独立作者） |
|---|---|---|---|
| 仓库 | `krisgfx9/Wireless-Device-Battery-Monitor`（**今日 404**） | `battery-hub/battery-hub`（**活跃**） | `ZGMFX01A/Wireless-Device-Battery-Monitor` |
| 技术栈 | C++ / Win32，无运行时 | Electron 45 + Node.js | Rust |
| 体积 | **约 350 KB，内存约 2 MB** | 安装包 **107 MB**，常驻内存约 **290 MB**（作者自述） | 未测 |
| 许可证 | **GPL-3.0**（中文媒体多误写为 MIT） | MIT | 未标 |
| 任务栏形态 | 真正画在任务栏通知区旁（🎧 30% 🖱 93%） | 系统托盘图标（每设备一个图标） | 托盘 |
| 设备覆盖 | **113 个品牌/设备条目**，20 种语言 | 12 个品牌族，6 种语言 | 较小 |
| 更新完整性 | **ECDSA P-256 签名校验**（更新被篡改则拒绝） | 无签名校验，仅靠 HTTPS + GitHub | 未知 |
| 社区 | 峰值约 **4.4k–5.8k star**、374 forks、2.8k→3.5k open issues、Discord | 28 star / 91 open issue（86 条是设备支持请求） | 47 star |

**安全性（依据代码实测，不是听媒体说）：**
- 两者都 **不需要管理员权限**、不装驱动、不装服务、**无遥测**，数据只落本地 `%APPDATA%` / `%LOCALAPPDATA%`。这一点是硬结论，已逐行核对 IPC、网络与文件写入路径。
- **A 版的更新机制更硬**：下载更新后先验证维护者 ECDSA 签名（公钥硬编码在 `src/update_public_key.h`），验签失败就拒绝安装。B 版没有这一步，仓库/CI 一旦被拿下，全量用户会在下次退出时静默装上攻击者的包（`autoInstallOnAppQuit = true`）。
- 两者共同的真实弱点：**安装包未做 Authenticode 代码签名**，SmartScreen 必然弹"Windows 已保护你的电脑"。这不等于有病毒，但它把"点仍要运行"变成了使用习惯——而这恰恰是同名克隆最容易被利用的地方。
- **当前最大风险不是代码，是"下错包"**：媒体文章里的链接已经 404，你在搜索引擎/软件站上找到的"BatteryHub 下载/汉化版/绿色版"，有相当概率是换了壳的第三方包（仓库消失后，已出现 6 个以上互不隶属的 fork 和网盘转载）。

**可用性：** A 版是"极致轻（约 350KB）+ 真任务栏 + 设备覆盖极广（113 条品牌/设备，20 种语言）"的路线；B 版是"现代 UI + 历史曲线 + 自动更新"的路线，代价是约 300 倍安装体积（107MB）与约 150 倍内存（约 290MB）——换来的是一个仍在维护、可 winget 安装、含 190 个自动化测试的项目。

**建议（简版）：**
1. 只是想知道电量 → **Windows 11 自带的"电池状态"小组件/蓝牙设备电量** 已经够用，零风险；
2. 一定要用这个小工具 → 用 **A 版**，从镜像仓库取源码自行编译，或核对 `SHA256SUMS.txt` + `.sig` 后再运行；
3. 装 B 版 → 只用 `winget install DORON177.BatteryHub`（winget 会校验哈希），并在设置里关掉自动安装更新。

---

## 1. 事实核查：你听到的"Battery Hub"到底是哪一个

### 1.1 同名多胞胎（实测）

| 项目 | 地址 | 状态（2026-11-23 实测） | 证据 |
|---|---|---|---|
| A | `github.com/krisgfx9/Wireless-Device-Battery-Monitor` | **HTTP 404**（仓库与作者主页 `github.com/krisgfx9` 同时 404） | 直接请求 + GitHub API 均返回 Not Found |
| A′ | `gitcode.com/gh_mirrors/wi/Wireless-Device-Battery-Monitor` | 可访问，最后同步提交 **2026-11-21 22:49**（`Merge PR #2784`） | 已 clone，339 文件 / 27k 行 C++ |
| B | `github.com/battery-hub/battery-hub` | 活跃，v2.0.4（2026-11-22） | GitHub API |
| B′ | `winget install DORON177.BatteryHub` | 已进 winget-pkgs，但**清单只到 2.0.1**，落后 3 个版本 | winget-pkgs 目录 `manifests/d/DORON177/BatteryHub/{1.2.0,2.0.0,2.0.1}` |
| C | `ZGMFX01A/Wireless-Device-Battery-Monitor`（Rust） | 独立项目，47 star，非 fork | GitHub API `parent: null` |
| D | `fuyue521/Wireless-Device-Battery-Monitor-CN`（汉化） | 9 star，2026-11-18 创建，描述写明"基于 krisgfx9/… 汉化" | 仓库描述 |
| E | `emincodesM/…-TURKCE`、`joaoarebalo/…-Fallen` | 第三方改版 | 搜索 |

**关键事实：A 是媒体（MakeUseOf、PC Watch Impress、日经 xTECH 系、中文若干博客）报道的那个。** 媒体页面的 GitHub 链接现在全部 404：
- MakeUseOf 文章外链 → `github.com/krisgfx9/Wireless-Device-Battery-Monitor`（实测页面内该链接）
- PC Watch（`pc.watch.impress.co.jp/docs/news/2158043.html`）外链同样指向该地址，文中描述"约 350KB、内存约 2MB、无需管理员权限"——与 A 版 README 完全吻合，与 B 版（107MB Electron）明显不符。

### 1.2 时间线（可核验）

- **2026-07-04**：A 版 1.0.2 首发（C++）；同日 B 版仓库建立（`created_at` = 2026-07-04T15:39:48Z）。
- **2026-11 上旬**：媒体开始报道 A 版，中文站称"4.9K star、350KB、纯 C++ Win32"。
- **2026-11-15**：B 版被迁入新建组织 `battery-hub`（组织 `created_at` = 2026-11-15T19:01:43Z），同时上线 `battery-hub.github.io` 官网、新增中文/西语/希伯来语。
- **2026-11-19 13:30 UTC**：Wayback 快照显示仓库一切正常：4.4k★、374 forks、**2.8k open issues**、GPL-3.0、577 commits。
- **2026-11-20**：PC Watch 仍在正常报道，验证可用。
- **2026-11-21 22:49**：gitcode 镜像仍有同步提交（`Merge PR #2784`），说明当天上游还存在。
- **2026-11-22**：第三方趋势记录显示 5,788★、**3,554 条 open issues** 后失去更新。
- **2026-11-23**：仓库与账号均 404，且**无 301 重定向**（重定向缺失＝不是改名，是账号/仓库被删或封禁）。

### 1.3 它为什么没了？三种解释的证据权重

| 解释 | 支持证据 | 反证/缺口 | 强度 |
|---|---|---|---|
| **被 issue 淹没后弃号**（最可能） | open issues 从 2.8k（11-19）涨到 3.5k（11-22），几乎全是"求支持某型号"；中文报道亦称"前十来条全是求支持新设备，没有一条是报错" | 没有作者告别留言（账号已删，无处可留） | 中 |
| 改名迁移 | 有第三方趋势站点把仓库名记作 `Battery-Monitor` | 两个名字均 404；真实改名会留 301 重定向；作者自建站 `krisgfx.com/tools/` 仍在线且仍写 "Wireless Battery" | 弱 |
| 被下架/封号（DMCA、滥用） | 4.9k→5.8k star 短期暴涨 + 2800+ PR，存在被判定异常增长的可能 | **未检索到任何 takedown 公告、DMCA 记录或社区投诉** | 弱 |

> 对使用者的含义：无论哪种原因，**这个软件目前既没有安全更新，也没有可信的官方下载**。这一个事实就足以把"是否安装"从"好不好用"降级成"要不要冒供应链风险"。

> 观察：A 版作者邮箱为 `bimbikri@students.zhaw.ch`（瑞士 ZHAW 学生，真实机构域名）；B 版的全部提交作者被刻意匿名为 `Battery Hub <battery-hub@noreply.invalid>`（`.invalid` 是保留域，永远不可达）。B 版不宣称与 A 版的关系，但在命名、功能面（"Asleep or off"、Compx/AULA/MCHOSE 等冷门型号）、组织迁移时机上与媒体报道高度重合。**是否存在"蹭同名"无法证实（中等强度怀疑），但对用户的现实影响是确定的：搜"BatteryHub"会同时命中两者，且只有 B 还给下载。**

### 1.4 30 秒分辨你装的是哪一个

| 看什么 | A（C++ 原版） | B（Electron 版） |
|---|---|---|
| 安装包大小 | 约 **1–3 MB**（`BatteryHub-Setup-1.x.x.exe`，Inno Setup + lzma2） | 约 **107 MB**（`Battery-Hub-Installer-2.0.4.exe`） |
| 进程 | 单个 `BatteryHub.exe`（约 2MB 内存） | 多个 `Battery Hub.exe` + Electron 子进程（数百 MB） |
| 安装目录 | `%LOCALAPPDATA%\Programs\BatteryHub`，内含 `headsetcontrol.exe` | 同路径下但含 `resources\app\src\main\main.js`、`chrome_100_percent.pak`、`ffmpeg.dll` 等 Electron 特征文件 |
| 任务栏表现 | 通知区旁直接出现 `🎧 30%` 文本 | 出现一个或多个带电量数字的**托盘图标** |
| 许可证文件 | GPL-3.0（`LICENSE.txt` 3 万 5 千字节） | MIT（约 1KB） |

---

## 2. 分析方法与边界

本次采用开源软件评估常用的七条线，每条都落到可复现证据：

| 方法 | 本次做法 | 产出 |
|---|---|---|
| ① 溯源与消歧 | GitHub API + git clone + 镜像比对 + 媒体外链反查 | 三个同名项目、A 版已 404 |
| ② 代码静态审计 | 全量 clone，按 sink 分类 grep（网络/子进程/文件/注册表/HID 写入/eval）| 端点清单、IPC 面、写入行为 |
| ③ 依赖与漏洞（SBOM） | `package-lock.json` 全量解析 + OSV.dev 逐包查询 | 43 个生产依赖，0 已知漏洞 |
| ④ 构建与发布可信度 | 审 `.github/workflows`、签名脚本、更新校验代码 | A 有 ECDSA 验签；B 无 |
| ⑤ 权限与攻击面 | 清单文件、安装器脚本、进程模型 | 均为 asInvoker / 免管理员 |
| ⑥ 隐私与数据 | 网络端点、日志内容、上报字段、脱敏逻辑 | 均无遥测；A 有作者自建上报端点（用户主动） |
| ⑦ 社区与口碑 | star/issue/PR/下载量、媒体与社区发言 | 见第 5 节 |

**证据强度标注约定：** 强＝本次一手接口/本地源码实测；中＝单一可靠来源或归档快照；弱＝聚合站转述、搜索快照、无法二次核验。

**未做的（诚实声明）：** 没有在隔离沙箱里动态运行安装包（当前网络下行仅 ~40KB/s，107MB 安装包下载未完成），没有跑 VirusTotal 多引擎（无 API Key），因此"无恶意行为"是基于**源码审计**而非沙箱观测；A 版因仓库 404，只能审镜像代码，**无法保证镜像与上游逐字节一致**。

---

## 3. 安全性分析

### 3.1 权限与攻击面：两者都干净

| 检查项 | A（C++） | B（Electron） |
|---|---|---|
| 管理员权限 | 不需要，`app.manifest` 为 `asInvoker`；Inno Setup `PrivilegesRequired=lowest`（可选"所有用户"安装才提权） | 不需要，electron-builder `perMachine: false` |
| 内核驱动 / 服务 | 无 | 无 |
| 自启动方式 | HKCU\...\Run 值（仅当用户勾选） | `app.setLoginItemSettings`（仅当用户开） |
| 渲染进程隔离 | N/A（原生 Win32） | `contextIsolation: true`、`nodeIntegration: false`、preload 白名单 IPC、渲染层 CSP `default-src 'self'; script-src 'self'`、`will-navigate` 全拦、`setWindowOpenHandler` deny |
| 特权代码隔离 | N/A | `node-hid` 跑在独立 utility process（驱动崩溃不至于拖垮主进程） |
| 外部命令 | 无 shell 调用 | 会起 PowerShell（`-ExecutionPolicy Bypass`）与 Python 子进程，见 3.4 |

> 结论：**"需要管理员权限"这条常见顾虑可以排除。** 两者都是"普通用户态 + 直接跟 HID 设备说话"。

### 3.2 网络行为：端点清单（逐行 grep 结果）

| 项目 | 端点 | 触发条件 | 内容 |
|---|---|---|---|
| A | `api.github.com`（更新检查/下载，且下载后强校验签名） | 自动检查（可关） | 版本号 |
| A | `github.com`, `release-assets.githubusercontent.com`, `objects.githubusercontent.com` | 用户点"下载更新" | 安装包（**白名单主机 + 允许重定向**） |
| A | `krisgfx.com/tools/api/requests`（作者自建服务器，WinHTTP POST） | **仅在用户填"请求支持某设备"表单并点发送时** | 用户自填的型号/备注（无设备指纹、无机器标识） |
| B | `api.github.com/repos/battery-hub/battery-hub/releases/latest` | 启动 60s 后 + 每 6h | 版本号 |
| B | `api.github.com/search/issues` | 用户点"发送设备报告"时 | 搜是否已有同类 issue |
| B | `github.com/.../issues/new?...` | 用户点发送后**打开浏览器**（不在应用内 POST） | 预填表单，用户自己按提交 |

**没有任何分析/统计/崩溃上报端点。** A 版唯一非 GitHub 的请求是"设备支持请求"，是用户主动的、内容自填的；B 版干脆把上报降级成"打开浏览器预填 GitHub 表单"。
另外 B 版的设备报告有明确的隐私处理：键盘类设备的 input 帧**整个不采集**（可能包含按键），feature report 也略去（可能含蓝牙地址），见 `src/main/diagnostics.js` 注释与 `sanitize()`。

### 3.3 更新与供应链：这是两者最大的安全差异

| 维度 | A（C++） | B（Electron） |
|---|---|---|
| 安装包代码签名 | ❌ 无（README 明示会弹 SmartScreen） | ❌ 无（README/SECURITY.md 明示） |
| 更新包完整性 | ✅ **ECDSA P-256 签名 `SHA256SUMS.txt.sig`，公钥硬编码**；`VerifyReleaseSignature()` 失败即中止，且"未签名的版本只提示有新版本，不提供安装按钮" | ⚠️ 仅 `electron-updater` 自带的 `latest.yml` 里 sha512（与安装包同源下发，**不构成签名**）；`publish` 未配 `publisherName`，不做 Authenticode 校验 |
| 更新安装方式 | 用户确认后由 `update-helper` 应用，安装前再验签 | `autoDownload = true` + `autoInstallOnAppQuit = true` → **下次退出自动安装**，用户可能毫不知情 |
| 构建来源 | GitHub Actions，tag 触发；Actions **全部 pin 到 commit SHA**；默认 `contents: read`，发布作业单独提权 | GitHub Actions，tag 触发；Actions 用 **tag 引用**（`@v4`）；无 provenance/attestation |
| 第三方二进制 | 捆绑 `headsetcontrol.exe`（GPLv2 上游 4.3.0），CI 下载官方 tar.gz 并**校验 SHA256**（在 build.yml 内） | 无捆绑二进制；但运行时依赖系统 PowerShell 与 `python`（见 3.4） |
| 可复现性 | 可（`make`，GPL 对应源码随包） | 部分（`npm ci` + electron-builder，未提供可复现构建说明） |
| 依赖面 | 无第三方运行时库（Win32 API） | 43 个生产依赖（含 electron 本体）；全量 309 包 |
| 已知 CVE | 无第三方依赖可查 | OSV 查询 `electron@45.2.1 / electron-updater@6.8.11 / node-hid@3.1.2 / electron-store@8.2.0` → **0 条** |

**风险解读：**
- 对 B 版而言，**信任根 = GitHub 仓库 + CI 的 `GITHUB_TOKEN`**。作者账号被钓鱼、CI 被注入 PR、或组织被接管，都能让全量用户静默装马。而 B 版作者是匿名身份（`noreply.invalid`）、28 star 的新项目——**这类项目的自动化更新通道值得谨慎**。
- A 版的签名方案把"签名私钥"留在维护者本机（README 明说），但副作用是**bus factor = 1**：人不在，就再也发不出安全更新。

### 3.4 代码级安全隐患（B 版专属，按严重度排序）

1. **PowerShell `-ExecutionPolicy Bypass` 调用**（`system-batteries.js`、`drivers/audeze.js`、`drivers/compx.js`、`main.js` 圆角脚本）。脚本本身是静态随包分发的，未发现拼接用户输入的注入点；但 `Bypass` + `powershell.exe` 是杀软启发式的高危特征，误报/拦截概率显著升高。
2. **Python 依赖注入**（`monitor.js`）：MCHOSE 耳机走 `spawn('python', [headset_reader.py])`，按 `python → python3 → py` 顺序在 **PATH 上找**，并需要用户自己 `pip install hidapi`。这意味着：① 用户被要求装一个未固定的 PyPI 包（供应链扩张）；② PATH 顺序劫持（本地已被控制的机器）可以借道执行；③ Windows 应用商店的 python 别名会让它首次失败（作者已在 1.3.0 修过这个坑）。
3. **`asar: false`**：应用源码以明文目录形式安装在 `resources/app`，任何本地进程都可改；配合"退出即自动更新"，篡改不会被完整性检查发现（B 版有 `package-check.js` 文件清单自查，但清单本身在同目录，可一并改）。
4. **工程稳定性历史**：CHANGELOG 自述修过内存泄漏（每次刷新注册一次 document 点击监听）、HID 句柄竞态导致的堆损坏崩溃、更新按钮状态机错乱等。这些是 Electron + 原生 HID 的典型坑，说明**频繁小版本更新带来的是回归风险**，而自动更新会把回归直接推给所有人。

### 3.5 设备写入行为（两者都有，风险可控但不为零）

两者都会向设备发 HID feature/output report——**这是读取厂商私有电量字段的必要手段**，不是"写固件"。实测统计：A 版 15 处 `HidD_SetFeature/WriteFile` 调用，B 版 13 处 `dev.write / sendFeatureReport`，均为"查询帧"。
- 理论风险：厂商协议逆向不完全时，某些字节组合可能触发设备的隐藏命令（改 DPI、切模式、进 DFU）。**没有任何证据表明发生过**，且两版都只对"用户显式添加的设备"发帧。
- 需要用户自己承担的部分：A 版部分机型（Keychron 2.4G、NuPhy）需要**自己刷第三方固件**才能读电量——这是用户侧的真实风险点，与软件本身无关但常被忽略。

### 3.6 安全评分（0–5，5 最好）

| 维度 | A 原版 | B Electron | 说明 |
|---|---|---|---|
| 权限最小化 | 5 | 5 | 均免管理员、无驱动无服务 |
| 网络与隐私 | 5 | 5 | 无遥测；端点最少 |
| 更新完整性 | 5 | **2** | A 有 ECDSA 验签；B 无签名 + 静默安装 |
| 依赖/供应链 | 4 | 3 | A 无依赖但有捆绑二进制；B 无捆绑但 309 包 + PATH 上的 Python |
| 代码安全实践 | 4 | 4 | B 的 Electron 加固到位；A 的 CI 更硬（SHA pin、只读权限） |
| 发布可核验性 | **1** | 3 | A 官方渠道 404，只能靠镜像；B 有 SHA256SUMS + winget 哈希 |
| **均分** | **4.0** | **3.7** | 只看这六项，两者都是"良好实践"；差距主要在"更新完整性"和"发布可核验性" |

---

## 4. 可用性分析

| 维度 | A（C++，350KB） | B（Electron，107MB） |
|---|---|---|
| 安装体验 | Inno Setup，per-user 默认，可选所有用户；有简体中文安装界面 | NSIS，per-user，一键；第一次必弹 SmartScreen |
| 资源占用 | 约 350KB 磁盘、约 2MB 内存、0 常驻服务 | 107MB 安装包、约 290MB 常驻内存（作者自述从 500MB 优化而来）、关窗 30s 后释放窗口 |
| 任务栏呈现 | **真正画在任务栏**（`🎧 30%  🖱 93%`），低电变红、充电 ⚡、睡眠变暗、可选显示离线 × | **系统托盘图标**（每设备一个，图标里画数字），另有托盘弹窗汇总 |
| 设备覆盖 | **113 条品牌/设备**（README 表逐行统计），含 81+ Razer、Logitech HID++、SteelSeries、Corsair、HyperX、Audeze、MCHOSE、VXE/ATK、Compx 键盘、蓝牙通用、Xbox/PS 手柄，甚至 Phone Link 手机电量 | 12 个品牌族（Razer 81 款 + 其他，合计约 150–200 型号），**86 条"求支持"issue 排队中** |
| 校准 | 有"30 秒扫描 + 手动换算"引导 | 有同款引导，且加"报告前先查重"门禁 |
| 语言 | **20 种**（含 zh-CN/zh-TW/ja/ko/ru/ar/he…） | 6 种（en/es/es-419/zh-CN/zh-TW/he） |
| 历史与曲线 | 有（可选录制，本地文件） | 有（24h/7d 曲线、drain rate、time-left 估算） |
| 已知限制 | 任务栏拥挤时会顶到小组件；部分机型需刷固件；无 HID++ 的 C542 接收器读不到 | 首次安装体积劝退；PowerShell 策略/杀软拦截；MCHOSE 需 Python；90+ 设备请求未实现 |
| 更新 | 手动确认 + 验签 | 自动下载 + 退出时静默安装 |

**典型用户推荐：**
- 老机器 / 极简主义 / 只想看数字 → **A**（前提是拿到可信副本）
- 喜欢图表、多设备历史、不介意体积 → **B**（记得关自动安装更新）
- 企业/受管终端 → **都不要**：未签名二进制 + 自动更新通道，很难过软件白名单与合规审计；用 Intune/组策略或自编译部署。

---

## 5. 口碑与生态

### 5.1 媒体：一边倒的正面，但深度普遍不足

| 媒体 | 标题/角度 | 可核验的信息 |
|---|---|---|
| MakeUseOf | "This free Windows app shows all your wireless battery levels on the Taskbar" | 链接指向 A 版（现 404） |
| PC Watch（Impress，日本） | 「マウスやキーボードの電池残量がタスクバーで確認できる『BatteryHub』」 | 明确写：约 350KB、约 2MB 内存、无需管理员、有实验性 Linux 构建 |
| iGuides（俄） | 同题材 | 提到免费、任务栏常驻 |
| 中文博客（nihendiao / vgover 等） | "350KB 的小工具把电量直接焊在任务栏上" | 称 4.9K star、"MIT 协议"——**许可证写错了，实际是 GPL-3.0** |

**共性**：都停留在"功能 + 轻量"层面，没有一家做过杀软扫描、长时间稳定性测试或供应链审查。这类"通稿式评测"是同类小工具爆红的典型路径，也是用户误判风险的主要来源。

### 5.2 社区：真实的用户声音比想象中少，而且被"求支持"淹没

- **Hacker News**：整站只有 1 个 story、1 条评论——"That's nice for Windows. I already have this for macOS… Cool!"。没有技术讨论，没有安全质疑。
- **Reddit / YouTube / V2EX / 吾爱 / 远景 / Chiphell**：本次检索**未发现成规模的讨论串**（也可能与地区网络、检索覆盖面有关，标注为"未取证"而非"不存在"）。
- **GitHub 侧才是真实口碑**：open issues 从 2.8k（11-19）涨到 3.5k（11-22），**几乎全是"求支持某型号"**，不是 bug 报告。这说明两件事：① 用户认可度确实高（愿意排队等支持）；② 维护者被社区需求压垮，这本身就是项目可持续性的红灯。
- **差评/隐患集中在四点**：设备支持不全（README 自认大量型号 "hardware-untested"）；某些设备插 USB 供电时百分比跳 100%（README 自述"不是可靠充电指示"）；安装包未签名导致 SmartScreen 警告；文档在多显示器/任务栏拥挤场景下描述不一致。
- **中文用户痕迹**：11-18 出现了专门的汉化 fork（`fuyue521/Wireless-Device-Battery-Monitor-CN`，9★），反向证明中文圈确实有人用。

### 5.3 传播量级

- 媒体口径："四天五个版本，累计下载接近八万次"（**弱证据**，只有媒体转述，无 Releases 计数支撑——A 版 Releases 已随仓库消失，无法复核）。
- 替代指标（**证据强度：中**，来自 Wayback 归档与第三方趋势站的二次检索，本次网络环境无法直连 archive.org 复核）：374 forks、577 commits、2.8k open issues、峰值 star 约 4.4k（11-19 归档）→ 约 5.8k（11-22）。媒体"4.9K star"的口径处于这两个数之间，量级可信。
- **A 版没有对应的 winget 清单**（本次检索 winget-pkgs 只找到 DORON177.BatteryHub，且 A 版只能从 GitHub Releases 下 exe）；进 winget 的是 B 版：`DORON177.BatteryHub`，PR #407677 已于 **2026-08-08 合并**，但清单只到 2.0.1，**落后当前 2.0.4 三个版本**。

### 5.4 生态：fork 森林 = 信任碎片化

| 分支 | 形态 | Star | 风险含义 |
|---|---|---|---|
| `fuyue521/…-CN` | 汉化 | 9 | "汉化版"是恶意打包的经典载体，需自行编译 |
| `joaoarebalo/…-Fallen` | 改版 | 10 | 同上 |
| `emincodesM/…-TURKCE` | 土耳其语 | 3 | 同上 |
| `ferloom/Wireless-Device-Battery-Monitor` | Rust 重写 | 32 | 代码已换语言，无法与原文对照 |
| `Hassani0x/BatteryHub-Community` | 社区续命版 | 0 | 明示仍为 unsigned |
| `ZGMFX01A/…`（Rust） | **独立项目，非 fork**（建仓 2026-02-23，早于 A 版走红） | 47 | 名字相同、实现不同，用户极易混淆 |

**结论**：官方一消失，社区立刻分成 6+ 个互不兼容的信任根，而用户没有任何办法从文件名分辨谁是谁。这是比"代码有漏洞"更现实的威胁。

### 5.5 竞品口碑（同类工具横向）

| 工具 | 技术/许可 | 口碑要点 | 适合谁 |
|---|---|---|---|
| **BatteryHub（A）** | C++/GPL-3.0 | 轻、覆盖广、但官方渠道消失 | 愿意自己编译的极客 |
| **Battery Hub（B）** | Electron/MIT | UI 现代、有 winget、但体积大、无名作者 | 想省事、能接受体积 |
| **BlueGauge** | Rust/MIT，462★ | 中文界面、别名、低电提醒；**只支持标准蓝牙**，2.4G 私有协议读不到 | 纯蓝牙键鼠/耳机 |
| **SplusXBTMeter** | C#/GPL-2.0，59★ | 面向蓝牙耳机/音箱，2026-05 后更新放缓 | 老用户沿用 |
| **BatteryMode** | 288★ | 只替换本机电池指示，**不读外设**（场景不同） | 笔记本本机电量 |
| **Win11 "电池状态"小组件** | 微软原生 | 汇总本机 + 蓝牙外设，测试中；不解决 2.4G 私有协议 | 90% 的普通用户 |
| **G HUB / Synapse** | 厂商原生 | 精确，但常驻数百 MB，"只为看个电量"不划算 | 已装厂商全家桶的人 |

### 5.6 安全事件：没有找到，但别把"没找到"当"没问题"

- 本次检索**未发现**任何 VirusTotal 讨论串、杀软误报记录、恶意样本报告或 CVE 关联。唯一相关表述来自聚合站"无已知 CVE"（弱证据）。
- 真实风险不在"已知恶意"，而在**信任链断裂**：官方 404 后，流传的每一个 exe 都无法追溯到发布者。

---

## 6. 综合评分

| 维度 | 权重 | A 原版 | B Electron |
|---|---|---|---|
| 安全性 | 35% | 4.0 | 3.7 |
| 可用性 | 25% | 4.5 | 3.5 |
| 可获取/可维护 | 20% | **1.5**（404） | 4.0 |
| 口碑 | 10% | 4.0 | 2.5 |
| 透明度 | 10% | 4.0 | 3.0 |
| **加权总分** | | **3.6** | **3.5** |

**为什么两者分数几乎打平，但给你的建议是"能用 A 就用 A"？**
1. **代码质量上两者接近**：A 版胜在"轻 + 更新验签 + 设备覆盖广 + 发布鉴权更硬"；B 版胜在"还活着 + 有 winget + 190 个自动化测试全绿 + Electron 安全加固到位"。
2. **A 版的低分几乎全部来自"现在拿不到官方副本"**（可获取性 1.5）。这个短板是可以被用户自己补上的——镜像源码在、能自编译、更新验签逻辑可读。
3. **B 版的短板补不上**：匿名作者 + 无签名更新 + 退出时自动安装。你可以通过 winget 安装、关自动更新把风险压到可接受，但**信任根仍然是那一个账号**。
4. 如果 A 版仓库恢复（或作者迁到新址并公布签名公钥），它的分数会立刻回到 4.3+，是这一类工具里综合最优的解。

---

## 7. 给不同人的操作建议

### 7.1 只想看电量（推荐顺序）
1. **Windows 11 原生**：设置 → 蓝牙和设备 → 设备，或"电池状态"小组件；零安装、零风险。
2. **厂商自带**：Logitech G HUB / Razer Synapse 的电量小组件（代价是重）。
3. **本工具**，按 7.2 的方式装。

### 7.2 如果要用 A 版（推荐路线）
```powershell
# 1) 从镜像取源码（当前唯一可核验的上游）
git clone https://gitcode.com/gh_mirrors/wi/Wireless-Device-Battery-Monitor.git
# 2) 自己编译（需要 MSYS2/MinGW，约 5 分钟）
cd Wireless-Device-Battery-Monitor && make
# 3) 只运行你编译出来的 BatteryHub.exe
```
- 若坚持用现成安装包：先下 `SHA256SUMS.txt` 与 `SHA256SUMS.txt.sig`，确认发布者签名后再装；**没有 .sig 的版本，连作者自己的程序都拒绝安装，你也不该装**。
- 接受 SmartScreen 提示是这段体验的一部分；不要"关掉 Defender 再运行"。

### 7.3 如果要用 B 版
```powershell
winget install DORON177.BatteryHub   # winget 校验清单哈希，比官网/软件站靠谱
# 装完在 设置 → 更新 里关掉自动安装；只在需要时手动更新
```
- 拒绝任何"绿色版/汉化版/破解版"；B 版已内置 zh-CN。
- 出现 PowerShell 弹窗或要求 `pip install` 时，先确认是哪台设备触发的（只影响 MCHOSE 耳机路径）。

### 7.4 通用卫生
- 出站防火墙只放行 `api.github.com`、`github.com`、`objects.githubusercontent.com`、`release-assets.githubusercontent.com`，能挡掉绝大部分"更新通道投毒"。
- 每季度看一眼 `%APPDATA%\battery-hub\battery-hub.log` 与安装目录文件清单有没有多出来的东西。
- 关注仓库是否恢复/迁移：A 版作者站点 `krisgfx.com/tools/` 目前仍在线（标题仍是 "Wireless Battery"），可作为上游恢复的信号。

---

## 8. 附录：证据与复现

```text
# 仓库状态
GET https://api.github.com/repos/krisgfx9/Wireless-Device-Battery-Monitor   -> 404 Not Found
GET https://api.github.com/users/krisgfx9                                   -> 404 Not Found
GET https://api.github.com/repos/battery-hub/battery-hub                    -> 200, 28 stars, 91 open issues, v2.0.4

# 依赖漏洞（OSV）
POST https://api.osv.dev/v1/query  {package:{name:"electron",ecosystem:"npm"},version:"45.2.1"} -> 0 vulns

# 本地验证
npm ci && npm test            -> 190 tests pass
grep -rn "https\?://" src/    -> 端点清单见 3.2
```

**关键源码位置（B 版）：** `src/main/main.js:383-457`（更新器）、`src/main/preload.js`（IPC 白名单）、`src/main/diagnostics.js`（报告脱敏）、`src/main/system-batteries.js`（PowerShell/python 调用）、`src/main/monitor.js`（Python 子进程）。

**关键源码位置（A 版）：** `src/update_win.cpp:437-450`（验签后才下载）、`src/update_signature.cpp`（BCrypt P-256）、`src/update_public_key.h`（内置公钥）、`res/app.manifest`（asInvoker）、`installer/BatteryHub.iss`（per-user 安装）。

---

## 附录 B：消失之谜与 fork 谱系（2026-11-23 追查）

### B.1 精确时间线

| 时间（北京时间） | 事件 | 证据 |
|---|---|---|
| 11-16 ~ 11-21 | 6 天内 **899 次提交、203 位作者、合并 PR 编号最高到 #4108** | gitcode 镜像全量 git 历史（`git fetch --unshallow`） |
| 11-21 22:10 | 第三方日报快照：5,788★、**3,554 open issues**（首次下降 11 条）、pushed_at=11-21 | 开源新生代雷达第 18 期 |
| 11-22 05:49 | 最后一条维护者提交 `Merge PR #2784`（作者本地时间 11-21 22:49 CET） | git log |
| 11-22 05:10~10:00 | 仍有贡献者提交（`123mmcr-blip`） | git log |
| 11-22 16:20 UTC | 社区出现 Rust 重写 `ferloom/Wireless-Device-Battery-Monitor`（"BatteryHub - Rewriten in Rust"） | repo created_at |
| 11-22 19:51 UTC | `symNistaken/TaskbarVitals` 建仓，自述"**a spiritual successor to BatteryHub**"、"builds on the last public source" | repo created_at + README |
| 11-23 11:32 UTC | 仓库与账号 404（本次实测） | GitHub API |
| 11-23 全天 | 陆续出现 10+ 个 fork/改版（含今日新建的 `artnway/WirelessBatteryRings`、`Pilgrim415`） | 搜索 API |

结论：**删除发生在 11-22 白天（UTC），从最后一次提交到整号消失不到 24 小时。**

### B.2 三种解释与证据权重

| 解释 | 支持证据 | 反证 | 权重 |
|---|---|---|---|
| **维护者主动删号**（流量与 issue 洪泛压垮单人） | 6 天 899 提交 / 203 作者 / 3,554 条待处理 issue / 合并到 PR #4108；核心开发者只有 bimbikri 一人；最后几天是深夜批量 merge | 没有留下告别公告（号没了也无处留） | **中高** |
| 被 GitHub 封禁/处置 | 两周内从 0 涨到 ~5.8k star、issue 爆炸、大量疑似模板化 PR，容易触发异常检测 | **无任何封禁/申诉/社区传闻**；个人站点仍在 | 低 |
| 改名或转私有 | 可以解释"旧链接 404" | 改名会留 301 重定向（实测没有）；转私有时用户主页仍应存在（实测 404） | **可排除** |
| DMCA / 侵权下架 | — | 未检索到任何下架公告、仓库也未显示 DMCA 页 | **可排除** |

补充观察：这个仓库 **watch 只有 6 人、fork 439**，但 star 5.8k——典型的"围观型热度"（被推荐流推到 GitHub Trending 第一），真实使用者比例远低于 star 数，却产生了 3,554 条设备请求。**流量结构本身就在制造维护灾难。**

### B.3 fork 谱系（父仓库删除后，GitHub 已将它们转为独立仓库：`fork=false, parent=null`）

**A 类：带完整/较完整历史、可直接用的延续版**

| 仓库 | 形态 | 活跃度 | 安装包 |
|---|---|---|---|
| [Hassani0x/BatteryHub-Community](https://github.com/Hassani0x/BatteryHub-Community) | 社区续命版（C++，668 commits） | 11-21 | ✅ v1.1.1（3.1MB）＋ portable zip ＋ **已换用自己的更新签名公钥** |
| [symNistaken/TaskbarVitals](https://github.com/symNistaken/TaskbarVitals) | "精神续作"，含 CPU/内存/GPU 读数 | 11-22 建仓 | 暂无 release |
| [yunluoxincheng/Wireless-Battery-Center](https://github.com/yunluoxincheng/Wireless-Battery-Center) | 中文增强（设备管理面板，439 commits） | 11-21 | ✅ v0.1.0（18.9MB setup，含 headsetcontrol 源码包） |
| [fuyue521/…-CN](https://github.com/fuyue521/Wireless-Device-Battery-Monitor-CN) | 汉化版（388 commits） | 11-18 | ✅ v1.0.0（1.0MB） |
| [poipoi1320/…_Add_VT9PRO](https://github.com/poipoi1320/Wireless-Device-Battery-Monitor_Add_VT9PRO) | 加 VT9 Pro 支持（383 commits） | 11-18 | — |
| [joaoarebalo/…-Fallen](https://github.com/joaoarebalo/Wireless-Device-Battery-Monitor-Fallen) | 加 Fallen 鼠标支持（10★，快照） | 11-17 | — |
| [Pilgrim415/…](https://github.com/Pilgrim415/Wireless-Device-Battery-Monitor) | 中文定制（托盘渲染、51 品牌配色） | 11-23 | — |

**B 类：换语言重写（代码已不可与原文对照）**

| 仓库 | 语言 | Star |
|---|---|---|
| [ferloom/Wireless-Device-Battery-Monitor](https://github.com/ferloom/Wireless-Device-Battery-Monitor) | Rust（"BatteryHub - Rewriten in Rust"） | 34★，11-22 起连发 3 个 release |
| [karanshukla/BatteryHubRust](https://github.com/karanshukla/BatteryHubRust) | Rust（719 commits） | 0★ |
| [ZGMFX01A/Wireless-Device-Battery-Monitor](https://github.com/ZGMFX01A/Wireless-Device-Battery-Monitor) | Rust，**2 月就建仓的独立项目**，与本次事件无关 | 49★ |

**C 类：浅历史快照 / 空壳**

- `raeAllen17`、`StarlessNight13`：145–146 commits（浅克隆后重建，历史不可信）
- `emincodesM`（土耳其语，4 commits）、`superxyline`（10-24 的旧快照）、`artnway/WirelessBatteryRings`（基于 v1.0.4，30 commits）
- `wcewong/batmon`：**macOS 移植**（14KB，1 commit，11-22 建仓）
- `qritenan/…`：**空仓库**（0KB，11-20 建仓，没来得及 fork）

### B.4 用 fork 时最容易踩的坑：更新签名

原版要求"更新包必须由作者的 ECDSA 私钥签名，否则拒绝安装"。父仓库一没，这套信任链就断了：

- `Hassani0x` 版**换成了自己的公钥**（已核对，原公钥字符串不在其中）→ 它能给自己的 release 签名，**安全模型和原版一致**，但信任根变成一个新的匿名维护者。
- `fuyue521` 等 fork **删掉了 `update_win.cpp`** → 要么没有自动更新，要么改成了不验签的下载（更危险）。
- 任何**仍保留原公钥**的 fork，都不可能给自己的更新签名 → 用户会看到"有新版本但装不上"，只能手动换包。

**所以：从 fork 安装 = 换一个信任根。请优先选"有 release + SHA256SUMS.txt + 附带 headsetcontrol 源码包（GPL 合规）+ 历史提交可追溯到 bimbikri"的那几个。**

---

## 附录 C：fork 选型对比与最终推荐（2026-11-23 实测）

对 5 个候选做了源码级对比（全部 clone 到本地，逐个 grep 网络/子进程/注册表/签名逻辑，并与上游最后快照 cpp-monitor 做文件级 diff）。

### C.1 决策矩阵

| 候选 | 自有更新仓库 | 更新验签 | 与上游差异 | 驱动数 | 最近提交 | 现成安装包 |
|---|---|---|---|---|---|---|
| **symNistaken/TaskbarVitals** | ✅ `symNistaken/TaskbarVitals` | ✅ **自有新公钥** `RUNTMSAAAABc071…` | +21 文件（sysmon/gpumon/panel，约 3.5k 行） | 119 | **11-23 01:52** | ❌ 无 release（需自编译） |
| Hassani0x/BatteryHub-Community | ❌ 仍指已死的 `krisgfx9/…` | ⚠️ 原公钥，但 tag 构建走 **dev 通道 → 更新器整体关闭** | **+7 文件**（改动最小） | 113 | 11-20 23:22 | ✅ v1.1.1（3.1MB） |
| yunluoxincheng/Wireless-Battery-Center | ❌ | ❌ 已删除验签代码 | +70/−180（引入 **Qt 6.8** 面板） | 78 | 11-21 12:26 | ✅ v0.1.0（18.9MB） |
| fuyue521/…-CN | ❌ | ❌ 无 | +5/−179（**基线过旧**） | 78 | 11-18 23:03 | ✅ v1.0.0（1.0MB） |
| ferloom/…（Rust） | ✅ | ❌ 无签名，仅同源 SHA256 | 全部重写 | **13** | 11-22 23:38 | ✅ v1.4.1（1.3MB） |
| （参照）上游镜像自编译 | — | ✅ 原公钥（仓库已死，等于不更新） | 0 | **136** | 11-21 22:49 | 需自编译 |

### C.2 结论：选 `symNistaken/TaskbarVitals`，自己编译一次

理由（按权重）：

1. **它是唯一"信任链完整"的延续版**：更新仓库指向自己、签名公钥是自己的新钥、CI 里 `actions` 全部 pin 到 commit SHA、默认 `contents: read`。其他 fork 要么更新地址还指着已死的 `krisgfx9`，要么把验签整段删了。
2. **新增代码干净**：我逐个检查了 `sysmon.cpp` / `gpumon.cpp` / `panel.cpp` —— 只用 PDH 性能计数器和 DXGI，**没有网络、没有 PowerShell、没有注册表写入、没有提权**（`PrivilegesRequired=lowest`）。带 CPU/内存/GPU 读数，属于"多给你一点，不多要权限"。
3. **设备覆盖没缩水**：119 个驱动（上游最后快照 136，差的 17 个是上游最后两天才合并的 PR，另有 3 个上游驱动它选择重写）。
4. **唯一还在动的项目**：最后提交 11-23 01:52（今天），有完整的 `build.yml`（PR 也跑构建 + 协议测试）。
5. **打包流程同样守规矩**：`installer/build-installer.ps1` 会下载固定版本 HeadsetControl 4.3.0 并**校验 SHA-256，不匹配就拒绝打包**；发布用的 `SHA256SUMS.txt` 要求维护者本机签名后才允许更新安装。

代价：**没有现成 release，需要自己编译一次**（约 10 分钟）——而这对你恰好是优点：绕开了"下载陌生人的 unsigned exe"这一步。

### C.3 操作步骤

```powershell
# 1) 装构建环境（MSYS2 + MinGW-w64 gcc/make）
winget install --id MSYS2.MSYS2 -e
# 打开 "MSYS2 MINGW64" 终端执行：
#   pacman -S --needed mingw-w64-x86_64-gcc make

# 2) 取源码
git clone https://github.com/symNistaken/TaskbarVitals.git D:\Tools\TaskbarVitals
cd D:\Tools\TaskbarVitals

# 3) 编译（默认 dev 通道：不联网检查更新，最保守）
make
.\TaskbarVitals.exe
```

需要耳机（HeadsetControl）支持时，再跑一次打包脚本拿完整安装包（会自动下载并校验 HeadsetControl）：

```powershell
# 需要先装 Inno Setup 6/7：winget install --id JRSoftware.InnoSetup -e
powershell -ExecutionPolicy Bypass -File installer\build-installer.ps1 -Version 1.0.0
# 产物：dist\TaskbarVitals-Setup-1.0.0.exe、dist\TaskbarVitals-1.0.0-portable.zip、SHA256SUMS.txt
```

### C.4 装完后的 5 条自检

1. 任务栏通知区旁出现 `🎧 30%  🖱 93%` 文本（不是托盘图标）→ 对。
2. 安装/启动**没有 UAC 弹窗** → 对。
3. 任务管理器里只有一个 `TaskbarVitals.exe`，内存 2–4 MB → 对。
4. 资源监视器里它的网络连接应为**零**（dev 通道不查更新）→ 对。
5. 确认公钥是它自己的：`src\update_public_key.h` 里应是 `RUNTMSAAAABc071…`，而不是上游的 `RUNTMSAAAABHVmMa…` → 对。

### C.5 备选路线（按你的偏好）

| 你的偏好 | 选它 | 代价 |
|---|---|---|
| 不想编译，能用就行 | Hassani0x v1.1.1 安装包 | 改动最小（+7 文件）、有 SHA256SUMS；但**永远不会有自动更新**，17 个上游后期驱动缺失 |
| 要求与"原版"逐字节一致 | 从 gitcode 镜像自己 `make` | 136 个驱动最全；无任何更新、无任何第三方改动 |
| 只要看电量 | Windows 11 原生"电池状态"小组件 | 不覆盖 2.4G 私有协议设备 |

**不推荐**：`yunluoxincheng`（引入 Qt 6 + 18.9MB 安装包、删掉验签，攻击面最大）、`fuyue521`（基线停在 11-18 之前，78 驱动）、`ferloom` Rust 版（只有 13 个驱动、更新不验签）。

---

*本报告基于 2026-11-23 的公开信息与本地源码审计。GitHub 状态可能随时变化；A 版仓库若恢复，请以其官方 `SHA256SUMS.txt` 为准重新核对。*
