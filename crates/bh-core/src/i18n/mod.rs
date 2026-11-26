//! 多语言：语言表、系统语言检测、查表。
//!
//! * 两层表：`ui/` 是界面文案（面板 / 菜单 / 通知 / 关于 / 配置注释），`cli/` 是命令行输出；
//! * 一个语言一个文件，文件里就是一张 `(key, text)` 静态表 —— 加语言 = 加两个文件 + 这里一行；
//! * 缺键回落英文，英文也没有就返回空串（`tests` 会拦住漏键）；
//! * `lang=auto`（默认）跟随 Windows 的界面语言（`GetUserDefaultUILanguage`）。
//!
//! 文案里用 `{name}` 这类占位符，配 [`tf`] 替换；不要在代码里拼句子，语序各语言不同。

use std::sync::atomic::{AtomicU8, Ordering};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

mod cli;
mod ui;

/// 支持的语言。`En` 是兜底（也是缺键时的回落）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Lang {
    En = 0,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
    De,
    Fr,
    Es,
    Ru,
    PtBr,
}

impl Lang {
    /// 菜单里的顺序（英文在前，其余按语言自身习惯排）。
    pub const ALL: [Lang; 10] = [
        Lang::En,
        Lang::ZhCn,
        Lang::ZhTw,
        Lang::Ja,
        Lang::Ko,
        Lang::De,
        Lang::Fr,
        Lang::Es,
        Lang::Ru,
        Lang::PtBr,
    ];

    /// 写进 config.ini 的标签（BCP-47 风格，读回时大小写/下划线都认）。
    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhCn => "zh-CN",
            Lang::ZhTw => "zh-TW",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::Ru => "ru",
            Lang::PtBr => "pt-BR",
        }
    }

    /// 语言自己的名字（菜单里直接显示，不翻译）。
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::ZhCn => "简体中文",
            Lang::ZhTw => "繁體中文",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Es => "Español",
            Lang::Ru => "Русский",
            Lang::PtBr => "Português (Brasil)",
        }
    }

    pub fn ui(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Lang::En => ui::en::TABLE,
            Lang::ZhCn => ui::zh_cn::TABLE,
            Lang::ZhTw => ui::zh_tw::TABLE,
            Lang::Ja => ui::ja::TABLE,
            Lang::Ko => ui::ko::TABLE,
            Lang::De => ui::de::TABLE,
            Lang::Fr => ui::fr::TABLE,
            Lang::Es => ui::es::TABLE,
            Lang::Ru => ui::ru::TABLE,
            Lang::PtBr => ui::pt_br::TABLE,
        }
    }

    pub fn cli(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Lang::En => cli::en::TABLE,
            Lang::ZhCn => cli::zh_cn::TABLE,
            Lang::ZhTw => cli::zh_tw::TABLE,
            Lang::Ja => cli::ja::TABLE,
            Lang::Ko => cli::ko::TABLE,
            Lang::De => cli::de::TABLE,
            Lang::Fr => cli::fr::TABLE,
            Lang::Es => cli::es::TABLE,
            Lang::Ru => cli::ru::TABLE,
            Lang::PtBr => cli::pt_br::TABLE,
        }
    }

    /// 解析用户写的标签：`zh-CN` / `zh_CN` / `zh-Hans` / `pt` / `en-US` 都认。
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let t = tag.trim().to_ascii_lowercase().replace('_', "-");
        let primary = t.split('-').next().unwrap_or("");
        Some(match primary {
            "en" => Lang::En,
            "zh" => {
                let traditional = ["hant", "tw", "hk", "mo"].iter().any(|s| t.contains(s));
                if traditional {
                    Lang::ZhTw
                } else {
                    Lang::ZhCn
                }
            }
            "ja" => Lang::Ja,
            "ko" => Lang::Ko,
            "de" => Lang::De,
            "fr" => Lang::Fr,
            "es" => Lang::Es,
            "ru" => Lang::Ru,
            "pt" => Lang::PtBr,
            _ => return None,
        })
    }

    /// Windows LANGID → 支持的语言（认不出的一律英文）。
    ///
    /// LANGID = 主语言（低 10 位）| 子语言（高 6 位）。中文要看子语言：
    /// 1/3/5 = 繁體（TW/HK/MO），2/4 = 简体（CN/SG）。
    pub fn from_langid(langid: u16) -> Lang {
        let primary = langid & 0x3FF;
        let sub = langid >> 10;
        match primary {
            0x04 => {
                if matches!(sub, 1 | 3 | 5) {
                    Lang::ZhTw
                } else {
                    Lang::ZhCn
                }
            }
            0x09 => Lang::En,
            0x11 => Lang::Ja,
            0x12 => Lang::Ko,
            0x07 => Lang::De,
            0x0C => Lang::Fr,
            0x0A => Lang::Es,
            0x19 => Lang::Ru,
            0x16 => Lang::PtBr,
            _ => Lang::En,
        }
    }
}

