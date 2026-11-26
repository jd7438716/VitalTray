//! i18n 的回归测试：语言表完整性、标签/LANGID 解析、占位符替换。
//!
//! 单独放一个文件：`mod.rs` 只留实现，行数门禁才不会把测试算进去。

use super::*;
use std::collections::HashSet;

#[test]
fn every_language_has_every_key_once() {
    // 英文表是基准：UI / CLI 两张表分开比，别的语言不能多也不能少。
    let ui_keys: Vec<&str> = Lang::En.ui().iter().map(|(k, _)| *k).collect();
    let cli_keys: Vec<&str> = Lang::En.cli().iter().map(|(k, _)| *k).collect();
    for (name, keys) in [("ui", &ui_keys), ("cli", &cli_keys)] {
        let unique: HashSet<&str> = keys.iter().copied().collect();
        assert_eq!(unique.len(), keys.len(), "英文 {name} 表里有重复键");
        for key in keys {
            assert!(!lookup(Lang::En, key).is_empty(), "英文 {name} 表里 {key} 是空的");
        }
    }

    for lang in Lang::ALL {
        for (name, table, keys) in [("ui", lang.ui(), &ui_keys), ("cli", lang.cli(), &cli_keys)] {
            let mut seen = HashSet::new();
            for (k, v) in table {
                assert!(!v.is_empty(), "{lang:?} 的 {name}/{k} 是空的");
                assert!(seen.insert(*k), "{lang:?} 的 {name} 表里 {k} 重复");
            }
            for k in keys.iter() {
                assert!(
                    table.iter().any(|(key, _)| key == k),
                    "{lang:?} 的 {name} 表缺键 {k}（与英文表对不上）"
                );
            }
        }
    }
}

#[test]
fn ui_and_cli_tables_do_not_share_keys() {
    let ui: HashSet<&str> = Lang::En.ui().iter().map(|(k, _)| *k).collect();
    for (k, _) in Lang::En.cli() {
        assert!(!ui.contains(k), "键 {k} 同时出现在 ui 和 cli 表里");
    }
}

#[test]
fn tags_roundtrip_and_accept_aliases() {
    for lang in Lang::ALL {
        assert_eq!(Lang::from_tag(lang.tag()), Some(lang));
    }
    assert_eq!(Lang::from_tag("zh_CN"), Some(Lang::ZhCn));
    assert_eq!(Lang::from_tag("zh-Hans"), Some(Lang::ZhCn));
    assert_eq!(Lang::from_tag("zh-HK"), Some(Lang::ZhTw));
    assert_eq!(Lang::from_tag("zh-Hant"), Some(Lang::ZhTw));
    assert_eq!(Lang::from_tag("en-US"), Some(Lang::En));
    assert_eq!(Lang::from_tag("pt"), Some(Lang::PtBr));
    assert_eq!(Lang::from_tag("PT-br"), Some(Lang::PtBr));
    assert_eq!(Lang::from_tag("klingon"), None);
}

#[test]
fn langid_maps_major_windows_languages() {
    // MAKELANGID = 主语言 | 子语言 << 10
    assert_eq!(Lang::from_langid(0x0804), Lang::ZhCn); // zh-CN
    assert_eq!(Lang::from_langid(0x0404), Lang::ZhTw); // zh-TW
    assert_eq!(Lang::from_langid(0x0C04), Lang::ZhTw); // zh-HK
    assert_eq!(Lang::from_langid(0x1004), Lang::ZhCn); // zh-SG
    assert_eq!(Lang::from_langid(0x0409), Lang::En); // en-US
    assert_eq!(Lang::from_langid(0x0411), Lang::Ja);
    assert_eq!(Lang::from_langid(0x0412), Lang::Ko);
    assert_eq!(Lang::from_langid(0x0407), Lang::De);
    assert_eq!(Lang::from_langid(0x040C), Lang::Fr);
    assert_eq!(Lang::from_langid(0x0C0A), Lang::Es); // es-ES, 现代排序
    assert_eq!(Lang::from_langid(0x0419), Lang::Ru);
    assert_eq!(Lang::from_langid(0x0416), Lang::PtBr); // pt-BR
    assert_eq!(Lang::from_langid(0x0816), Lang::PtBr); // pt-PT 暂无，回落 pt-BR
    assert_eq!(Lang::from_langid(0x0405), Lang::En); // cs-CZ 未支持 → 英文
    assert_eq!(Lang::from_langid(0), Lang::En);
}

#[test]
fn auto_and_garbage_follow_the_system() {
    assert_eq!(resolve("auto"), system());
    assert_eq!(resolve("  "), system());
    assert_eq!(resolve("xx-YY"), system());
    assert_eq!(resolve("de"), Lang::De);
}

#[test]
fn placeholders_are_substituted() {
    set(Lang::En);
    assert_eq!(
        tf("low.body", &[("name", "MX Keys"), ("soc", "15")]),
        "MX Keys is at 15% — time to charge"
    );
    // 没替换的占位符原样留着，便于发现漏传参数
    assert!(tf("low.body", &[("name", "MX")]).contains("{soc}"));
    set(Lang::En);
}

#[test]
fn every_placeholder_used_by_transform_contract_exists() {
    // 代码里会替换的占位符必须真的在文案里，否则翻译时容易写坏
    let checks = [
        ("low.body", "name"),
        ("low.body", "soc"),
        ("panel.disk_detail", "free"),
        ("panel.disk_detail", "total"),
        ("panel.updated", "time"),
        ("about.body", "version"),
        ("about.body", "plugins_list"),
        ("about.body", "config_path"),
        ("about.body", "log_path"),
        ("est.hours", "n"),
    ];
    for lang in Lang::ALL {
        for (key, placeholder) in checks {
            let loaded = lang
                .ui()
                .iter()
                .chain(lang.cli())
                .find(|(k, _)| *k == key)
                .map(|(_, v)| *v)
                .unwrap_or("");
            assert!(
                loaded.contains(&format!("{{{placeholder}}}")),
                "{lang:?} 的 {key} 里没有占位符 {{{placeholder}}}"
            );
        }
    }
}

#[test]
fn missing_key_falls_back_to_empty() {
    assert_eq!(t("no.such.key"), "");
}
