//! 面板配色：跟随系统应用明暗主题；数值色按语义（绿/琥珀/红/蓝/灰）由 model 决定。
//!
//! 电量色与托盘图标同源（`crate::status`），避免“托盘绿、面板黄”的口径分裂；
//! 系统温度仍用面板自己的琥珀档（那是另一套语义）。

use super::gdi::rgb;
use crate::status::{self, State};

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
    /// 充电中（电量 20–80%）。
    pub accent: u32,
    /// 上次读数 / 读不到。
    pub stale: u32,
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
            green: status::state_color(State::Good, true),
            amber: rgb(242, 178, 64),
            red: status::state_color(State::Low, true),
            accent: status::state_color(State::Charging, true),
            stale: status::state_color(State::Stale, true),
        }
    } else {
        Palette {
            bg: rgb(250, 250, 252),
            border: rgb(214, 217, 222),
            text: rgb(24, 24, 28),
            muted: rgb(92, 97, 104),
            subdued: rgb(132, 137, 144),
            separator: rgb(226, 228, 233),
            green: status::state_color(State::Good, false),
            amber: rgb(200, 130, 20),
            red: status::state_color(State::Low, false),
            accent: status::state_color(State::Charging, false),
            stale: status::state_color(State::Stale, false),
        }
    }
}
