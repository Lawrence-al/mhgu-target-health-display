# MHGU 锁定目标显血逆向与实现报告

分析日期：2026-10-08。目标是 MHGU 1.4.0 的 ARM32 main NSO。使用现有 Hex-Rays 导出、只读 NSO 解压器和 Rust / devkitARM 工具链；全过程离线。

已构建独立 `target_health_display.nro`，通过 GU 当前目标 getter 读取本任务当前与最大血量，交给 Forge 画可配置血条。产物为 57,344 字节，SHA256 为 `16d3664358f8c79a57e116ac6971ab3fefb250ccb793759f9b2bb7bbbd734a37`。

## 范围与验证状态

范围和用户授权记录在 [scope.md](work/20261008-target-health/scope.md)，仅包括本地分析、独立插件、配置、离线验证和新增插件文件安装。时间线见 [timeline.md](work/20261008-target-health/timeline.md)。

手机 yuzu、Switch 实机和狩猎中的动态效果未执行验证。下文的原生结构结论来自 main 的 ARM 指令和反编译结果，插件正确性检查来自主机数据测试和装载条件校验。

## Evidence → Finding → Path

| Evidence | source_ref | repro_command | content_hash |
| --- | --- | --- | --- |
| E-001 | work/20261008-target-health/evidence/E-001-native.c | node tools/capture-evidence.cjs | 见 E-001.md 的 SHA256 |
| E-002 | work/20261008-target-health/evidence/E-002-native.c | node tools/capture-evidence.cjs | 见 E-002.md 的 SHA256 |
| E-003 | target/build-verification.json | bash test.sh | 见 E-003.md 的 SHA256 |
| E-004 | target/install-verification.json | node tools/install.cjs | 见 E-004.md 的 SHA256 |

| Finding | severity | evidence_ids | confidence | location | status |
| --- | --- | --- | --- | --- | --- |
| F-001：必须读当前选择，不能用上次 L 转向时缓存的目标 | n/a_re | E-001 | 高，静态已核对 | main+0x18E50、0x18FFC、0x29A58 | verified_static |
| F-002：血量位于怪物 work 内，最大值包含任务倍率 | n/a_re | E-002 | 高，读写指令和比例判断交叉验证 | monster+0x1428 → work+0x370/+0x374 | verified_static |
| F-003：绘图坐标应读取实际可见裁剪尺寸 | n/a_re | E-002/E-003 | 高，静态与缩放测试通过 | graphics+0xB0/+0xB4 | verified_offline |
| F-004：NRO 兼容当前加载器，配置可独立调整 | n/a_re | E-003/E-004 | 高，已执行构建及安装校验 | src、target、插件目录 | verified_offline |

P-001，path_type=callflow：Forge 在原 sMain 更新后调用 `forge_onUpdate` → 原版 `0x18E50` 判断所选目标可用 → `0x18FFC` 获取当前选中怪物 → `monster+0x1428` 取 work → 读取当前／最大血量和图形裁剪尺寸 → 写入插件快照 → `forge_onImGuiFreeRender` 根据配置绘图。

## 原版目标选择

全局 `main+0x0184D750` 保存 sFestaCamera 指针。`main+0x00018FFC(manager)` 从管理器 `+0x54` 获取主镜头，确认其属于 uQuestCamera，然后调用 `0x29A58` 读取镜头 `+0xA6C` 的当前目标序号。目标序号为 0 时没有选择。

`main+0x00018E50(manager)` 除了判断目标序号，还检查目标的原版虚函数状态、怪物 `+0x1054` 与当前区域编号一致，以及 `+0x75C0` 的可用状态，最终返回目标的原版可用性结果。插件每帧使用这个判断再取当前选项；取消选择或离开区域时把快照改为 None。

这里没有使用持续锁定镜头的 `camera+0xA94`：该字段是上次镜头转向目标，不能代表当前选中的怪物。

## 当前与最大血量

`main+0x0009A764(monster, value)` 先取 `*(monster+0x1428)`，再读取 work `+0x374`，将 value 写到 work `+0x370` 并执行原有上下限逻辑。实际指令包含 `movw r2, #0x1428`、`ldr r0, [r0,r2]`、`ldr r2,[r0,#0x374]` 和 `str r1,[r0,#0x370]`。

`main+0x000C04D8(monster, mode)` 从任务数据获取基础血量，指定模式下乘任务倍率，最后把结果写到 work `+0x374`。`main+0x003AED0C` 直接用 work `+0x370 / +0x374` 判断血量比例，进一步确认分子和分母的含义。两个字段均以有符号 32 位整数处理。

插件读取的是游戏已经建立的本任务最大血量，按该值计算比例，配置仅改变绘图。非正数最大血量不产生快照；负当前血量按 0 显示，超过最大值时绘图限定为满血。

## 画面尺寸与线程

全局 `main+0x02126998` 保存图形管理器。`main+0x00BC0874` 把管理器 `+0xB0/+0xB4` 交给 `0x86A24C`，后者以原点 `(0,0)` 调用实际 NVN Window 裁剪函数。插件按这一宽高把 1280×720 配置坐标转换为实际坐标，字号、边框和偏移以实际高度／720 缩放。

Forge 的原版 `0x3DA8FC` 更新 Hook 先运行 original_update，再分发插件更新。游戏对象只在这个更新阶段读取；绘图阶段通过短时共享锁拷贝配置与数值快照，不继续使用游戏对象指针。

## 配置与实现原则

`data.rs` 集中固定地址、INI 解析、血量比例与文字格式、共享状态；`lib.rs` 集中文件读取、游戏访问和 Forge 绘图，保持两个 Rust 源文件。复用已有 Forge 帧回调，不额外安装游戏 Hook，也不在帧循环中动态分配文本缓冲。

配置提供血条位置、尺寸、圆角、边框、整体透明度、填充／背景／边框颜色、低血量阈值及颜色；文字提供当前值、最大值、百分比、字号、TTF 文件、颜色、描边、位置和偏移。所有数值范围与用法记录在 README 和带中文注释的 INI 中。

## 已执行验证

1. 默认 INI 和用户修改解析，UTF-8 BOM、CRLF、行注释和 RGBA／RGB。
2. 拼错配置、非法数字、超界颜色、非法字体路径等错误行号。
3. 血量比例上下限、最大值非正数、低血量阈值与整数百分比的区别。
4. 当前值、当前／最大值、仅百分比、全部隐藏和 32 位大血量的格式。
5. 720p／1080p 的坐标与字号缩放，掌机纹理为 1080p 时仍采用游戏的 720p 可见范围。
6. 多个写线程与读取线程同时访问时，血量和画面字段属于同一份完整快照。

上述 6 项测试通过。另对照 13 条原版 ARM 指令，检查 ARM32 soft-float、三个 NRO 段与 ELF 一致、MOD0 和 BSS、8 个回调导出、19 个强导入均被当前加载器提供，确认没有 `forge_hook_create` 强导入。

已向本机 Ryujinx 已有 Forge mod 新增 NRO 和 INI，安装前已有 7 个插件／配置文件的 SHA256 均保持一致。ZIP 已逐项核对 NRO、INI 和 README 的解压内容与项目文件相同；安装和打包记录见 E-004。

## 复现与附件

在本项目目录执行：

```bash
node "tools/capture-evidence.cjs"
bash "build.sh"
bash "test.sh"
```

用法见 [README.md](README.md)。最终产物和配置为 `target_health_display.nro` 与 `target_health_display.ini`；手机安装包为 `target_health_display.zip`。
