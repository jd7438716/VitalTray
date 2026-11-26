//! 运行时：定时收轮询结果、更新托盘、低电提醒。

use crate::i18n;
use crate::log;
use crate::state::{self, State};
use crate::status;

pub fn on_timer() {
    let mut latest = None;
    state::with(|st| {
        if let Some(st) = st {
            for msg in st.rx.try_iter() {
                latest = Some(msg);
            }
        }
    });
    let Some(readings) = latest else { return };

    let line: Vec<String> = readings
        .iter()
        .map(|r| {
            if r.pending {
                format!("{}:{}", r.domain, i18n::t("panel.waiting"))
            } else if let Some(e) = &r.error {
                format!("{}:{e}", r.domain)
            } else {
                format!("{}:{}%", r.domain, r.soc)
            }
        })
        .collect();
    log::info("poll", &format!("-> [{}]", line.join(", ")));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let alerts = state::with_mut(|st| {
        let st: &mut State = st?;
        st.readings = readings;
        Some(st.monitor.observe(&st.readings, st.low_battery, now))
    })
    .unwrap_or_default();

    for a in alerts {
        log::warn("battery", &format!("{} {}%", i18n::t("low.log"), a.soc));
        state::with_mut(|st| {
            if let Some(t) = st.and_then(|s| s.tray.as_mut()) {
                let body = i18n::tf("low.body", &[("name", &a.name), ("soc", &a.soc.to_string())]);
                t.balloon(i18n::t("low.title"), &body);
            }
        });
    }
    refresh();
}

/// 托盘悬停提示：只显示标题，电量与续航都进面板，提示里不塞数字。
fn tip_text() -> String {
    i18n::t("tray.title").to_string()
}

/// 把当前读数汇总成“一个状态 + 一个电量”，写成托盘提示（固定标题）与托盘图标。
///
/// 颜色口径见 `status`：低电红 > 全部充足绿 > 充电蓝 > 正常中性；
/// 没有新鲜读数时才显示灰（上次 / 无数据）。
pub fn refresh() {
    let (tip, agg) = state::with(|st| {
        let Some(st) = st else {
            return (String::new(), status::Aggregate { state: status::State::Unknown, percent: None });
        };
        (tip_text(), status::aggregate(&st.readings, st.cfg.low_battery))
    });

    if tip.is_empty() {
        return;
    }
    state::with_mut(|st| {
        let Some(st) = st else { return };
        if let Some(t) = st.tray.as_mut() {
            t.set_tip(&tip);
            let style = crate::tray::icons::Style::parse(&st.cfg.icon_style);
            t.set_icon(crate::tray::icons::render(&agg, style));
        }
    });
    log::debug("ui", &format!("refresh: {tip} [{} {:?}]", agg.state.name(), agg.percent));
}
