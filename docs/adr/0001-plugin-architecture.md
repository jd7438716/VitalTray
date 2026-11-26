# ADR-0001：插件化架构（按设备域切分）

- 状态：已采纳（M0 已实现骨架）
- 日期：2026-11-23
- 决策者：项目维护者

## 背景

用户要求：鼠标是一个插件、键盘是一个插件、笔记本是一个插件，所有功能都分离，且避免出现大文件。
约束：Rust、MIT 许可、零第三方运行时、单 exe、常驻内存小。

## 需求拆解

1. 新增一类设备（耳机、手柄、手机）不改核心代码。
2. 单个插件不能长成几千行的巨型文件。
3. 插件出错不能拖垮宿主。
4. 用户不需要额外运行时（不强制 .NET/Python/WebView）。
5. 未来允许第三方扩展，但默认不执行不可信代码。

## 候选方案

| 方案 | 机制 | 优点 | 缺点 |
|---|---|---|---|
| A. 编译期 crate | 每个插件一个 crate，实现 trait，静态注册 | 零开销、类型安全、单 exe | 用户不能运行时加插件 |
| B. Rust dylib + C ABI | cdylib + extern "C" 表 | 可运行时加载 | Rust ABI 不稳定，跨 crate 传 trait object 很痛 |
| C. abi_stable / stabby | 稳定 ABI 的 Rust 动态库 | 保留 Rust 类型 | abi_stable 2023 年后停更；stabby 版本迭代快、生态小 |
| D. WASM（wasmtime） | 沙箱内跑插件 | 安全隔离、跨语言 | 运行时 +5–10 MB，HID 要宿主代理，复杂 |
| E. 独立进程 + JSON-RPC | sidecar 子进程 | 崩溃隔离、语言无关、无 ABI 问题 | 每插件多一个进程（内存/启动开销） |

## 决策

A 为默认（v1），E 为 v2 的可选扩展；B/C/D 不做。

理由：
- v1 目标是自己用 + 少量贡献者。编译期 crate 让"加一个设备域"= 加一个百行 crate + 一行注册，无 ABI/沙箱/序列化额外复杂度，产物仍是单 exe。
- "用户自己加设备"用声明式驱动包（TOML）解决（M1），比 WASM/sidecar 简单一个数量级，且不执行任何代码。
- 第三方复杂插件（v2）用 sidecar：语言无关、崩溃隔离、无需稳定 ABI；代价是每插件约 5–10 MB 内存，仅用户显式安装时付出。

## 插件模型

两类插件，都实现 bh_api::Plugin：

1. DeviceDomain（设备域）：mouse / keyboard / laptop，未来 headset / gamepad / phone
   - matches(device) -> Match：识别设备
   - read(device, io) -> Reading：读电量
   - glyph()：图标字形
   - commands()：往右键菜单贡献命令
2. Feature（功能）：历史、续航估算、低电通知、校准向导
   - 只消费读数/事件，不碰 HID

协议库不是插件：logitech、razer、sonix 等协议实现放在普通库 crate（bh-proto-*），被多个域插件共用，避免"鼠标插件和键盘插件各抄一遍 HID++"。

## 目录与注册

~~~text
crates/bh-api                插件 API（无平台依赖）
crates/bh-core               宿主：窗口、托盘、菜单、配置、自启、调度
plugins/bh-plugin-mouse      鼠标域
plugins/bh-plugin-keyboard   键盘域
plugins/bh-plugin-laptop     笔记本域
~~~

注册只有一处：

~~~rust
pub fn domains() -> Vec<&'static dyn DeviceDomain> {
    vec![&MOUSE, &KEYBOARD, &LAPTOP]
}
~~~

## 后果

- 好处：加设备域 = 加 crate + 一行注册；单文件小；无运行时依赖；测试可按 crate 切分。
- 代价：用户不能运行时加载代码插件（v1 用 TOML 补偿）。
- 约束：插件 crate 禁止依赖 windows（保持可移植），平台能力只能通过 bh_api::HidIo 等 trait 注入。
- 门禁：任何 .rs > 400 行 CI 失败（tools/check-size.ps1）。

## 后续

- M1：声明式驱动包（TOML）+ 校验器。
- M2：任务栏 widget 作为 core 模块（所有域共用，不是插件）。
- v2：sidecar 插件协议（JSON-RPC over stdio），先写协议文档再实现。
