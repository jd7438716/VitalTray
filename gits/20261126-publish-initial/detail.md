# 执行明细（detail）

## 环境核查（只读）

```text
git -C . rev-parse --is-inside-work-tree   -> fatal: not a git repository
git ls-remote git@github.com:jd7438716/battery-hub.git -> (无输出, exit 0)
ssh -T git@github.com -> Hi jd7438716! You've successfully authenticated
```

## 目录体量与取舍

| 目录 | 文件数 | 体积 | 处置 |
|---|---|---|---|
| `crates/` | 60 | 0.20 MB | 入库 |
| `docs/` | 5 | 0.22 MB | 入库 |
| `plugins/` | 8 | 0.01 MB | 入库 |
| `tools/` | 3 | <0.01 MB | 入库 |
| 根文档 | 7 | 0.09 MB | 入库（README/PLAN/DEVLOG/report + Cargo.toml/lock/.gitignore） |
| `target/` | 5519 | 1191.7 MB | 忽略（原有规则） |
| `research/` | 1786 | 48.3 MB | **新增忽略**（8 个第三方 clone，各带 `.git`；`bin/` 24.4 MB 二进制） |
| `tmp/` | 27 | 5.5 MB | **新增忽略**（调试截图/日志/临时 `.ps1`；旧规则只挡 `tmp/*.png`） |

## 执行步骤

```powershell
git init -b main
git config core.quotepath false          # 中文路径正常显示
git add -A
git commit -F <消息文件>
git remote add origin git@github.com:jd7438716/battery-hub.git
git push -u origin main
```

## 同步产出

- 新增 `LICENSE`（MIT，与 `Cargo.toml` 的 `license = "MIT"` 对齐）
- 改写 `.gitignore`（分组注释 + 新增 `tmp/`、`research/`）
- 新增本决策记录 `gits/20261126-publish-initial/`
