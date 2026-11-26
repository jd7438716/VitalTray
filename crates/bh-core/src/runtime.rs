//! 运行时：定时收轮询结果、更新托盘、低电提醒。

use crate::i18n;
use crate::log;
use crate::state::{self, State};

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
        .map(|r| match &r.error {
            Some(e) => format!("{}:{e}", r.domain),
            None => format!("{}:{}%", r.domain, r.soc),
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
                let body = format!("{}{}", a.name, i18n::t("low.body").replace("{}", &a.soc.to_string()));
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

/// 把当前读数写成托盘提示（固定标题）与托盘图标。
pub fn refresh() {
    let (tip, worst) = state::with(|st| {
        let Some(st) = st else {
            return (String::new(), None);
        };
        let worst = st
            .readings
            .iter()
            .filter(|r| r.error.is_none())
            .min_by_key(|r| r.soc)
            .map(|r| (r.soc, r.charging));
        (tip_text(), worst)
    });

    if tip.is_empty() {
        return;
    }
    state::with_mut(|st| {
        let Some(st) = st else { return };
        if let Some(t) = st.tray.as_mut() {
            t.set_tip(&tip);
            if let Some((soc, charging)) = worst {
                let style = crate::tray::icons::Style::parse(&st.cfg.icon_style);
                t.set_icon(crate::tray::icons::render(soc, charging, style));
            }
        }
    });
    log::debug("ui", &format!("refresh: {tip}"));
}
