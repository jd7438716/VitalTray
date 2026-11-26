# 决策链（decision-chain）

## D1. `research/` 不入库
- 备选：入库 / 忽略 / 转 submodule
- 选择：**忽略**
- 理由：8 个子目录均为第三方仓库 clone 且各自带 `.git`，直接 `git add` 会变成不可用的嵌套仓库（Git 只记录 gitlink，克隆后是个空目录）；含 24.4 MB 参考二进制与上游源码，体积与许可归属都不该混进本仓库。
- 后果：`research/` 仅本地留存；如需共享调研结论，后续单独写文档或按上游地址在此记录。

## D2. `tmp/` 整目录忽略
- 备选：补 `tmp/*.ps1`、`tmp/*.log` 单条规则 / 整目录忽略
- 选择：**整目录忽略**
- 理由：`tmp/` 是调试产物区（截图、日志、一次性脚本），无稳定产物需要版本化；逐条枚举必然再次漏项（旧规则就漏了 `.ps1`）。
- 后果：`tmp/` 下任何文件不再进入 git 视野。

## D3. 分支名 `main`
- 理由：GitHub 新仓库默认分支为 `main`；空仓库首次 push 的远端分支即默认分支，用 `main` 避免后续改名。

## D4. 补 MIT LICENSE
- 备选：保持无 LICENSE / 加 MIT
- 选择：**加 MIT**（用户确认）
- 理由：`Cargo.toml` 已声明 `license = "MIT"`，且 README 写明自研；缺文件会让 GitHub 无法识别协议、发布到 crates.io 也不合规。

## D5. 未跑 butler-git 五阶段 workflow，改为单 agent 直执行
- 理由：该 workflow 前置假设是「仓库已存在 + 有 status/diff 可比」的提交场景，本任务是空目录建仓 + 首次 push（无 diff 基线，`git init`/`git remote`/`git push` 也不在其阶段覆盖内）。按 skill 守门要求保留：todo 清单、写操作先请许可、决策文档落 `gits/`、提交后只读验证。
- 后果：若后续日常提交需要五阶段编排，再走 `workflow(butler-git.js)`。

## D6. 加 `.gitattributes`：仓库内统一 LF
- 备选：只靠 `core.autocrlf=true` / 加 `.gitattributes`
- 选择：`* text=auto eol=lf`，但 `*.ps1`/`*.bat`/`*.cmd` 用 `eol=crlf`，图片/exe 标 `binary`
- 理由：首次 `git add` 刷了 87 条 "LF will be replaced by CRLF"；规则落在仓库里比依赖每人本机配置可靠。Windows 脚本保留 CRLF 是为了记事本/旧版 PowerShell 直接改不闹心。
- 后果：`core.autocrlf` 的本地差异不再影响仓库内容。

## D7. CI：Windows runner + 固定 Rust 1.88.0 + clippy 不加 `-D warnings`
- 备选：`stable` 浮动 / 固定 `1.88.0`；clippy 严格模式 / 宽松模式
- 选择：`windows-latest` + `dtolnay/rust-toolchain@stable{ toolchain: 1.88.0 }`；clippy 用默认等级
- 理由：项目是 Win32 专用，Linux runner 编不出；固定工具链让「本机能过 = CI 能过」（本机 cargo 1.88.0）。clippy 若是 `-D warnings`，现有 27 条风格 warning（`unnecessary_cast` / `uninlined_format_args` / `unusual_byte_groupings` 等）会直接卡死发布；但不加 `-D` 时 clippy 自身 **默认 deny** 的 lint（如 `not_unsafe_ptr_arg_deref`）仍然生效，门禁不是空的。
- 后果：风格债留到 1.0 之后清；真要开严格模式，先跑 `cargo clippy --fix` 再补 allow 列表。

## D8. 发布自动化：`release.yml`（打 tag 即发版）
- 备选：本地构建后 `gh release create` 手动传 / tag 触发 workflow
- 选择：`push: tags: v*` 触发，runner 上构建 → 校验 tag 与 `bh-app` 版本一致 → 行数门禁 → 自检 → 产出 `batteryhub.exe` + `SHA256SUMS.txt` → `softprops/action-gh-release`
- 理由：exe 由 GitHub runner 构建，来源可追溯（谁下载都能对照 workflow 日志复现）；本地手搓的 exe 无法自证。
- 后果：发版流程固定为「改版本号 → 提交 → 打 tag → 推 tag」；发布说明默认自动生成，可用 `gh release edit` 换成手写。

## D9. clippy 默认 deny 的 3 处 `not_unsafe_ptr_arg_deref`：改 `unsafe`，不 allow
- 位置：`crates/bh-core/src/tray/menu.rs` 的 `show(hwnd: HWND, ...)`
- 选择：`pub fn` → `pub unsafe fn`，补 `# Safety` 文档，唯一调用点 `commands.rs` 用 `unsafe {}` 包住
- 理由：该函数把 `HWND` 直接交给 `SetForegroundWindow` / `TrackPopupMenuEx` / `PostMessageW`，句柄失效就是 UB；用 `allow` 掩盖等于把风险写进习惯。改 `unsafe` 零运行时行为变化，只影响调用约定。
- 后果：`cargo clippy` 从「红」变「0 error / 27 warning」，可以进 CI。

## 待办（后续）
- 风格债 27 条 clippy warning（`cargo clippy --fix` 可自动修大部分）。
- `cargo fmt --check` 未启用：全仓有 182 处 rustfmt 差异，要先一次性 `cargo fmt` 才能进门禁（适合 1.0 之后单独一个格式化提交）。
- 72 小时常驻 / 内存增长 ≤ 5 MB 的稳定性验收（PLAN §9「发布前必跑」）尚未执行。
- `research/` 的来源与调研结论尚未在仓库内留下可追溯记录。
