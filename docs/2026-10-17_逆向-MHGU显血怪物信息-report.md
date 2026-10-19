# MHGU 显血 v0.3 新增怪物信息报告

日期：2026-10-17。新增名字、种类、发怒状态、体型倍率、原版实际体型与大小冠；原有 HP 百分比默认开启。各项均通过 INI 配置。按用户最新要求，暂停新增 Plugins 菜单编辑／保存，这次交付保留原有两个临时开关。

## Scope

授权范围及静态字段证据见 [analysis/scope.md](work/20261016-monster-info/analysis/scope.md) 与 [字段报告](work/20261016-monster-info/analysis/2026-10-16_逆向-MHGU怪物信息-report.md)。本次只交付 target_health_display.nro、对应 INI 和说明；不修改两份盾斧插件或 Forge 前置。

## Evidence

| E-id | source_ref | repro_command | content_hash SHA256 |
| --- | --- | --- | --- |
| E-001 | [metadata-evidence.json](work/20261016-monster-info/analysis/metadata-evidence.json) | node work/20261016-monster-info/analysis/extract-metadata.cjs | da43020986e964d2a9f736099d9ac9260044688b6ec2be08c227e5d41185f575 |
| E-002 | [build-verification.json](work/20261016-monster-info/evidence/build-verification.json) | bash build.sh 与 bash test.sh | 98ca473dd58fd6460a86da6d05e9b4bac781015638672f0188bd07b4c6a461d8 |
| E-003 | [metadata-native-verification.json](work/20261016-monster-info/evidence/metadata-native-verification.json) | Python 执行 tools/verify-metadata-native.py | b27911ae0e89cf85855638cb3c59f41b3d567be8ad94ac39de62375a41f056f6 |

## Findings

| F-id | severity | evidence_ids | confidence / status | location | 结论 |
| --- | --- | --- | --- | --- | --- |
| F-001 | n/a_re | E-001,E-003 | 原版字节与资源及受控 ARM 执行已验证 | 50B64、55BC4C、55FF98、162D9F0 | 名字使用 UTF-8；类别复用原版表，未知类别用资源17/9，包含第16项 Temnoceran。 |
| F-002 | n/a_re | E-001,E-003 | 原版字节与受控 ARM 执行已验证 | work+1B0、67310 | 体型倍率用 mEmChgScale；先加0.001乘100再截断，实际体型/大小冠使用当前原版资源阈值。 |
| F-003 | n/a_re | E-001,E-003 | 256状态值已执行验证 | 8172C、work+518 | 只有状态1为发怒，不能把状态2或狞猛化并入。 |
| F-004 | n/a_re | E-002 | 编译与数据回归通过；游戏动态待测 | src/data.rs、src/lib.rs | 仍是两个 Rust 文件；快照只含自有文字与数值，保留跨区显血和加载隐藏。 |

## Path

P-001，path_type=callflow：原版 sMain 更新后 → 当前任务选中目标 → 加载 UI 状态检查 → HP 和怪物 ID → 游戏消息表复制名字/种类 → 原版体型、冠及怒状态查询 → 自有快照 → 绘图前再检查加载 UI → Forge 文字与血条绘制。

## 验证与复现

在本项目目录用配置好的 devkitPro Bash 执行 bash build.sh、bash test.sh；另用现有 Python 执行 tools/verify-metadata-native.py。构建遵循工作区 docs/2026-10-16_Forge修复回迁原项目.md，使用原项目与本地 cargo-forge，没有引用工作目录临时插件源码编译。

11 项数据测试覆盖旧 INI、全部新增开关、Unicode边界、文字组合、原版百分截断、无体型记录、HP与线程快照。核对 31 条 main 指令、19 个强导入、108 个可写重定位。原版 ARM 测试执行 137 个名字索引、特殊ID归并、129 个分类映射、256 个怒状态及 8 个冠边界、3 个无体型记录情形。

产物 77824 字节，SHA256：191592e58c6c2dcfaf8a3aa6602092c466577608c0a0d73cfd6eea3b28386b50。参数含义和安装升级见 README.md。配置和原版对象生命周期沿用此前实现；没有增加重复空指针检查。缺少游戏体型记录时显示 N/A。中文汉化的名字需要含中文字形的 TTF，默认 Roboto 没有中文字形。

手机、实机与游戏内动态显示尚未验证；模拟执行不等同实际狩猎。

## Timeline 与 Checklist

- 2026-10-16：确认用户所指为怪物种类和发怒状态；完成24函数及资源取证。
- 2026-10-17：按最新指令暂停菜单编辑／保存，将草稿单独保留于 work/20261016-monster-info/menu-draft，交付源码撤出该部分。
- 2026-10-17：数据、二进制、可写重定位及原版 ARM 回归全部通过。

- [x] 当前请求范围明确，未覆盖另一个对话的盾斧源码。
- [x] Evidence → Finding → Path 可追踪。
- [x] 复用原版函数和表，不重复实现大小冠规则。
- [x] 保留两个 Rust 文件、固定缓冲和原有配置兼容。
- [x] 编译与离线验证完成；明确标注未做游戏实测。

## Plugins 菜单与 INI 的分析

盾斧菜单的现有草稿在启动时读取配置，编辑 draft，点击 Save INI 写 SD /forge/plugins/ 同名文件，下次重启才重新读取并安装对应补丁；运行中修改外部 INI 不会自动刷新菜单。SD 配置存在时覆盖 ROMFS 模板，不会写入只读 app:/nativeNX/plugins。依据为 MHGU-Charge-Blade-to-4G/shared/settings_menu.rs、shared/config_io.rs 与两项目 ui.rs 的当前源码，代码草稿尚未最终验证和部署。

因此菜单是 INI 的可选编辑界面，不是第二套必须保留的配置源。对需要重启才安装的盾斧补丁，直接编辑 INI 足够；菜单的收益是手机端便于调整大量开关、帧数和范围。按用户决定，本轮暂缓该菜单工作；显血新增信息只通过原有 INI 路径配置。

- 2026-10-17：已更新本地显血 NRO、补齐 INI 字段并开启旧缺省配置的 HP 百分比；其它 9 个插件／配置哈希保持不变。已用实际解析器核对安装后的 INI。ZIP 内容与发布 NRO、模板、说明逐字节一致。
