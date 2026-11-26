//! 面板配色：跟随系统明暗主题，数值颜色按“等级”语义（绿/琥珀/红）由 model 决定。

use super::gdi::rgb;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub subdued: u32,
    pub separator: u32,
    pub green: u32,
    pub amber: u32,
    pub red: u32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: rgb(32, 34, 38),
            border: rgb(58, 61, 66),
            text: rgb(242, 244, 247),
            muted: rgb(160, 166, 173),
            subdued: rgb(120, 126, 134),
            separator: rgb(56, 59, 65),
            green: rgb(96, 200, 112),
            amber: rgb(242, 178, 64),
            red: rgb(238, 92, 86),
        }
    } else {
        Palette {
            bg: rgb(250, 250, 252),
            border: rgb(214, 217, 222),
            text: rgb(24, 24, 28),
            muted: rgb(92, 97, 104),
            subdued: rgb(132, 137, 144),
            separator: rgb(226, 228, 233),
            green: rgb(36, 160, 72),
            amber: rgb(200, 130, 20),
            red: rgb(208, 58, 52),
        }
    }
}
