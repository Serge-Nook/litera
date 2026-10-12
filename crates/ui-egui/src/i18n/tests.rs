//! Interface language tests (#8).

use super::catalog::{Catalog, parse_entries, placeholders};
use super::*;

fn lang(code: &str) -> Lang {
    Lang::from_code(code).unwrap_or_else(|| panic!("{code} registered"))
}

#[test]
fn placeholders_fill_without_reinterpreting_values() {
    let args = [("path", "a-{pages}.docx"), ("pages", "2")];
    assert_eq!(fmt("{pages}: {path}; {unknown}", &args), "2: a-{pages}.docx; {unknown}");
    assert_eq!(fmt("text {unfinished", &args), "text {unfinished");
    assert_eq!(fmt(tr(lang("zh-hans"), "Page {page} of {pages}"), &[("page", "3"), ("pages", "9")]), "第 3 页，共 9 页");
}

#[test]
fn malformed_catalog_lines_are_skipped_not_fatal() {
    let (c, errors) =
        Catalog::parse("# c\n\tHello\t你好\nctx\tA\tB\n\tOpen…\t打开\n\tPage {n}\t第 {m} 页\nno tabs\n\tHello\t重复\n\tLine\\nTwo\t一\\n二\n");
    assert_eq!(errors.len(), 5, "{errors:?}");
    assert_eq!(c.plain("Hello"), Some("你好"), "the first entry wins");
    assert_eq!(c.plain("Line\nTwo"), Some("一\n二"));
    assert_eq!(c.plain("Open…"), None);
}

/// Every bundled catalog parses without errors, and its entries keep the English placeholders.
#[test]
fn bundled_catalogs_are_well_formed() {
    let mut codes = std::collections::HashSet::new();
    for l in &LANGUAGES {
        assert!(codes.insert(l.code), "duplicate code {}", l.code);
        assert_eq!(l.code, l.code.to_ascii_lowercase());
        let (entries, errors) = parse_entries(l.source);
        assert!(errors.is_empty(), "{}: {errors:?}", l.code);
        for e in &entries {
            assert_eq!(placeholders(&e.source), placeholders(&e.translation), "{}: {:?}", l.code, e.source);
            assert_eq!(e.translation.trim(), e.translation, "{}: stray whitespace in {:?}", l.code, e.translation);
            assert!(!e.translation.contains("..."), "{}: use … rather than three dots: {:?}", l.code, e.translation);
        }
        assert!(l.code == "en" || entries.len() > 700, "{} has {} entries", l.code, entries.len());
    }
}

