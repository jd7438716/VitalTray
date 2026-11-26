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

## 待办（本次未做）
- CI：根目录暂无 `.github/workflows`，可加 `cargo build`/`cargo test` + `tools/check-size.ps1` 门禁。
- `research/` 的来源与调研结论尚未在仓库内留下可追溯记录。
