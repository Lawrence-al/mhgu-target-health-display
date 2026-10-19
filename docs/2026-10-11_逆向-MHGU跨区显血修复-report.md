# MHGU 跨区显血与加载隐藏 v0.2 修复报告

日期：2026-10-11。血条默认跨区显示，加载页面隐藏；show_other_areas=false 可恢复原版区域限制。

## Scope 与时间记录

授权及范围见 [scope.md](work/20261011-ui-health-update/scope.md)，记录见 [timeline.md](work/20261011-ui-health-update/timeline.md)。仅改用户要求的两个插件，不改 main／GUI、不做 Git 操作。

## Evidence

source_ref：本地 MHGU 1.4.0 main.c / main 和原版 GUI，详见 manifest。repro_command：在 D:/Work/MHGU-forge 运行 node "mhgu-target-health-display/tools/capture-ui-health-evidence.cjs"；构建核对运行该项目的 bash "build.sh" 和 bash "test.sh"。

| E-id | source_ref | content_hash（SHA256） |
| --- | --- | --- |
| E-001 | [E-001-native.c](work/20261011-ui-health-update/evidence/E-001-native.c) | 485383d182362ec9f46a39c29426b6ec18e819461caa16ca26864ea5d3fe109d |
| E-002 | [E-002-build-verification.json](work/20261011-ui-health-update/evidence/E-002-build-verification.json) | 5988ca1a8883332bbe14d158c11b478daba10aecc09b9983c8c7337c8e360b97 |

## Findings

| F-id | severity | evidence_ids | confidence / status | location | 具体原因与修复 |
| --- | --- | --- | --- | --- | --- |
| F-001 | n/a_re | E-001,E-002 | 已验证静态原因；动态待测 | selected_health | 原 0x18E50 比较 monster+0x1054 与世界+73，排除异区目标；新版按原版任务目标选择与列表取血量。 |
| F-002 | n/a_re | E-001,E-002 | 已验证开关写入；动态待测 | loading_visible、draw_health | cUIOLoading+0x18 打开写 1、关闭写 0；更新时清快照，绘图时也实时读开关，覆盖加载暂停更新的情况。 |

## Path

P-001，path_type=callflow：Forge 原版 sMain 更新后回调 → cUIOLoading 开关 → 当前任务选择编号及目标列表 → 当前目标 getter → monster+0x1428 → work+0x370/+0x374 → 仅数值快照 → 绘图前再查询加载开关 → 绘制血条。

## 改动与验证

源码仍为 data.rs 与 lib.rs，配置默认值让旧 INI 直接兼容；调用游戏现有函数，避免另写 UI 动画或保存怪物地址。新增规则和显示开关集中在原有数据模块。已通过 7 项数据测试，21 条 native main 指令、ARM32 soft-float、NRO 段和当前 Forge 导入核对。绘图快照无游戏对象地址；更新与绘图均检查常驻加载对象开关。

NRO：target_health_display.nro，57344 字节，SHA256：630f7208b41cbb795568501144bc6d1de5e0272ae1c706061b081612a2f5a989。安装脚本先备份，仅补 INI 新键，保留用户颜色、位置和其他插件。

游戏内运行、手机 yuzu 和 Switch 实机尚未验证。待核对怪物先换区、玩家换区的整个加载页面、加载后血量恢复及取消／切换选择。

## Checklist

- [x] 按当前用户要求限定修改范围。
- [x] 原因由 native 函数与资源证据支撑，Evidence → Finding → Path 可追踪。
- [x] 两个 Rust 文件、INI、中文说明及可复现构建检查。
- [x] 编译及离线检查通过。
- [x] 不把静态核对表述为游戏内实测。