/// The ribbon tabs and every registered command's label and ribbon location are translated, in
/// every language, so menus, tooltips and command search never fall back to English.
#[test]
fn every_tab_and_command_is_translated() {
    let session = wordcraft_engine::Session::new(wordcraft_doc::Document::new());
    // Labels that read the same in every language.
    let universal = |s: &str| !s.chars().any(char::is_alphabetic);
    let mut missing = Vec::new();
    for l in Lang::all().filter(|l| *l != Lang::EN) {
        let tabs = crate::ribbon::TABS.iter().chain(&["Table Design", "Table Layout"]).copied();
        let commands = session.registry.all().iter().flat_map(|spec| std::iter::once(spec.label).chain(spec.location.split(" › ")));
        for s in tabs.chain(commands).filter(|s| !s.is_empty() && !universal(s)) {
            if !has(l, s) {
                missing.push(format!("{}: {s:?}", l.code()));
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "untranslated: {missing:#?}");
}

#[test]
fn chinese_interface_fonts_load_and_cover_simplified_hanzi() {
    // #241: without an embedded Chinese face the interface adds an installed CJK font; egui must
    // accept it (it panics on font data it can't parse) and draw simplified-only hanzi with it.
    let ctx = egui::Context::default();
    ctx.set_fonts(crate::theme::font_definitions(true, true));
    ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
    let covered = ctx.fonts_mut(|f| f.has_glyphs(&egui::FontId::proportional(14.0), "删除页选"));
    let (catalogs, _) = Lang::cjk_font_text(true);
    let embedded = !wordcraft_fonts::ui_needs_system_cjk(&catalogs, &wordcraft_fonts::ui_cjk_fonts(true));
    if embedded || wordcraft_fonts::system_cjk_ui_font(true, &['页']).is_some() {
        assert!(covered, "a Chinese face is available but the interface lacks simplified hanzi");
    }
}

#[test]
fn russian_locales_and_saved_preference_work_without_changing_the_document() {
    let ru = lang("ru");
    for tag in ["ru", "ru-RU", "ru_RU.UTF-8", "RU-ru", "ru_UA.UTF-8", "ru-Cyrl-RU"] {
        assert_eq!(lang_from_tag(tag), Some(ru), "{tag}");
    }
    assert_eq!(first_supported(["xx-XX", "ru-RU", "en-US"]), Some(ru));
    let mut app = crate::WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Default::default());
    let before = serde_json::to_value(&app.session.doc).unwrap();
    let result = app.run("ui.language", serde_json::json!({"value": "RU"})).unwrap();
    assert_eq!(result["effective"], "ru");
    assert_eq!(app.ui.language, "ru");
    assert_eq!(serde_json::to_value(&app.session.doc).unwrap(), before);
    let saved = serde_json::to_string(&app.ui).unwrap();
    let restored: crate::UiState = serde_json::from_str(&saved).unwrap();
    assert_eq!(Lang::from_pref(&restored.language), ru);
    assert_eq!(ru.name(), "Русский");
}

#[test]
fn russian_covers_the_entire_existing_interface_catalog() {
    use std::collections::HashSet;
    let keys = |source| parse_entries(source).0.into_iter().map(|e| e.source).collect::<HashSet<_>>();
    // Everything the other catalogs translate, plus the preview samples below.
    assert!(keys(lang("ru").0.source).is_superset(&keys(lang("zh-hans").0.source)));
    assert!(parse_entries(lang("ru").0.source).1.is_empty());
    let ru = lang("ru");
    // Style gallery and Design tab previews show Cyrillic sample text.
    assert_eq!(tr(ru, "AaBbCcDd"), "АаБбВвГг");
    assert_eq!(tr(ru, "Title"), "Заголовок");
    assert_eq!(tr(ru, "Heading 1"), "Заголовок 1");
    assert!(tr(ru, "Body text in a short paragraph to show spacing.").starts_with("Основной текст"));
    assert_eq!(tr(ru, "Home"), "Главная");
    assert_eq!(tr(ru, "Font"), "Шрифт");
    assert_eq!(tr(ru, "Review"), "Рецензирование");
    assert_eq!(tr(ru, "Save"), "Сохранить");
    assert_eq!(tr(ru, "unknown future label"), "unknown future label");
    // The search hint's example must find the command it names.
    assert_eq!(tr(ru, "Insert Table"), "Вставить таблицу");
    assert!(tr(ru, "Type a command, e.g. \"insert table\"").contains("вставить таблицу"));
    // Count-neutral wording is grammatical for every count: `Слов: 21`, `Слов: 1 из 21`.
    for count in [0, 1, 2, 5, 21, 100, 1000] {
        let words = count.to_string();
        assert_eq!(fmt(tr(ru, "{words} words"), &[("words", &words)]), format!("Слов: {count}"));
        assert_eq!(fmt(tr(ru, "{selected} of {words} words"), &[("selected", "1"), ("words", &words)]), format!("Слов: 1 из {count}"));
    }
}

#[test]
fn bundled_interface_fonts_cover_russian_without_system_fallbacks() {
    let db = wordcraft_fonts::FontDb::with_font_dirs(Vec::new());
    let (entries, errors) = parse_entries(lang("ru").0.source);
    assert!(errors.is_empty());
    let chars: std::collections::HashSet<char> = entries
        .iter()
        .flat_map(|e| e.translation.chars())
        // Cyrillic letters plus the Russian quotes and dash; symbols such as ¶, ⌘ or 📂 come from
        // the English sources and render as they already do.
        .filter(|c| ('\u{0400}'..='\u{04FF}').contains(c) || matches!(c, '«' | '»' | '—'))
        .chain("АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюя".chars())
        .collect();
    for (family, style) in [("Inter", "Regular"), ("Inter", "Medium"), ("Inter", "SemiBold"), ("JetBrains Mono", "Regular")] {
        let face = db.face(family, style);
        assert_eq!(face.family, family, "must use the bundled interface face");
        for ch in &chars {
            assert_ne!(face.glyph_for(*ch), 0, "{family} {style} lacks {ch}");
        }
    }
}

#[test]
fn built_in_style_names_are_translated_for_display_and_user_styles_are_not() {
    let sheet = wordcraft_doc::StyleSheet::builtin();
    let name = |id: &str| style_name(sheet.get(id).unwrap());
    set_current(lang("ru"));
    assert_eq!(name("Normal"), "Обычный");
    assert_eq!(name("Title"), "Заголовок");
    assert_eq!(name("Heading1"), "Заголовок 1");
    // Numbered families beyond the catalog's explicit entries go through one template.
    assert_eq!(name("Heading7"), "Заголовок 7");
    assert_eq!(name("TOC9"), "Оглавление 9");
    // A style the author made keeps its name, even when it matches a built-in one.
    let mine = wordcraft_doc::Style { name: "Normal".into(), builtin: false, ..Default::default() };
    assert_eq!(style_name(&mine), "Normal");
    // A built-in name without a translation stays English.
    assert_eq!(builtin_style_name("Some Future Style"), "Some Future Style");
    assert_eq!(builtin_style_name("Heading x"), "Heading x");
    // Every built-in paragraph and character style shows in Russian. Table styles (`Grid Table 4 –
    // Blue`) appear only as tooltips under Table Design and stay English for now.
    let untranslated: Vec<&str> = sheet
        .styles
        .iter()
        .filter(|s| s.kind != wordcraft_doc::StyleKind::Table && style_name(s) == s.name && s.name.chars().any(char::is_alphabetic))
        .map(|s| s.name.as_str())
        .collect();
    assert!(untranslated.is_empty(), "built-in styles without a Russian name: {untranslated:?}");
    set_current(Lang::EN);
    assert_eq!(name("Heading7"), "Heading 7");
    // The document itself never changes.
    assert_eq!(sheet.get("Normal").unwrap().name, "Normal");
}

#[test]
fn bundled_interface_fonts_cover_estonian_letters() {
    let db = wordcraft_fonts::FontDb::with_font_dirs(Vec::new());
    for (family, style) in [("Inter", "Regular"), ("Inter", "Medium"), ("Inter", "SemiBold"), ("JetBrains Mono", "Regular")] {
        let face = db.face(family, style);
        assert_eq!(face.family, family, "must use the bundled interface face");
        for ch in "ÕõÄäÖöÜüŠšŽž".chars() {
            assert_ne!(face.glyph_for(ch), 0, "{family} {style} lacks {ch}");
        }
    }
}

#[test]
fn bundled_interface_fonts_cover_german_letters() {
    let db = wordcraft_fonts::FontDb::with_font_dirs(Vec::new());
    for (family, style) in [("Inter", "Regular"), ("Inter", "Medium"), ("Inter", "SemiBold"), ("JetBrains Mono", "Regular")] {
        let face = db.face(family, style);
        assert_eq!(face.family, family, "must use the bundled interface face");
        for ch in "ÄäÖöÜüß".chars() {
            assert_ne!(face.glyph_for(ch), 0, "{family} {style} lacks {ch}");
        }
    }
}

#[test]
fn bundled_interface_fonts_cover_italian_letters() {
    let db = wordcraft_fonts::FontDb::with_font_dirs(Vec::new());
    for (family, style) in [("Inter", "Regular"), ("Inter", "Medium"), ("Inter", "SemiBold"), ("JetBrains Mono", "Regular")] {
        let face = db.face(family, style);
        assert_eq!(face.family, family, "must use the bundled interface face");
        for ch in "ÀàÈèÉéÌìÒòÙù".chars() {
            assert_ne!(face.glyph_for(ch), 0, "{family} {style} lacks {ch}");
        }
    }
}

#[test]
fn russian_is_the_default_and_only_three_languages_are_offered() {
    let codes: Vec<&str> = Lang::all().map(Lang::code).collect();
    assert_eq!(codes, ["ru", "en", "zh-hans"]);
    assert_eq!(DEFAULT, "ru");
    assert_eq!(Lang::from_pref(AUTO), Lang::RU);
    assert_eq!(Lang::from_pref("xx"), Lang::RU);
    assert_eq!(normalize_pref("auto"), Some("ru"));
    assert_eq!(normalize_pref("EN"), Some("en"));
    assert_eq!(normalize_pref("de"), None);
    assert_eq!(tr(Lang::RU, "About"), "О программе");
}
