use tempfile::tempdir;

use sysclean::i18n::{Language, load_language_from_file};

#[test]
fn language_parses_supported_values_and_falls_back_to_english() {
    assert_eq!(Language::from_config_value("en"), Language::En);
    assert_eq!(Language::from_config_value("zh-CN"), Language::ZhCn);
    assert_eq!(Language::from_config_value(""), Language::En);
    assert_eq!(Language::from_config_value("fr"), Language::En);
}

#[test]
fn load_language_from_file_reads_ini_and_defaults_to_english() {
    let temp = tempdir().expect("temp dir");
    let zh_file = temp.path().join("sysclean.ini");
    std::fs::write(&zh_file, "[ui]\nlanguage=zh-CN\n").expect("write ini");
    assert_eq!(
        load_language_from_file(&zh_file).expect("load ini"),
        Language::ZhCn
    );

    let invalid_file = temp.path().join("invalid.ini");
    std::fs::write(&invalid_file, "[ui]\nlanguage=fr\n").expect("write invalid ini");
    assert_eq!(
        load_language_from_file(&invalid_file).expect("load invalid ini"),
        Language::En
    );

    let missing = temp.path().join("missing.ini");
    assert_eq!(
        load_language_from_file(&missing).expect("missing falls back"),
        Language::En
    );
}

#[test]
fn language_exposes_distinct_help_copy() {
    assert_eq!(Language::En.help_hint(), "Press ? for help");
    assert_eq!(Language::ZhCn.help_hint(), "按 ? 查看帮助");
}