fn from_code(v: u8) -> Lang {
    match v {
        1 => Lang::ZhCn,
        2 => Lang::ZhTw,
        3 => Lang::Ja,
        4 => Lang::Ko,
        5 => Lang::De,
        6 => Lang::Fr,
        7 => Lang::Es,
        8 => Lang::Ru,
        9 => Lang::PtBr,
        _ => Lang::En,
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

/// 当前生效的语言。
pub fn current() -> Lang {
    from_code(CURRENT.load(Ordering::Relaxed))
}

/// 按配置值切语言：`auto` / 空 = 跟随系统，其它按标签解析（认不出也跟系统）。
pub fn init(setting: &str) {
    set(resolve(setting));
}

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn resolve(setting: &str) -> Lang {
    if is_auto(setting) {
        system()
    } else {
        Lang::from_tag(setting.trim()).unwrap_or_else(system)
    }
}

/// 配置值是不是“跟随系统”（空或 `auto`）。
pub fn is_auto(setting: &str) -> bool {
    let s = setting.trim();
    s.is_empty() || s.eq_ignore_ascii_case("auto")
}

/// Windows 的界面语言（用户首选显示语言）。取不到就英文。
pub fn system() -> Lang {
    let langid = unsafe { GetUserDefaultUILanguage() };
    if langid == 0 {
        Lang::En
    } else {
        Lang::from_langid(langid)
    }
}

fn lookup(lang: Lang, key: &str) -> &'static str {
    for table in [lang.ui(), lang.cli(), Lang::En.ui(), Lang::En.cli()] {
        if let Some((_, value)) = table.iter().find(|(k, _)| *k == key) {
            return value;
        }
    }
    ""
}

/// 查一条文案（缺失时回落英文，再缺就是空串）。
pub fn t(key: &str) -> &'static str {
    lookup(current(), key)
}

/// 查一条带占位符的文案：`tf("low.body", &[("name", "MX Keys"), ("soc", "15")])`。
pub fn tf(key: &str, args: &[(&str, &str)]) -> String {
    let mut out = t(key).to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// 设备类别短名（面板与提示共用）。
pub fn device_label(glyph: bh_api::Glyph) -> &'static str {
    t(match glyph {
        bh_api::Glyph::Mouse => "glyph.mouse",
        bh_api::Glyph::Keyboard => "glyph.keyboard",
        bh_api::Glyph::Headset => "glyph.headset",
        bh_api::Glyph::Gamepad => "glyph.gamepad",
        bh_api::Glyph::Laptop => "glyph.laptop",
        bh_api::Glyph::Phone => "glyph.phone",
        bh_api::Glyph::Unknown => "glyph.bluetooth",
    })
}

pub fn icon_style_label(style: &str) -> &'static str {
    match style {
        "number" => t("icon.style.number"),
        "ring" => t("icon.style.ring"),
        _ => t("icon.style.battery"),
    }
}

/// 面板 / 托盘字形用的字体：按语言挑一套带该文种字形的系统字体。
///
/// Windows 10/11 自带这些字体；挑错了只会变成回退字体，不会画不出来。
pub fn ui_font() -> &'static str {
    match current() {
        Lang::ZhCn | Lang::ZhTw => "Microsoft YaHei UI",
        Lang::Ja => "Yu Gothic UI",
        Lang::Ko => "Malgun Gothic",
        _ => "Segoe UI",
    }
}

#[cfg(test)]
mod tests;

