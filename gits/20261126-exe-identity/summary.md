# 提交「exe 身份补齐 + 托盘健壮性」变更（summary）

- logid: `20261126-exe-identity`
- 时间: 2026-11-26 15:45 (Asia/Shanghai)
- 基线: `c7a1023 fix(panel,poller): 幽灵设备可发现性`
- 目标: 用户问「应用是不是没有图标」——查证发现是 exe 零资源（图标/版本信息/应用清单全缺），
  顺带把同一类的「托盘消失 / 关掉托盘失联 / 双击没反应」一起修掉

## 结论

不是「图标没设置」，而是**一个 Windows 资源都没有**：`rsrc_rva = 0x0`，`RT_GROUP_ICON / RT_VERSION /
RT_MANIFEST` 全不存在。资源管理器取到的是通用白窗图标，属性页一片空白，原生菜单也走经典样式。

本次改动：

1. `crates/bh-app/build.rs`（新）：零第三方依赖，自己找 Windows SDK 的 `rc.exe`，
   把 ICON + VERSIONINFO + MANIFEST 编进 exe；版本号从 `CARGO_PKG_VERSION` 现生成。
2. `assets/batteryhub.ico`（新）：竖向绿色电池（`#6CCB5F` = 状态色板 Good 深色），16–256 九档；
   壳宽 46%、触点 22%、壳厚 5.5%，与托盘字形同一套比例。
3. 清单：comctl32 v6（原生菜单跟随系统主题）+ PerMonitorV2 + Win10/11 supportedOS + longPathAware + asInvoker。
4. 运行时：`TaskbarCreated` 重挂托盘；第二个实例唤出面板（`WAKE_MESSAGE`）；关托盘时给出恢复提示；
   窗口类 `hIcon` 与托盘兜底图标改用内嵌图标。
5. 托盘电池字形由横向改竖向（状态色语义不变）。
6. `tools/check-exe.ps1`（新）+ CI/release 接入：缺图标/版本/清单即失败。

## 关键事实

| 项 | 值 |
|---|---|
| 改动 | 9 个 .rs / 2 个 workflow / 4 篇文档 / 1 个新构建脚本 / 1 个工具 / 1 个二进制资源 |
| 资源目录 | `rva=0x0` → `rva=0x76000`（GROUP_ICON / ICON / VERSION / MANIFEST） |
| release 体积 | 470,528 → 500,224 B（+29.7 KB，主要是 24 KB 图标） |
| `cargo test --all-targets` | ✅ 71 项全绿 |
| clippy | ✅ 0 error（18 项既有风格 warning，未新增） |
| 行数门禁 | ✅ 66 文件 0 超限 0 警告 |
| `tools/check-exe.ps1` | ✅ 新 exe 5 项 PASS；对旧 exe 复跑 5 项 FAIL（门禁确实拦得住） |
| 实机 `TaskbarCreated` | ✅ 日志 `taskbar recreated; re-adding icon` |
| 实机第二实例 | ✅ `wake from second instance: show panel`，面板在 (527,40) 弹出 |
| 开始菜单快捷方式 | ✅ 取到新图标（绿电池 + 快捷方式角标） |

## 为什么这算「用户明确反馈的质量问题」

用户看到的是「下载下来一个没有图标的 exe」，背后是发布流水线从不检查资源。
只补图标治标；本次把检查写进 `check-exe.ps1` 并接进 CI/release，才算堵住复发路径。
