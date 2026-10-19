# MHGU Target Health Display

为《怪物猎人 XX / Generations Ultimate》的当前锁定怪物显示血条。适用于 **MHGU 1.4.0**，需要先安装与本项目配套的 Forge 前置。

## 功能与改动

- 显示当前血量、最大血量和百分比，低血量时自动变色。
- 默认显示名字、大小冠状态和发怒状态；种类、体型倍率、实际体型可分别开启。
- 怪物信息横向排成一行，位置独立配置，不会因为血量数字自动向下挪动。
- 默认跨区域保留已锁定怪物的血条，换区加载画面期间隐藏。
- 适配掌机与主机模式，支持 Forge 菜单修改、保存 INI 和中文名称显示。

## 安装与使用

1. 安装配套 Forge 前置。
2. 将发布包中的 `romfs/` 合并到 MHGU 的 Mod 目录，保留目录结构。
3. 完全退出并重启游戏，在任务中选择要观察的怪物。

主要文件：

```text
romfs/nativeNX/plugins/target_health_display.nro
romfs/nativeNX/forge/fonts/NotoSansSC-Medium.otf
```

字体用于中文名称显示，请一并安装。插件目录只保留一份显血 NRO。升级时覆盖同名 NRO，已有配置保留；移出 NRO 并重启即可停用。

## 修改配置

打开 **Forge → Plugin UI → Target Health Display**。外观设置立即预览，点击 **Save INI** 保存；**Revert changes** 恢复上次保存值，**Defaults** 恢复默认草稿，仍需保存。

首次运行在 **MHGU 当前用户的存档根目录**生成 `target_health_display.ini`，不建子目录。菜单与手动编辑使用同一文件。手动编辑 INI 或更换字体后，需要重启游戏。

| 设置 | 默认值与用途 |
| --- | --- |
| 血条位置与尺寸 | 按 1280×720 填写，位置 `(760,45)`，尺寸 `303×24` |
| 低血量提示 | 25% 及以下显示红色 |
| 血量数字 | 当前 / 最大血量及百分比，字号 17 |
| 怪物信息 | 名字、冠位、发怒状态开启；种类、倍率、实际体型关闭 |
| 信息字号与位置 | 字号 18，偏移 `(0,2)`，与血条间隔 4 |
| 跨区显示 | 默认开启，加载期间隐藏 |
| 颜色 | `#RRGGBBAA`，最后两位为透明度 |

位置、尺寸、颜色、描边和信息开关均可调整。默认配置模板放在 [conf/target_health_display.ini](conf/target_health_display.ini)，编译时嵌入 NRO；游戏运行时仍读写存档根目录的同名文件。

自定义字体放入 `romfs/nativeNX/forge/fonts/`，在 `[text]` 的 `font_file` 中填写有效 TTF 的文件名；该字体需要包含所需中文字形。

## 编译

本项目可以单独下载编译，无需 MHGU-forge 工作区。准备 Git、Rust nightly、`rust-src` 和 devkitPro / devkitARM。Windows 使用 devkitPro 的 MSYS2 Bash，Rust MSVC 工具链需要 C++ Build Tools。

从本项目根目录执行：

```bash
rustup toolchain install nightly --component rust-src
bash "build.sh"
```

最终 NRO 为 `dist/target_health_display.nro`。SDK 和打包工具取自固定的公开 Git 提交，工具安装在本项目 `target/`，不需要全局安装。首次构建需要联网下载依赖；缓存齐全后使用 `bash "build.sh" --offline`。

Windows 也可执行 `powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1`；离线时在末尾加 `-Offline`。devkitPro 默认位置为 `C:/devkitPro`，其它位置用当前进程的 `DEVKITPRO` 指定。

## 验证与打包

安装 Node.js 后运行 `bash "test.sh"`，检查默认配置、菜单保存、NRO 格式和安装流程。可传入配套 Forge NSO 和原版 MHGU ExeFS 目录，额外核对真实导出与游戏指令：

```bash
bash "test.sh" "Forge subsdk9" "MHGU ExeFS目录"
```

测试通过后，使用 Python 3 执行 `python "tools/package.py"`，生成 `dist/target_health_display.zip`，内含中文字体及其授权。也可用 `node "tools/install.cjs" "你的Mod目录/romfs/nativeNX/plugins"` 安装 NRO 和字体。安装和打包均不复制 INI，已有存档配置保留。

编译和默认测试不需要游戏本体文件。`tools/` 内的原版怪物信息、字体排版和取证脚本另需按其参数提供原版数据或 Forge 源码。以上验证不代替实机或模拟器中的狩猎测试。

## 字体授权

随包的 Noto Sans SC 字体使用 SIL Open Font License 1.1，授权见 [NotoSansSC-LICENSE.txt](assets/licenses/NotoSansSC-LICENSE.txt)，来源及校验信息见 [font-source.json](assets/font-source.json)。分发字体时保留其授权。
