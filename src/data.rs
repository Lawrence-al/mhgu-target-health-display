//! 固定地址、配置、血量快照和绘图数据；只有两个 Rust 源文件。
#![cfg_attr(test, allow(dead_code))]

use core::{
    cell::UnsafeCell,
    ffi::CStr,
    fmt::{self, Write},
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

pub const CONFIG_MAX_BYTES: usize = 8192;
pub const CAMERA_MANAGER: u32 = 0x0184D750;
pub const TARGET_USABLE: u32 = 0x00018E50;
pub const SELECTED_INDEX: u32 = 0x00018F90;
pub const SELECTED_TARGET: u32 = 0x00018FFC;
pub const TARGET_COUNT: u32 = 0x00029870;
pub const ACTIVE_CAMERA: u32 = 0x54;
pub const MULTIPART_TARGETS: u32 = 0xA8E;
pub const UI_MANAGER: u32 = 0x01899458;
pub const GET_UI_GROUP: u32 = 0x0055D2B4;
pub const GET_LOADING_OBJECT: u32 = 0x0065E1D0;
pub const UI_OBJECT_ACTIVE: u32 = 0x18;
pub const MONSTER_WORK: u32 = 0x1428;
pub const CURRENT_HP: u32 = 0x370;
pub const MAXIMUM_HP: u32 = 0x374;
pub const MONSTER_ID: u32 = 0xB5F0;
pub const MONSTER_SCALE: u32 = 0x1B0;
pub const NAME_MANAGER: u32 = 0x01899220;
pub const ENEMY_MANAGER: u32 = 0x01851FE0;
pub const MONSTER_NAME_INDEX: u32 = 0x00050B64;
pub const GET_MESSAGE: u32 = 0x0055BC4C;
pub const SIZE_AND_CROWN: u32 = 0x00067310;
pub const IS_ENRAGED: u32 = 0x0008172C;
pub const NORMALIZE_MONSTER_ID: u32 = 0x0055FF98;
pub const SPECIES_RECORDS: u32 = 0x0162D9F0;
pub const SPECIES_MESSAGES: u32 = 0x016325A8;
pub const SPECIES_RECORD_COUNT: u32 = 123;
pub const MONSTER_NAME_COUNT: u32 = 137;
pub const GRAPHICS_MANAGER: u32 = 0x02126998;
pub const DISPLAY_WIDTH: u32 = 0xB0;
pub const DISPLAY_HEIGHT: u32 = 0xB4;
pub const SET_CURRENT_HP: u32 = 0x0009A764;
pub const SET_MAXIMUM_HP: u32 = 0x000C04D8;
pub const SET_CROP: u32 = 0x0086A24C;

// 对照原版目标 getter、当前/最大血量写入和可见裁剪范围函数。
pub const SIGNATURES: &[(u32, u32)] = &[
    (TARGET_USABLE, 0xE92D4830),
    (TARGET_USABLE + 4, 0xE1A04000),
    (SELECTED_INDEX, 0xE92D4010),
    (SELECTED_INDEX + 4, 0xE5904054),
    (SELECTED_TARGET, 0xE92D4010),
    (SELECTED_TARGET + 4, 0xE5904054),
    (TARGET_COUNT, 0xE92D4010),
    (TARGET_COUNT + 4, 0xE59F01B4),
    (GET_UI_GROUP, 0xE0800101),
    (GET_UI_GROUP + 4, 0xE5900148),
    (GET_LOADING_OBJECT, 0xE3A01002),
    (GET_LOADING_OBJECT + 4, 0xEAFFFA58),
    (SET_CURRENT_HP, 0xE3012428),
    (SET_CURRENT_HP + 8, 0xE7900002),
    (SET_CURRENT_HP + 16, 0xB5801370),
    (SET_CURRENT_HP + 24, 0xE5902374),
    (SET_MAXIMUM_HP + 64, 0xE3010428),
    (SET_MAXIMUM_HP + 68, 0xE7940000),
    (SET_MAXIMUM_HP + 72, 0xE5806374),
    (SET_CROP, 0xE92D4800),
    (SET_CROP + 4, 0xE24DD008),
    (MONSTER_NAME_INDEX, 0xE92D4800),
    (MONSTER_NAME_INDEX + 4, 0xE59F2060),
    (GET_MESSAGE, 0xE590005C),
    (GET_MESSAGE + 4, 0xE5900000),
    (SIZE_AND_CROWN, 0xE92D4BF0),
    (SIZE_AND_CROWN + 4, 0xED2D8B06),
    (IS_ENRAGED, 0xE3011428),
    (IS_ENRAGED + 4, 0xE7900001),
    (NORMALIZE_MONSTER_ID, 0xE300052A),
    (NORMALIZE_MONSTER_ID + 4, 0xE1510000),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontFile([u8; 192]);

impl FontFile {
    pub fn new(filename: &str) -> Option<Self> {
        let prefix = b"app:/nativeNX/forge/fonts/";
        if !filename.ends_with(".ttf")
            || filename
                .bytes()
                .any(|c| c.is_ascii_control() || b"/\\:;#".contains(&c))
            || prefix.len() + filename.len() >= 192
        {
            return None;
        }
        let mut bytes = [0; 192];
        bytes[..prefix.len()].copy_from_slice(prefix);
        bytes[prefix.len()..prefix.len() + filename.len()].copy_from_slice(filename.as_bytes());
        Some(Self(bytes))
    }

    pub fn path(&self) -> &CStr {
        CStr::from_bytes_until_nul(&self.0).unwrap()
    }

    pub fn filename(&self) -> &str {
        self.path()
            .to_str()
            .unwrap()
            .strip_prefix("app:/nativeNX/forge/fonts/")
            .unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextPosition {
    Inside,
    Above,
    Below,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InfoConfig {
    pub show_name: bool,
    pub show_crown: bool,
    pub show_species: bool,
    pub show_enraged: bool,
    pub show_scale: bool,
    pub show_size: bool,
    pub font_size: f32,
    pub position: TextPosition,
    pub offset_x: f32,
    pub offset_y: f32,
    pub line_spacing: f32,
    pub color: [u8; 4],
}

impl InfoConfig {
    pub const DEFAULT: Self = Self {
        show_name: true,
        show_crown: true,
        show_species: false,
        show_enraged: true,
        show_scale: false,
        show_size: false,
        font_size: 18.0,
        position: TextPosition::Below,
        offset_x: 0.0,
        offset_y: 2.0,
        line_spacing: 4.0,
        color: [255, 255, 255, 255],
    };

    pub fn any(self) -> bool {
        self.show_name
            || self.show_crown
            || self.show_species
            || self.show_enraged
            || self.show_scale
            || self.show_size
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub show_other_areas: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rounding: f32,
    pub border_width: f32,
    pub opacity: f32,
    pub fill_color: [u8; 4],
    pub background_color: [u8; 4],
    pub border_color: [u8; 4],
    pub low_hp_threshold: f32,
    pub low_hp_color: [u8; 4],
    pub show_values: bool,
    pub show_max: bool,
    pub show_percent: bool,
    pub font_size: f32,
    pub font_file: Option<FontFile>,
    pub text_color: [u8; 4],
    pub outline_color: [u8; 4],
    pub outline_width: f32,
    pub text_position: TextPosition,
    pub text_offset_x: f32,
    pub text_offset_y: f32,
    pub info: InfoConfig,
}

#[derive(Debug)]
pub struct ConfigError {
    pub line: usize,
    pub message: &'static str,
}

impl Config {
    pub const DEFAULT: Self = Self {
        enabled: true,
        show_other_areas: true,
        x: 760.0,
        y: 45.0,
        width: 303.0,
        height: 24.0,
        rounding: 4.0,
        border_width: 1.0,
        opacity: 1.0,
        fill_color: [80, 220, 110, 235],
        background_color: [15, 20, 25, 190],
        border_color: [230, 230, 230, 230],
        low_hp_threshold: 25.0,
        low_hp_color: [255, 90, 70, 235],
        show_values: true,
        show_max: true,
        show_percent: true,
        font_size: 17.0,
        font_file: None,
        text_color: [255, 255, 255, 255],
        outline_color: [0, 0, 0, 230],
        outline_width: 1.0,
        text_position: TextPosition::Below,
        text_offset_x: 150.0,
        text_offset_y: 1.0,
        info: InfoConfig::DEFAULT,
    };

    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config = Self::DEFAULT;
        let mut section = "";
        for (index, line) in text.trim_start_matches('\u{FEFF}').lines().enumerate() {
            let line = line.split(';').next().unwrap().trim();
            let error = |message| ConfigError {
                line: index + 1,
                message,
            };
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                section = match line {
                    "[bar]" => "bar",
                    "[text]" => "text",
                    "[info]" => "info",
                    _ => return Err(error("unknown section; use [bar], [text], or [info]")),
                };
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| error("expected key = value"))?;
            let value = if value.trim_start().starts_with('#') {
                value.split_ascii_whitespace().next().unwrap()
            } else {
                value.split('#').next().unwrap().trim()
            };
            let number = |min: f32, max: f32| {
                value
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite() && (min..=max).contains(v))
                    .ok_or_else(|| error("number outside documented range"))
            };
            let boolean = || {
                value
                    .parse::<bool>()
                    .map_err(|_| error("value must be true or false"))
            };
            let color =
                || parse_color(value).ok_or_else(|| error("color must be #RRGGBB or #RRGGBBAA"));
            match (section, key.trim()) {
                ("bar", "enabled") => config.enabled = boolean()?,
                ("bar", "show_other_areas") => config.show_other_areas = boolean()?,
                ("bar", "x") => config.x = number(-1280.0, 2560.0)?,
                ("bar", "y") => config.y = number(-720.0, 1440.0)?,
                ("bar", "width") => config.width = number(8.0, 1280.0)?,
                ("bar", "height") => config.height = number(4.0, 200.0)?,
                ("bar", "rounding") => config.rounding = number(0.0, 100.0)?,
                ("bar", "border_width") => config.border_width = number(0.0, 10.0)?,
                ("bar", "opacity") => config.opacity = number(0.0, 1.0)?,
                ("bar", "fill_color") => config.fill_color = color()?,
                ("bar", "background_color") => config.background_color = color()?,
                ("bar", "border_color") => config.border_color = color()?,
                ("bar", "low_hp_threshold") => config.low_hp_threshold = number(0.0, 100.0)?,
                ("bar", "low_hp_color") => config.low_hp_color = color()?,
                ("text", "show_values") => config.show_values = boolean()?,
                ("text", "show_max") => config.show_max = boolean()?,
                ("text", "show_percent") => config.show_percent = boolean()?,
                ("text", "font_size") => config.font_size = number(8.0, 128.0)?,
                ("text", "font_file") => {
                    config.font_file = if value == "default" {
                        None
                    } else {
                        Some(FontFile::new(value).ok_or_else(|| {
                            error("font_file must be a .ttf filename without a directory")
                        })?)
                    };
                }
                ("text", "color") => config.text_color = color()?,
                ("text", "outline_color") => config.outline_color = color()?,
                ("text", "outline_width") => config.outline_width = number(0.0, 10.0)?,
                ("text", "position") => {
                    config.text_position = match value {
                        "inside" => TextPosition::Inside,
                        "above" => TextPosition::Above,
                        "below" => TextPosition::Below,
                        _ => return Err(error("position must be inside, above or below")),
                    }
                }
                ("text", "offset_x") => config.text_offset_x = number(-1280.0, 1280.0)?,
                ("text", "offset_y") => config.text_offset_y = number(-720.0, 720.0)?,
                ("info", "show_name") => config.info.show_name = boolean()?,
                ("info", "show_crown") => config.info.show_crown = boolean()?,
                ("info", "show_species") => config.info.show_species = boolean()?,
                ("info", "show_enraged") => config.info.show_enraged = boolean()?,
                ("info", "show_scale") => config.info.show_scale = boolean()?,
                ("info", "show_size") => config.info.show_size = boolean()?,
                ("info", "font_size") => config.info.font_size = number(8.0, 128.0)?,
                ("info", "position") => {
                    config.info.position = match value {
                        "above" => TextPosition::Above,
                        "below" => TextPosition::Below,
                        _ => return Err(error("info position must be above or below")),
                    }
                }
                ("info", "offset_x") => config.info.offset_x = number(-1280.0, 1280.0)?,
                ("info", "offset_y") => config.info.offset_y = number(-720.0, 720.0)?,
                ("info", "line_spacing") => config.info.line_spacing = number(0.0, 100.0)?,
                ("info", "color") => config.info.color = color()?,
                ("", _) => return Err(error("settings must follow [bar], [text], or [info]")),
                _ => return Err(error("unknown setting in this section")),
            }
        }
        Ok(config)
    }
    pub fn write_ini(&self, output: &mut impl Write) -> fmt::Result {
        let rgba = |c: [u8; 4]| (c[0], c[1], c[2], c[3]);
        writeln!(
            output,
            "; MHGU Target Health Display v0.4\n; MHGU save: /target_health_display.ini\n; Menu and manual edits use this same file.\n[bar]"
        )?;
        writeln!(
            output,
            "enabled = {}\nshow_other_areas = {}",
            self.enabled, self.show_other_areas
        )?;
        writeln!(
            output,
            "x = {}\ny = {}\nwidth = {}\nheight = {}",
            self.x, self.y, self.width, self.height
        )?;
        writeln!(
            output,
            "rounding = {}\nborder_width = {}\nopacity = {}",
            self.rounding, self.border_width, self.opacity
        )?;
        for (key, color) in [
            ("fill_color", self.fill_color),
            ("background_color", self.background_color),
            ("border_color", self.border_color),
            ("low_hp_color", self.low_hp_color),
        ] {
            let (r, g, b, a) = rgba(color);
            writeln!(output, "{key} = #{r:02X}{g:02X}{b:02X}{a:02X}")?;
        }
        writeln!(
            output,
            "low_hp_threshold = {}\n\n[text]",
            self.low_hp_threshold
        )?;
        writeln!(
            output,
            "show_values = {}\nshow_max = {}\nshow_percent = {}\nfont_size = {}",
            self.show_values, self.show_max, self.show_percent, self.font_size
        )?;
        let filename = self
            .font_file
            .as_ref()
            .map_or("default", FontFile::filename);
        writeln!(output, "font_file = {filename}")?;
        for (key, color) in [
            ("color", self.text_color),
            ("outline_color", self.outline_color),
        ] {
            let (r, g, b, a) = rgba(color);
            writeln!(output, "{key} = #{r:02X}{g:02X}{b:02X}{a:02X}")?;
        }
        writeln!(
            output,
            "outline_width = {}\nposition = {}\noffset_x = {}\noffset_y = {}",
            self.outline_width,
            self.text_position.name(),
            self.text_offset_x,
            self.text_offset_y
        )?;
        writeln!(
            output,
            "\n[info]\nshow_name = {}\nshow_crown = {}\nshow_species = {}\nshow_enraged = {}\nshow_scale = {}\nshow_size = {}",
            self.info.show_name,
            self.info.show_crown,
            self.info.show_species,
            self.info.show_enraged,
            self.info.show_scale,
            self.info.show_size
        )?;
        writeln!(
            output,
            "font_size = {}\nposition = {}\noffset_x = {}\noffset_y = {}\nline_spacing = {}",
            self.info.font_size,
            self.info.position.name(),
            self.info.offset_x,
            self.info.offset_y,
            self.info.line_spacing
        )?;
        let (r, g, b, a) = rgba(self.info.color);
        writeln!(output, "color = #{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

impl TextPosition {
    pub fn name(self) -> &'static str {
        match self {
            Self::Inside => "inside",
            Self::Above => "above",
            Self::Below => "below",
        }
    }
}

pub fn selected_slot_exists(index: u32, available: u32) -> bool {
    // 原版 0 表示未选择；普通目标会在退出任务／移除怪物时从目标列表中移除。
    index > 0 && index <= available
}

fn parse_color(value: &str) -> Option<[u8; 4]> {
    if let Some(hex) = value.strip_prefix('#') {
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let mut rgba = [0, 0, 0, 255];
        for (i, out) in rgba.iter_mut().take(hex.len() / 2).enumerate() {
            *out = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        return Some(rgba);
    }
    let mut parts = value.split(',');
    let rgba = [
        parts.next()?.trim().parse().ok()?,
        parts.next()?.trim().parse().ok()?,
        parts.next()?.trim().parse().ok()?,
        parts.next().map_or(Some(255), |v| v.trim().parse().ok())?,
    ];
    parts.next().is_none().then_some(rgba)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Health {
    pub current: u32,
    pub maximum: u32,
}

impl Health {
    pub fn new(current: i32, maximum: i32) -> Option<Self> {
        // 非正数的最大血量还没有可绘制的比例；负血量显示为 0。
        (maximum > 0).then(|| Self {
            current: current.clamp(0, maximum) as u32,
            maximum: maximum as u32,
        })
    }

    pub fn fraction(self) -> f32 {
        self.current as f32 / self.maximum as f32
    }

    pub fn percent(self) -> u32 {
        ((self.current as u64 * 100 + self.maximum as u64 / 2) / self.maximum as u64) as u32
    }

    pub fn fill_color(self, config: &Config) -> [u8; 4] {
        if config.low_hp_threshold > 0.0 && self.fraction() * 100.0 <= config.low_hp_threshold {
            config.low_hp_color
        } else {
            config.fill_color
        }
    }

    pub fn text(self, config: &Config) -> Text {
        let mut text = Text::new();
        if config.show_values {
            let _ = write!(text, "{}", self.current);
            if config.show_max {
                let _ = write!(text, " / {}", self.maximum);
            }
        }
        if config.show_percent {
            if config.show_values {
                let _ = write!(text, " ({}%)", self.percent());
            } else {
                let _ = write!(text, "{}%", self.percent());
            }
        }
        text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text<const N: usize = 256> {
    bytes: [u8; N],
    length: usize,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            length: 0,
        }
    }

    pub fn from_str(value: &str) -> Self {
        let mut text = Self::new();
        for character in value.chars() {
            let mut encoded = [0; 4];
            if text.write_str(character.encode_utf8(&mut encoded)).is_err() {
                break;
            }
        }
        text
    }

    pub fn as_str(&self) -> &str {
        // 只经 fmt::Write 或完整 UTF-8 字符写入，不截断多字节字符。
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.length]) }
    }
}

impl<const N: usize> Write for Text<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.length + text.len();
        if end > self.bytes.len() {
            return Err(fmt::Error);
        }
        self.bytes[self.length..end].copy_from_slice(text.as_bytes());
        self.length = end;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonsterInfo {
    pub name: Text<128>,
    pub species: Text<96>,
    pub scale: f32,
    pub size_cm: f32,
    pub crown: u8,
    pub enraged: bool,
}

impl MonsterInfo {
    pub const EMPTY: Self = Self {
        name: Text::new(),
        species: Text::new(),
        scale: 0.0,
        size_cm: 0.0,
        crown: 0xFF,
        enraged: false,
    };

    pub fn text(&self, config: InfoConfig) -> Text<512> {
        let mut text = Text::new();
        let append = |text: &mut Text<512>, value: &str| {
            if !text.as_str().is_empty() && !value.is_empty() {
                let _ = text.write_str(" · ");
            }
            let _ = text.write_str(value);
        };
        if config.show_name {
            append(&mut text, self.name.as_str());
        }
        if config.show_crown {
            append(
                &mut text,
                match self.crown {
                    1 => "大金冠",
                    2 => "大银冠",
                    3 => "小金冠",
                    _ if self.size_cm > 0.0 => "无冠",
                    _ => "冠位未知",
                },
            );
        }
        if config.show_species {
            append(&mut text, self.species.as_str());
        }
        if config.show_enraged {
            append(&mut text, if self.enraged { "发怒" } else { "未发怒" });
        }
        if config.show_size {
            if !text.as_str().is_empty() {
                let _ = text.write_str(" · ");
            }
            if self.size_cm > 0.0 {
                let _ = write!(text, "体型 {:.2} cm", self.size_cm);
            } else {
                let _ = text.write_str("体型未知");
            }
        }
        if config.show_scale {
            if !text.as_str().is_empty() {
                let _ = text.write_str(" · ");
            }
            let _ = write!(text, "×{:.2}", self.scale);
        }
        text
    }
}

pub fn info_origin(
    config: &Config,
    display: DisplayArea,
    size: [f32; 2],
) -> [f32; 2] {
    let scale = display.scale();
    let gap = config.info.line_spacing * scale;
    let x = (config.x + config.info.offset_x) * scale;
    let y = match config.info.position {
        TextPosition::Above => config.y * scale - gap - size[1],
        _ => (config.y + config.height) * scale + gap,
    } + config.info.offset_y * scale;
    // 信息位置只取决于自身配置，不因血量数字的位置而换行。
    [
        x.clamp(0.0, (display.width - size[0]).max(0.0)),
        y.clamp(0.0, (display.height - size[1]).max(0.0)),
    ]
}

// 和原版 0x3A6108 一样，先加 0.001，再乘 100，最后向零截断。
pub fn scale_percent(scale: f32) -> u32 {
    ((scale + 0.001) * 100.0) as u32
}

#[repr(C)]
pub struct SizeCrown {
    pub crown: u8,
    pub padding: [u8; 3],
    pub size_cm: f32,
}
const _: () = assert!(core::mem::size_of::<SizeCrown>() == 8);
const _: () = assert!(core::mem::offset_of!(SizeCrown, size_cm) == 4);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayArea {
    pub width: f32,
    pub height: f32,
}

impl DisplayArea {
    pub fn scale(self) -> f32 {
        self.height / 720.0
    }

    pub fn point(self, x: f32, y: f32) -> [f32; 2] {
        [x * self.width / 1280.0, y * self.height / 720.0]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub health: Health,
    pub display: DisplayArea,
    pub info: MonsterInfo,
}

#[derive(Clone, Copy)]
pub struct State {
    pub config: Config,
    pub snapshot: Option<Snapshot>,
}

impl State {
    pub const INITIAL: Self = Self {
        config: Config::DEFAULT,
        snapshot: None,
    };
}

pub struct Shared<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for Shared<T> {}

impl<T> Shared<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn with<R>(&self, operation: impl FnOnce(&mut T) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let guard = Guard(&self.locked);
        let result = operation(unsafe { &mut *self.value.get() });
        drop(guard);
        result
    }
}

struct Guard<'a>(&'a AtomicBool);
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub static STATE: Shared<State> = Shared::new(State::INITIAL);
pub static GAME_BASE: AtomicU32 = AtomicU32::new(0);
pub static READY: AtomicBool = AtomicBool::new(false);
pub static ENABLED: AtomicBool = AtomicBool::new(true);
pub static SAVED_CONFIG: Shared<Config> = Shared::new(Config::DEFAULT);
pub static SAVE_STATUS: AtomicU32 = AtomicU32::new(0);
pub static FONT_INPUT: Shared<[u8; 168]> = Shared::new([0; 168]);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_rgba_and_legacy_colors_preserve_alpha_on_save() {
        let config = Config::parse("# comment\n[bar]\nfill_color = #12aBcD80 ; comment\nborder_color = #FFFFFF\n[text]\ncolor = 1,2,3,4 # old format\n[info]\ncolor = #00000000").unwrap();
        assert_eq!(config.fill_color, [0x12, 0xAB, 0xCD, 0x80]);
        assert_eq!(config.border_color, [255; 4]);
        assert_eq!(config.text_color, [1, 2, 3, 4]);
        assert_eq!(config.info.color, [0; 4]);
        let mut text = Text::<CONFIG_MAX_BYTES>::new();
        config.write_ini(&mut text).unwrap();
        assert!(text.as_str().contains("fill_color = #12ABCD80"));
        assert_eq!(Config::parse(text.as_str()).unwrap(), config);
        for color in ["#12345", "#1234567", "#123456789", "#12GG5680", "#中文"] {
            assert!(Config::parse(&std::format!("[bar]\nfill_color={color}")).is_err());
        }
    }

    #[test]
    fn shipped_configuration_and_user_changes() {
        assert_eq!(
            Config::parse(include_str!("../conf/target_health_display.ini")).unwrap(),
            Config::DEFAULT
        );
        let cfg = Config::parse("\u{FEFF}[bar]\r\nx = 100 ; comment\r\ny = 600\r\nopacity = 0.5\r\nfill_color = 0,128,255 # comment\r\n[text]\r\nshow_values = false\r\nshow_percent = true\r\nposition = above\r\nfont_file = digits.ttf").unwrap();
        assert_eq!((cfg.x, cfg.y, cfg.opacity), (100.0, 600.0, 0.5));
        assert_eq!(cfg.fill_color, [0, 128, 255, 255]);
        assert_eq!(cfg.text_position, TextPosition::Above);
        assert_eq!(
            cfg.font_file.unwrap().path(),
            c"app:/nativeNX/forge/fonts/digits.ttf"
        );
        assert!(!cfg.show_values && cfg.show_percent);
        assert!(cfg.show_other_areas); // 旧配置未填写此项时也跨区显示。
        assert!(
            !Config::parse("[bar]\nshow_other_areas = false")
                .unwrap()
                .show_other_areas
        );
    }

    #[test]
    fn selection_disappears_when_deselected_or_removed_from_native_target_list() {
        assert!(selected_slot_exists(1, 2));
        assert!(selected_slot_exists(3, 3));
        assert!(!selected_slot_exists(0, 2));
        assert!(!selected_slot_exists(1, 0));
        assert!(!selected_slot_exists(2, 1));
        assert!(!selected_slot_exists(u32::MAX, 3));
    }

    #[test]
    fn invalid_settings_report_the_line() {
        for bad in [
            "[bar]\nwidth = 0",
            "[bar]\nx = NaN",
            "[bar]\nopacity = 1.1",
            "[bar]\nfill_color = 256,0,0",
            "[bar]\nfill_color = 1,2,3,4,5",
            "[bar]\nwidht = 400",
            "[text]\nposition = top",
            "[text]\nshow_values = 1",
            "[text]\nfont_file = ../bad.ttf",
            "[text]\nfont_file = bad\0.ttf",
            "[info]\nshow_enraged = 1",
            "[info]\nposition = inside",
            "[info]\nfont_size = NaN",
            "[info]\nline_spacing = -1",
        ] {
            assert_eq!(Config::parse(bad).unwrap_err().line, 2, "{bad}");
        }
        assert!(Config::parse("[wrong]\nwidth = 400").is_err());
        assert!(Config::parse("enabled = true").is_err());
    }

    #[test]
    fn hp_ratio_clamps_and_color_threshold_uses_unrounded_hp() {
        assert_eq!(Health::new(-5, 100).unwrap().fraction(), 0.0);
        assert_eq!(Health::new(120, 100).unwrap().fraction(), 1.0);
        assert!(Health::new(10, 0).is_none());
        assert!(Health::new(10, -1).is_none());
        let cfg = Config::DEFAULT;
        assert_eq!(
            Health::new(25, 100).unwrap().fill_color(&cfg),
            cfg.low_hp_color
        );
        assert_eq!(
            Health::new(251, 1000).unwrap().fill_color(&cfg),
            cfg.fill_color
        );
        let mut disabled = cfg;
        disabled.low_hp_threshold = 0.0;
        assert_eq!(
            Health::new(0, 100).unwrap().fill_color(&disabled),
            cfg.fill_color
        );
    }

    #[test]
    fn numerical_display_combinations_and_large_hp() {
        let hp = Health::new(7150, 10000).unwrap();
        let mut cfg = Config::DEFAULT;
        cfg.show_percent = false;
        assert_eq!(hp.text(&cfg).as_str(), "7150 / 10000");
        cfg.show_percent = true;
        assert_eq!(hp.text(&cfg).as_str(), "7150 / 10000 (72%)");
        cfg.show_max = false;
        assert_eq!(hp.text(&cfg).as_str(), "7150 (72%)");
        cfg.show_values = false;
        assert_eq!(hp.text(&cfg).as_str(), "72%");
        cfg.show_percent = false;
        assert_eq!(hp.text(&cfg).as_str(), "");
        cfg.show_values = true;
        cfg.show_max = true;
        cfg.show_percent = true;
        assert_eq!(
            Health::new(i32::MAX, i32::MAX).unwrap().text(&cfg).as_str(),
            "2147483647 / 2147483647 (100%)"
        );
    }

    #[test]
    fn handheld_and_docked_coordinates_keep_the_same_relative_position() {
        let handheld = DisplayArea {
            width: 1280.0,
            height: 720.0,
        };
        let docked = DisplayArea {
            width: 1920.0,
            height: 1080.0,
        };
        assert_eq!(handheld.point(760.0, 45.0), [760.0, 45.0]);
        assert_eq!(docked.point(760.0, 45.0), [1140.0, 67.5]);
        assert_eq!(handheld.scale() * Config::DEFAULT.font_size, 17.0);
        assert_eq!(docked.scale() * Config::DEFAULT.font_size, 25.5);
        // 掌机可见范围为 720p，即使 ImGui 后备纹理仍然是 1080p。
        let center = handheld.point(640.0, 360.0);
        assert_eq!(center, [640.0, 360.0]);
    }

    #[test]
    fn concurrent_game_and_render_snapshots_keep_fields_together() {
        let shared = std::sync::Arc::new(Shared::new(State::INITIAL));
        let mut threads = std::vec::Vec::new();
        for writer in 0..4 {
            let shared = shared.clone();
            threads.push(std::thread::spawn(move || {
                for frame in 1..2000 {
                    let current = frame + writer * 2000;
                    shared.with(|state| {
                        state.snapshot = Some(Snapshot {
                            info: MonsterInfo::EMPTY,
                            health: Health::new(current, current * 2).unwrap(),
                            display: DisplayArea {
                                width: current as f32,
                                height: current as f32 * 2.0,
                            },
                        })
                    });
                }
            }));
        }
        for _ in 0..10000 {
            if let Some(snapshot) = shared.with(|state| state.snapshot) {
                assert_eq!(snapshot.health.maximum, snapshot.health.current * 2);
                assert_eq!(snapshot.display.width, snapshot.health.current as f32);
                assert_eq!(snapshot.display.height, snapshot.health.maximum as f32);
            }
        }
        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[test]
    fn horizontal_metadata_keeps_independent_toggles_and_native_crown_codes() {
        let mut info = MonsterInfo {
            name: Text::from_str("Rathalos"),
            species: Text::from_str("Flying Wyvern"),
            scale: 1.25,
            size_cm: 1875.5,
            crown: 1,
            enraged: true,
        };
        let mut cfg = InfoConfig::DEFAULT;
        assert_eq!(info.text(cfg).as_str(), "Rathalos · 大金冠 · 发怒");
        cfg.show_species = true;
        cfg.show_scale = true;
        cfg.show_size = true;
        assert_eq!(
            info.text(cfg).as_str(),
            "Rathalos · 大金冠 · Flying Wyvern · 发怒 · 体型 1875.50 cm · ×1.25"
        );
        info.crown = 2;
        assert!(info.text(cfg).as_str().contains("大银冠"));
        info.crown = 3;
        assert!(info.text(cfg).as_str().contains("小金冠"));
        info.crown = 0xFF;
        assert!(info.text(cfg).as_str().contains("无冠"));
        cfg.show_name = false;
        cfg.show_crown = false;
        cfg.show_enraged = false;
        cfg.show_scale = false;
        assert_eq!(info.text(cfg).as_str(), "Flying Wyvern · 体型 1875.50 cm");
        cfg.show_species = false;
        cfg.show_size = false;
        assert!(!cfg.any());
        assert!(info.text(cfg).as_str().is_empty());
    }

    #[test]
    fn horizontal_info_honors_its_position_independently_of_hp_text() {
        let mut config = Config::DEFAULT;
        let display = DisplayArea {
            width: 1280.0,
            height: 720.0,
        };
        let start = info_origin(&config, display, [600.0, 18.0]);
        assert_eq!(start, [680.0, 75.0]);
        assert_eq!(config.text_offset_x, 150.0);
        assert_eq!(config.text_offset_y, 1.0);
        let high = DisplayArea {
            width: 1920.0,
            height: 1080.0,
        };
        assert_eq!(
            info_origin(&config, high, [900.0, 27.0]),
            [1020.0, 112.5]
        );
        config.text_offset_y = -2.0;
        config.font_size = 40.0;
        assert_eq!(info_origin(&config, display, [600.0, 18.0]), start);
        config.show_values = false;
        config.show_percent = false;
        assert_eq!(info_origin(&config, display, [600.0, 18.0]), start);
        config.info.offset_x = 20.0;
        config.info.offset_y = -5.0;
        assert_eq!(info_origin(&config, display, [200.0, 18.0]), [780.0, 68.0]);
        let mut above = config;
        above.info.position = TextPosition::Above;
        assert_eq!(info_origin(&above, display, [200.0, 18.0]), [780.0, 18.0]);
    }

    #[test]
    fn names_are_owned_utf8_and_truncate_only_at_character_boundaries() {
        assert_eq!(Text::<7>::from_str("雄火龙").as_str(), "雄火");
        assert_eq!(Text::<3>::from_str("😀").as_str(), "");
        assert_eq!(Text::<5>::from_str("😀X").as_str(), "😀X");
        let mut value = std::string::String::from("雄火龙");
        let stored = Text::<32>::from_str(&value);
        value.clear();
        assert_eq!(stored.as_str(), "雄火龙");
    }

    #[test]
    fn info_settings_are_independent_and_legacy_ini_gets_metadata_defaults() {
        let legacy = Config::parse("[bar]\nx = 120\n[text]\nshow_percent = false").unwrap();
        assert_eq!(legacy.x, 120.0);
        assert!(!legacy.show_percent);
        assert_eq!(legacy.info, InfoConfig::DEFAULT);
        let cfg=Config::parse("[info]\nshow_name = false\nshow_crown = false\nshow_species = false\nshow_enraged = false\nshow_scale = false\nshow_size = false\nfont_size = 28\nposition = above\noffset_x = -40\noffset_y = 80\nline_spacing = 7\ncolor = 10,20,30,40").unwrap();
        assert!(!cfg.info.any());
        assert_eq!(cfg.info.font_size, 28.0);
        assert_eq!(cfg.info.position, TextPosition::Above);
        assert_eq!(
            (cfg.info.offset_x, cfg.info.offset_y, cfg.info.line_spacing),
            (-40.0, 80.0, 7.0)
        );
        assert_eq!(cfg.info.color, [10, 20, 30, 40]);
    }

    #[test]
    fn size_uses_native_percent_truncation_and_marks_unsupported_measurements() {
        assert_eq!(scale_percent(1.0), 100);
        assert_eq!(scale_percent(1.235), 123);
        assert_eq!(scale_percent(1.2495), 125);
        assert_eq!(scale_percent(0.9995), 100);
        let info = MonsterInfo::EMPTY;
        assert_eq!(
            info.text(InfoConfig {
                show_size: true,
                show_scale: true,
                ..InfoConfig::DEFAULT
            }).as_str(),
            "冠位未知 · 未发怒 · 体型未知 · ×0.00"
        );
    }

    #[test]
    fn saved_ini_roundtrips_all_fields_and_utf8_font_names() {
        let custom = Config::parse("[bar]\nenabled=false\nshow_other_areas=false\nx=-10.25\ny=120.5\nwidth=750\nheight=50\nrounding=9\nborder_width=4\nopacity=0.65\nfill_color=1,2,3,4\nbackground_color=5,6,7,8\nborder_color=9,10,11,12\nlow_hp_threshold=35\nlow_hp_color=13,14,15,16\n[text]\nshow_values=false\nshow_max=false\nshow_percent=false\nfont_size=38\nfont_file=思源黑体.ttf\ncolor=17,18,19,20\noutline_color=21,22,23,24\noutline_width=3\nposition=above\noffset_x=-40\noffset_y=60\n[info]\nshow_name=false\nshow_crown=false\nshow_species=false\nshow_enraged=false\nshow_scale=false\nshow_size=false\nfont_size=30\nposition=above\noffset_x=16\noffset_y=-18\nline_spacing=8\ncolor=25,26,27,28").unwrap();
        for config in [Config::DEFAULT, custom] {
            let mut text = Text::<CONFIG_MAX_BYTES>::new();
            config.write_ini(&mut text).unwrap();
            assert_eq!(Config::parse(text.as_str()).unwrap(), config);
            assert!(
                text.as_str()
                    .contains("MHGU save: /target_health_display.ini")
            );
        }
        assert_eq!(custom.font_file.unwrap().filename(), "思源黑体.ttf");
        let mut tiny = Text::<16>::new();
        assert!(custom.write_ini(&mut tiny).is_err());
        for filename in [
            "foo;bar.ttf",
            "foo#bar.ttf",
            "../font.ttf",
            "font.ttf\n",
            "default",
        ] {
            assert!(FontFile::new(filename).is_none(), "{filename}");
        }
    }
}
