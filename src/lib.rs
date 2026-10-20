//! MHGU 1.4.0：读取当前选中怪物的血量，用 Forge 绘制可配置血条。
#![no_std]

mod data;
#[path = "../shared/plugin_config.rs"]
mod plugin_config;

use core::{
    ffi::{CStr, c_char, c_void},
    mem, ptr,
    sync::atomic::Ordering,
};
use data::*;
use forge::{
    imgui,
    log::prelude::{error, info, warn},
    sys::imgui::{ImDrawFlags, ImGuiInputTextFlags, ImGuiSliderFlags, ImGuiTreeNodeFlags, ImVec2},
};

// 字体仅在 ImGui 初始化时写入，其后只由绘图线程读取。
static mut CUSTOM_FONT: Option<imgui::ImFont> = None;
static mut INFO_FONT: Option<imgui::ImFont> = None;
const INFO_FONT_PATH: &CStr = c"app:/nativeNX/forge/fonts/NotoSansSC-Medium.otf";

#[forge::entry]
fn main() {
    let base = forge::mem::text_addr();
    if !compatible(base) {
        return;
    }
    let config = load_config();
    set_font_input(&config);
    SAVED_CONFIG.with(|saved| *saved = config);
    STATE.with(|state| state.config = config);
    GAME_BASE.store(base, Ordering::Relaxed);
    ENABLED.store(config.enabled, Ordering::Relaxed);
    READY.store(true, Ordering::Release);
    info!("mhgu-target-health-display v0.4: main=0x{base:08X}");
}

fn file_exists(path: &core::ffi::CStr) -> bool {
    // 直接使用 C 字符串，避免 SDK 的 CString 包装带入 malloc/free 强导入。
    unsafe { forge::sys::fs::forge_fs_fileExists(path.as_ptr()) }
}

fn load_config() -> Config {
    plugin_config::HEALTH.load(Config::DEFAULT, Config::parse)
}

