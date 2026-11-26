# Logitech HID++ 2.0（只记录协议事实）

> 用途：本项目的洁净室笔记。只记录帧格式/字段，不摘抄任何 GPL 代码。
> 来源：Logitech 公开的 HID++ 2.0 文档 + 社区项目（Solaar、libratbag 等）的协议描述。

## 传输

- report id：长报文 `0x11`（20 字节），短报文 `0x10`（7 字节）。
- 帧结构（长）：`[0x11, deviceIndex, featureIndex, (function<<4)|swId, params...]`
- deviceIndex：直连（USB/蓝牙）通常 `0xFF`；接收器（Unifying/Bolt）里是配对槽位 `1..6`。

## 特性发现（IRoot）

- featureIndex `0x00`，function `0x00` = getFeature
- 请求参数：`[featureIdHigh, featureIdLow]`（2 字节大端）
- 应答参数：`[featureIndex, featureType, featureVersion]`；`featureIndex == 0` 表示不支持

## 电池（0x1000 BATTERY_STATUS）

- function `0x00` = getBatteryLevelStatus，无参数
- 应答参数（前 5 字节）：
  | 偏移 | 含义 | 取值 |
  |---|---|---|
  | 0 | 电量百分比 | 0–100 |
  | 1 | 电池状态 | 0=放电, 1=充电, 2=接近充满, 3=已充满, 4=补充电, 5=慢充, 6=无效 |
  | 2 | 外部供电 | 0=未接, 1..=已接 |
  | 3–4 | 电池电压 | 小端 mV |
- 备用特性 `0x1001 BATTERY_VOLTAGE`（function 0x00）只给电压，需自建 mV→% 曲线。

## 错误

- 应答 `featureIndex == 0xFF` 表示错误，参数首字节为错误码：
  1 无效子命令 / 2 无效地址 / 3 无效值 / 4 连接失败 / 5 越界 / 6 不支持。

## 已知限制（本项目）

- 接收器同时配对鼠标+键盘时，需要 HID++ 1.0 寄存器读取配对表才能区分槽位；M1 只试 `0xFF` 与 `1..6`，取第一个应答。
- 结论：直连设备（Lightspeed 专用接收器、蓝牙）可用；多设备 Unifying 接收器需 M1.5 补配对表。
