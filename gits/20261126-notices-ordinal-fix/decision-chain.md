# 决策链（decision-chain）

## 上下文

`d5578b1` 已推送到 `origin main` 后，CI（windows-latest + `shell: pwsh` 7.x）在新增的
「第三方许可清单未过期」门禁上失败：仓库里的 `THIRD-PARTY-NOTICES.txt` 与 CI 现场生成的不一致，
差异出现在第一份许可全文的标签（简式声明 vs 全文）。本地同一门禁始终 exit 0。

关键事实：**失败的形态是"门禁自己判自己"，而不是"门禁发现了真漂移"** ——
仓库内文件与 `Cargo.lock` 完全同步，只是生成顺序随区域设置漂移。

## 决策

### D1 排序改为 Ordinal 比较，而不是在脚本里固定 culture

| 备选 | 评价 |
|---|---|
| A. 脚本开头把 `CurrentThread.CurrentCulture` 设成 en-US / invariant | 改的是线程全局状态，副作用外溢到调用方；且依赖"每个入口都记得设一次"的纪律，属于隐式约定 |
| B. `Sort-Object -Culture` | PowerShell 无此参数 |
| C. 一律 `[string]::CompareOrdinal`，封装成 `Sort-OrdinalBy`（**选中**） | 语义上就与 locale / PS 版本无关；一处封装覆盖全部 4 个排序点 |

选 C。理由：把"确定性"从**运行环境纪律**降级为**代码属性**，不再依赖谁记得设环境。

### D2 排序键用 ASCII 代号，中文标签只做展示

若排序键继续用中文，虽在 Ordinal 下也稳定，但会让后人（或未来的自己）误以为可以换回
`Sort-Object`。拆成 `Code`（`apache2-full` / `mit` …）+ `Label`（中文）两层，
让"可排序的东西"与"给人看的东西"物理分离。

### D3 不放宽 `-Check`（不改成"比对内容集合"）

只比对集合会吞掉三类漂移：漏更新、手改、顺序抖动——而本项目纪律是"零遗留、门禁要能真失败"。
上一 logid 已用负向测试证明门禁能报错；本次保留逐字节严格比较。

### D4 新建 logid，不改写上一份决策文档

`butler-shared` 红规 2：独立审计产物不得被编排者改写。上一份
`gits/20261126-license-compliance/detail.md:48` 记录的函数行号（42–50）已因本次改动失效，
但那是**历史记录**，不改；改为在新 logid 里记录修复，并在旧 `decision-chain.md` 末尾追加一行指引。

### D5 用新 commit，不 amend / 不 force-push

`d5578b1` 已推送且已产生 CI run。amend + force-push 会抹掉"失败 → 根因 → 修复"这条证据链，
也会破坏他人已引用的 sha。用后续 commit 保留完整轨迹。

## 后果

- `THIRD-PARTY-NOTICES.txt` 第四节小节编号变化（内容、字节总量、许可分布不变）；
  外部若有文档引用"【1】= 简式声明"需更新——目前仅 `detail.md` 有历史描述，已在 D4 处理。
- `gen-notices.ps1` 增加一个通用工具函数 `Sort-OrdinalBy`，后续任何脚本要确定性排序都可复用。
- 门禁从"本地绿、CI 红"恢复为两边一致；**这才是它第一次被跨环境验证**。

## 状态

- 代码与生成物：已修复并重新生成（本 commit）
- CI 真机：待本次推送后的 run 确认（未确认前不宣称"CI 通过"）
- 若仍红：优先怀疑其它文化敏感点（`-match` 的大小写折叠、`-f` 数字格式），
  其次在该 run 的日志里定位第 N 行差异，而不是继续凭猜

## 验收断言（提交后复核）

| # | 断言 | 命令 | 期望 |
|---|---|---|---|
| 1 | 两 locale 产出逐字节相同 | 切换 `CurrentThread.CurrentCulture` 后各生成一次 | SHA256 相等 |
| 2 | 最接近 CI 的本地场景门禁绿 | en-US 文化下 `-Check` | exit 0 |
| 3 | 生成物体量与分布未变 | 文件长度 / `^【\d+】` 计数 | 44,371 B / 10 份 |
| 4 | CI 真机绿 | `gh run list --commit <sha>` | conclusion=success |