#[unsafe(no_mangle)]
pub extern "C" fn forge_onImGuiInit(
    _context: *mut c_void,
    _alloc: *mut c_void,
    _free: *mut c_void,
) {
    if let Some(file) = STATE.with(|state| state.config.font_file) {
        let path = file.path();
        match file_exists(path)
            .then(|| imgui::ImFont::load(path))
            .flatten()
        {
            Some(font) => unsafe { CUSTOM_FONT = Some(font) },
            None => warn!(
                "Could not load {}; using Forge font",
                path.to_str().unwrap()
            ),
        }
    }
    match file_exists(INFO_FONT_PATH)
        .then(|| imgui::ImFont::load(INFO_FONT_PATH))
        .flatten()
    {
        Some(font) => unsafe { INFO_FONT = Some(font) },
        None => warn!("Chinese information font missing; install the full health display package"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(_dt: f32) {
    if !READY.load(Ordering::Acquire) {
        return;
    }
    // Forge 在 sMain 原版更新结束后调用。只在这里读怪物对象，绘图只拿数值快照。
    let base = GAME_BASE.load(Ordering::Relaxed);
    let config = STATE.with(|state| state.config);
    let snapshot = if ENABLED.load(Ordering::Relaxed) {
        unsafe { selected_health(base, &config) }
    } else {
        None
    };
    STATE.with(|state| state.snapshot = snapshot);
}

unsafe fn selected_health(base: u32, config: &Config) -> Option<Snapshot> {
    let show_other_areas = config.show_other_areas;
    let usable: unsafe extern "C" fn(u32) -> i32 = unsafe { mem::transmute(base + TARGET_USABLE) };
    let index: unsafe extern "C" fn(u32) -> u32 = unsafe { mem::transmute(base + SELECTED_INDEX) };
    let count: unsafe extern "C" fn() -> u32 = unsafe { mem::transmute(base + TARGET_COUNT) };
    let selected: unsafe extern "C" fn(u32) -> u32 =
        unsafe { mem::transmute(base + SELECTED_TARGET) };
    let manager = unsafe { read_u32(base + CAMERA_MANAGER) };
    // 先按任务镜头的选择编号判断生命周期；跨区显血不能复用镜头的同区域限制。
    let slot = unsafe { index(manager) };
    if slot == 0 {
        return None;
    }
    // 进入任务且已有选择时，常驻加载 UI 已建立；启动初期不提前查询 UI 对象。
    if unsafe { loading_visible(base) } {
        return None;
    }
    let camera = unsafe { read_u32(manager + ACTIVE_CAMERA) };
    let available = if unsafe { read_u8(camera + MULTIPART_TARGETS) } != 0 {
        3 // 特殊多部位目标的原版固定选择槽。
    } else {
        unsafe { count() }
    };
    if !selected_slot_exists(slot, available)
        || (!show_other_areas && unsafe { usable(manager) } != 1)
    {
        return None;
    }
    let monster = unsafe { selected(manager) };
    let work = unsafe { read_u32(monster + MONSTER_WORK) };
    let health = Health::new(unsafe { read_u32(work + CURRENT_HP) } as i32, unsafe {
        read_u32(work + MAXIMUM_HP)
    } as i32)?;
    let graphics = unsafe { read_u32(base + GRAPHICS_MANAGER) };
    Some(Snapshot {
        health,
        info: if config.info.any() {
            unsafe { monster_info(base, monster, work) }
        } else {
            MonsterInfo::EMPTY
        },
        display: DisplayArea {
            width: unsafe { read_u32(graphics + DISPLAY_WIDTH) } as f32,
            height: unsafe { read_u32(graphics + DISPLAY_HEIGHT) } as f32,
        },
    })
}

unsafe fn loading_visible(base: u32) -> bool {
    let group: unsafe extern "C" fn(u32, u32) -> u32 =
        unsafe { mem::transmute(base + GET_UI_GROUP) };
    let loading: unsafe extern "C" fn(u32) -> u32 =
        unsafe { mem::transmute(base + GET_LOADING_OBJECT) };
    // 常驻 UI group 0 的 cUIOLoading 对象，由原版 open/close 写入 +0x18。
    let global_group = unsafe { group(read_u32(base + UI_MANAGER), 0) };
    unsafe { read_u8(loading(global_group) + UI_OBJECT_ACTIVE) != 0 }
}

unsafe fn monster_info(base: u32, monster: u32, work: u32) -> MonsterInfo {
    let id = unsafe { read_u16(monster + MONSTER_ID) } as u32;
    let name_index: unsafe extern "C" fn(u32) -> u32 =
        unsafe { mem::transmute(base + MONSTER_NAME_INDEX) };
    let message: unsafe extern "C" fn(u32, u32, u32) -> *const c_char =
        unsafe { mem::transmute(base + GET_MESSAGE) };
    let normalize: unsafe extern "C" fn(u32, u32, u32) -> u32 =
        unsafe { mem::transmute(base + NORMALIZE_MONSTER_ID) };
    let size_and_crown: unsafe extern "C" fn(u32, u32, u32, *mut SizeCrown) =
        unsafe { mem::transmute(base + SIZE_AND_CROWN) };
    let enraged: unsafe extern "C" fn(u32) -> u32 = unsafe { mem::transmute(base + IS_ENRAGED) };
    let names = unsafe { read_u32(base + NAME_MANAGER) };
if id == 50 || id == 0x32 || id == 59 || id == 0x3B || id > 137 {
    return None;
}
    let index = unsafe { name_index(id) };
    // 文本在任务期间由游戏消息资源持有，立即复制成自有 UTF-8，绘图不保存游戏地址。
    let name = if index < MONSTER_NAME_COUNT {
        Text::from_str(
            unsafe { CStr::from_ptr(message(names, 3, index)) }
                .to_str()
                .unwrap_or("Unknown"),
        )
    } else {
        Text::from_str("Unknown")
    };
    let canonical_id = unsafe { normalize(read_u32(base + UI_MANAGER), id, 0) };
    let mut species = Text::from_str("Unknown");
    for record in 0..SPECIES_RECORD_COUNT {
        let entry = base + SPECIES_RECORDS + record * 16;
        if unsafe { read_u16(entry + 4) } as u32 == canonical_id {
            let category = unsafe { read_u8(entry + 6) } as u32;
            let (resource, key) = if category == 14 {
                (17, 9)
            } else {
                (
                    3,
                    unsafe { read_u16(base + SPECIES_MESSAGES + category * 2) } as u32,
                )
            };
            species = Text::from_str(
                unsafe { CStr::from_ptr(message(names, resource, key)) }
                    .to_str()
                    .unwrap_or("Unknown"),
            );
            break;
        }
    }
    let scale = f32::from_bits(unsafe { read_u32(work + MONSTER_SCALE) });
    let mut size = SizeCrown {
        crown: 0xFF,
        padding: [0; 3],
        size_cm: 0.0,
    };
    unsafe {
        size_and_crown(
            read_u32(base + ENEMY_MANAGER),
            id,
            scale_percent(scale),
            &mut size,
        )
    };
    MonsterInfo {
        name,
        species,
        scale,
        size_cm: size.size_cm,
        crown: size.crown,
        enraged: unsafe { enraged(monster) } == 1,
    }
}

#[forge::imgui_free_render]
fn draw_health() {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let state = STATE.with(|state| *state);
    let Some(snapshot) = state.snapshot else {
        return;
    };
    // 加载期间 sMain 可暂停更新；绘图也检查实时开关，不能只信上一帧的快照。
    if unsafe { loading_visible(GAME_BASE.load(Ordering::Relaxed)) } {
        return;
    }
    let config = state.config;
    let display = snapshot.display;
    let scale = display.scale();
    // 使用游戏的 NVN Window 裁剪宽高，不能用 Forge 的整张交换纹理 DisplaySize。
    let start = display.point(config.x, config.y);
    let end = display.point(config.x + config.width, config.y + config.height);
    let min = ImVec2::new(start[0], start[1]);
    let max = ImVec2::new(end[0], end[1]);
    let draw = imgui::DrawList::foreground();
    let rgba = |color: [u8; 4]| {
        imgui::color(
            color[0],
            color[1],
            color[2],
            (color[3] as f32 * config.opacity) as u8,
        )
    };
    let rounding = config
        .rounding
        .min(config.height * 0.5)
        .min(config.width * 0.5)
        * scale;
    draw.push_clip_rect(
        ImVec2::new(0.0, 0.0),
        ImVec2::new(display.width, display.height),
        true,
    );
    draw.add_rect_filled(min, max, rgba(config.background_color), rounding);
    if snapshot.health.current > 0 {
        let fill_right = start[0] + (end[0] - start[0]) * snapshot.health.fraction();
        draw.push_clip_rect(min, ImVec2::new(fill_right, end[1]), true);
        draw.add_rect_filled(
            min,
            max,
            rgba(snapshot.health.fill_color(&config)),
            rounding,
        );
        draw.pop_clip_rect();
    }
    if config.border_width > 0.0 {
        draw.add_rect(
            min,
            max,
            rgba(config.border_color),
            rounding,
            config.border_width * scale,
            ImDrawFlags::None,
        );
    }
    let text = snapshot.health.text(&config);
    if !text.as_str().is_empty() {
        let font = unsafe { CUSTOM_FONT }.unwrap_or_else(imgui::ImFont::current);
        let font_size = config.font_size * scale;
        imgui::push_font(&font, Some(font_size));
        let size = imgui::calc_text_size(text.as_str(), false, 0.0);
        let x = (start[0] + end[0] - size.x) * 0.5 + config.text_offset_x * scale;
        let y = match config.text_position {
            TextPosition::Inside => (start[1] + end[1] - size.y) * 0.5,
            TextPosition::Above => start[1] - size.y - 6.0 * scale,
            TextPosition::Below => end[1] + 6.0 * scale,
        } + config.text_offset_y * scale;
        let edge = config.outline_width * scale;
        if edge > 0.0 {
            for (dx, dy) in [(-edge, 0.0), (edge, 0.0), (0.0, -edge), (0.0, edge)] {
                draw.add_text_with_font(
                    &font,
                    font_size,
                    ImVec2::new(x + dx, y + dy),
                    rgba(config.outline_color),
                    text.as_str(),
                );
            }
        }
        draw.add_text_with_font(
            &font,
            font_size,
            ImVec2::new(x, y),
            rgba(config.text_color),
            text.as_str(),
        );
        imgui::pop_font();
    }
    draw.pop_clip_rect();
    draw_monster_info(snapshot, config);
}

fn draw_monster_info(snapshot: Snapshot, config: Config) {
    if !config.info.any() {
        return;
    }
    let text = snapshot.info.text(config.info);
    let display = snapshot.display;
    let scale = display.scale();
    let font = unsafe { CUSTOM_FONT.or(INFO_FONT) }.unwrap_or_else(imgui::ImFont::current);
    let mut size = config.info.font_size * scale;
    imgui::push_font(&font, Some(size));
    let mut measured = imgui::calc_text_size(text.as_str(), false, 0.0);
    let available = display.width - 8.0 * scale;
    if measured.x > available {
        imgui::pop_font();
        size = (size * available / measured.x).max(8.0 * scale);
        imgui::push_font(&font, Some(size));
        measured = imgui::calc_text_size(text.as_str(), false, 0.0);
    }
    let start = info_origin(&config, display, [measured.x, measured.y]);
    let rgba = |c: [u8; 4]| imgui::color(c[0], c[1], c[2], (c[3] as f32 * config.opacity) as u8);
    let draw = imgui::DrawList::foreground();
    draw.push_clip_rect(
        ImVec2::new(0.0, 0.0),
        ImVec2::new(display.width, display.height),
        true,
    );
    let position = ImVec2::new(start[0], start[1]);
    let edge = config.outline_width * scale;
    if edge > 0.0 {
        for (dx, dy) in [(-edge, 0.0), (edge, 0.0), (0.0, -edge), (0.0, edge)] {
            draw.add_text_with_font(
                &font,
                size,
                ImVec2::new(position.x + dx, position.y + dy),
                rgba(config.outline_color),
                text.as_str(),
            );
        }
    }
    draw.add_text_with_font(
        &font,
        size,
        position,
        rgba(config.info.color),
        text.as_str(),
    );
    imgui::pop_font();
    draw.pop_clip_rect();
}

#[forge::imgui_render]
fn draw_settings() {
    imgui::push_id(imgui::Id::Str(c"target_health_display"));
    if imgui::tree_node(c"Target Health Display") {
        draw_settings_body();
        imgui::tree_pop();
    }
    imgui::pop_id();
}

fn draw_settings_body() {
    if !READY.load(Ordering::Acquire) {
        imgui::text("Game function signatures did not match. See Forge log.");
        return;
    }
    let original = STATE.with(|state| state.config);
    let mut config = original;
    imgui::text("Changes preview immediately. Save INI to keep them after restart.");
    imgui::checkbox(c"Show selected monster health", &mut config.enabled);
    imgui::checkbox(c"Show health in other areas", &mut config.show_other_areas);
    if imgui::collapsing_header(c"Monster information", ImGuiTreeNodeFlags::None) {
        imgui::checkbox(c"Monster name", &mut config.info.show_name);
        imgui::checkbox(
            c"Crown (small / large gold, large silver)",
            &mut config.info.show_crown,
        );
        imgui::checkbox(c"Monster species", &mut config.info.show_species);
        imgui::checkbox(c"Enraged state", &mut config.info.show_enraged);
        imgui::checkbox(c"Size multiplier", &mut config.info.show_scale);
        imgui::checkbox(c"Actual size (cm)", &mut config.info.show_size);
        float_control(
            c"Information font size",
            &mut config.info.font_size,
            8.0,
            128.0,
        );
        float_control(
            c"Information X offset",
            &mut config.info.offset_x,
            -1280.0,
            1280.0,
        );
        float_control(
            c"Information Y offset",
            &mut config.info.offset_y,
            -720.0,
            720.0,
        );
        float_control(
            c"Information row gap",
            &mut config.info.line_spacing,
            0.0,
            100.0,
        );
        let mut above = config.info.position == TextPosition::Above;
        if imgui::checkbox(c"Place information above health bar", &mut above) {
            config.info.position = if above {
                TextPosition::Above
            } else {
                TextPosition::Below
            };
        }
        color_control(c"Information color", &mut config.info.color);
    }
    if imgui::collapsing_header(c"Health numbers and font", ImGuiTreeNodeFlags::None) {
        imgui::checkbox(c"Show current HP", &mut config.show_values);
        imgui::checkbox(c"Show maximum HP", &mut config.show_max);
        imgui::checkbox(c"Show HP percentage", &mut config.show_percent);
        float_control(c"HP font size", &mut config.font_size, 8.0, 128.0);
        float_control(
            c"HP text X offset",
            &mut config.text_offset_x,
            -1280.0,
            1280.0,
        );
        float_control(
            c"HP text Y offset",
            &mut config.text_offset_y,
            -720.0,
            720.0,
        );
        float_control(c"Text outline width", &mut config.outline_width, 0.0, 10.0);
        let mut place = match config.text_position {
            TextPosition::Inside => 0,
            TextPosition::Above => 1,
            TextPosition::Below => 2,
        };
        if imgui::slider_int(
            c"HP text: 0 inside / 1 above / 2 below",
            &mut place,
            0,
            2,
            ImGuiSliderFlags::AlwaysClamp,
        ) {
            config.text_position = match place.clamp(0, 2) {
                1 => TextPosition::Above,
                2 => TextPosition::Below,
                _ => TextPosition::Inside,
            };
        }
        color_control(c"HP text color", &mut config.text_color);
        color_control(c"Text outline color", &mut config.outline_color);
        let mut buffer = FONT_INPUT.with(|value| *value);
        if imgui::input_text(c"Font TTF filename", &mut buffer, ImGuiInputTextFlags::None) {
            FONT_INPUT.with(|value| *value = buffer);
            SAVE_STATUS.store(0, Ordering::Relaxed);
            if let Ok(value) = CStr::from_bytes_until_nul(&buffer).unwrap().to_str() {
                config.font_file = if value == "default" {
                    None
                } else {
                    FontFile::new(value).or(config.font_file)
                };
            }
        }
        imgui::text("Put the font in nativeNX/forge/fonts/. Font changes require restart.");
    }
    if imgui::collapsing_header(c"Health bar appearance", ImGuiTreeNodeFlags::None) {
        float_control(c"Bar X", &mut config.x, -1280.0, 2560.0);
        float_control(c"Bar Y", &mut config.y, -720.0, 1440.0);
        float_control(c"Bar width", &mut config.width, 8.0, 1280.0);
        float_control(c"Bar height", &mut config.height, 4.0, 200.0);
        float_control(c"Bar rounding", &mut config.rounding, 0.0, 100.0);
        float_control(c"Border width", &mut config.border_width, 0.0, 10.0);
        float_control(c"Opacity", &mut config.opacity, 0.0, 1.0);
        float_control(
            c"Low HP threshold (%)",
            &mut config.low_hp_threshold,
            0.0,
            100.0,
        );
        color_control(c"Bar fill color", &mut config.fill_color);
        color_control(c"Bar background color", &mut config.background_color);
        color_control(c"Bar border color", &mut config.border_color);
        color_control(c"Low HP color", &mut config.low_hp_color);
    }
    imgui::separator();
    if imgui::button(c"Revert changes", Default::default()) {
        config = SAVED_CONFIG.with(|saved| *saved);
        set_font_input(&config);
        SAVE_STATUS.store(0, Ordering::Relaxed);
    }
    imgui::same_line(0.0, -1.0);
    if imgui::button(c"Defaults", Default::default()) {
        config = Config::DEFAULT;
        set_font_input(&config);
        SAVE_STATUS.store(0, Ordering::Relaxed);
    }
    if config != original {
        STATE.with(|state| {
            if config.show_other_areas != original.show_other_areas
                || config.info.any() != original.info.any()
            {
                state.snapshot = None;
            }
            state.config = config;
        });
        ENABLED.store(config.enabled, Ordering::Relaxed);
        SAVE_STATUS.store(0, Ordering::Relaxed);
    }
    let font_valid = FONT_INPUT.with(|value| {
        let name = CStr::from_bytes_until_nul(value).unwrap().to_str();
        name.is_ok_and(|name| name == "default" || FontFile::new(name).is_some())
    });
    if imgui::button(c"Save INI", Default::default()) {
        SAVE_STATUS.store(
            if !font_valid {
                4
            } else if save_config(&config).is_ok() {
                SAVED_CONFIG.with(|saved| *saved = config);
                1
            } else {
                2
            },
            Ordering::Relaxed,
        );
    }
    imgui::text("MHGU save: /target_health_display.ini");
    if !font_valid {
        imgui::text("Font filename must be default or a valid .ttf filename without a directory.");
    } else if config != SAVED_CONFIG.with(|saved| *saved) {
        imgui::text("Unsaved changes. Click Save INI to keep them after restart.");
    }
    match SAVE_STATUS.load(Ordering::Relaxed) {
        1 => imgui::text(
            "Saved to INI. Appearance changes are live; font file changes need restart.",
        ),
        2 => imgui::text("Save failed. See Forge log; settings were not reported as saved."),
        4 => imgui::text("INI was not saved. Correct the font filename first."),
        _ => {}
    }
}

fn set_font_input(config: &Config) {
    let name = config
        .font_file
        .as_ref()
        .map_or("default", FontFile::filename);
    FONT_INPUT.with(|buffer| {
        buffer.fill(0);
        buffer[..name.len()].copy_from_slice(name.as_bytes());
    });
}

fn float_control(label: &CStr, value: &mut f32, min: f32, max: f32) {
    let previous = *value;
    if imgui::input_float(label, value, 1.0, 10.0, ImGuiInputTextFlags::None) {
        *value = if value.is_finite() {
            value.clamp(min, max)
        } else {
            previous
        };
    }
}

fn color_control(label: &CStr, color: &mut [u8; 4]) {
    plugin_config::color4(label, color);
}

fn save_config(config: &Config) -> Result<(), &'static str> {
    let mut text = Text::<CONFIG_MAX_BYTES>::new();
    config
        .write_ini(&mut text)
        .map_err(|_| "Configuration too large.")?;
    plugin_config::HEALTH.save(text.as_str())
}

unsafe fn read_u32(address: u32) -> u32 {
    unsafe { ptr::read_volatile(address as *const u32) }
}

unsafe fn read_u8(address: u32) -> u8 {
    unsafe { ptr::read_volatile(address as *const u8) }
}

unsafe fn read_u16(address: u32) -> u16 {
    unsafe { ptr::read_volatile(address as *const u16) }
}

fn compatible(base: u32) -> bool {
    for &(offset, expected) in SIGNATURES {
        let actual = unsafe { read_u32(base + offset) };
        if actual != expected {
            error!(
                "mhgu-target-health-display: main+0x{offset:08X}=0x{actual:08X}, expected 0x{expected:08X}; plugin disabled"
            );
            return false;
        }
    }
    true
}
