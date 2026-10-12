//! Russian proofing.
//!
//! - **Spelling** against a Hunspell `ru_RU` dictionary: the one installed on the system (found
//!   by [`crate::hunspell`]) or, without one, the dictionary bundled with the program
//!   (A. Lebedev's, `assets/spelling/ru_RU.*.z`). Capitalized stems are names, hyphenated
//!   compounds are checked part by part, `ё` may be written as `е`, and the user dictionary
//!   applies as in English. Words in other scripts are left alone.
//! - **Suggestions**: letter case, one-letter typos (the dictionary's `TRY` letters), a missing
//!   space and a dictionary scan, as for Polish.
//! - **Grammar** ([`grammar`]): cheap, reliable rules with Russian messages.

pub mod grammar;

use std::sync::{Arc, RwLock};

use crate::affix::AffixDict;

static DICT: RwLock<Option<Arc<AffixDict>>> = RwLock::new(None);

const NAMES: &[&str] = &["ru_RU", "ru-RU", "ru"];
const BUNDLED_AFF: &[u8] = include_bytes!("../../../../assets/spelling/ru_RU.aff.z");
const BUNDLED_DIC: &[u8] = include_bytes!("../../../../assets/spelling/ru_RU.dic.z");

/// The Russian dictionary: the installed Hunspell `ru_RU`, else the bundled one (read on first
/// use), or one given with [`install_dictionary`].
pub fn dictionary() -> Arc<AffixDict> {
    if let Some(d) = DICT.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return d.clone();
    }
    let loaded = Arc::new(load());
    let mut slot = DICT.write().unwrap_or_else(|e| e.into_inner());
    slot.get_or_insert(loaded).clone()
}

fn load() -> AffixDict {
    if let Some((aff, dic)) = crate::hunspell::find(NAMES) {
        match crate::hunspell::load(&aff, &dic) {
            Ok(d) if !d.is_empty() => return d,
            Ok(_) => {}
            Err(e) => log::warn!("Russian spelling: {e}"),
        }
    }
    load_bundled().unwrap_or_else(|e| {
        log::warn!("Russian spelling: bundled dictionary: {e}");
        AffixDict::default()
    })
}

fn inflate(data: &[u8]) -> Result<String, String> {
    let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(data).map_err(|e| format!("{e:?}"))?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// The dictionary bundled with the program.
pub fn load_bundled() -> Result<AffixDict, String> {
    AffixDict::from_hunspell(&inflate(BUNDLED_AFF)?, &inflate(BUNDLED_DIC)?, &[])
}

/// Use this Russian dictionary from now on (Hunspell `.aff` and `.dic` text, decoded to UTF-8).
/// On error the current dictionary stays.
pub fn install_dictionary(aff: &str, dic: &str) -> Result<(), String> {
    let d = AffixDict::from_hunspell(aff, dic, &[])?;
    *DICT.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(d));
    Ok(())
}

/// Forget the loaded dictionary, so the next check looks for one again.
pub fn reload_dictionary() {
    *DICT.write().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Is a Russian dictionary available (spelling is checked)?
pub fn has_dictionary() -> bool {
    !dictionary().is_empty()
}

fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{052F}').contains(&c)
}

/// Is `word` spelled correctly in Russian? Numbers, single letters, short words in capitals
/// (acronyms: `МГУ`, `ООО`), words with digits and words in other scripts are accepted.
pub fn is_correct(word: &str) -> bool {
    is_correct_with(&dictionary(), word)
}

/// [`is_correct`] against a given dictionary (an empty one accepts everything).
pub fn is_correct_with(d: &AffixDict, word: &str) -> bool {
    if d.is_empty() {
        return true;
    }
    let w = word.trim_matches(|c: char| !c.is_alphanumeric());
    let n = w.chars().count();
    if n <= 1 || w.chars().any(|c| c.is_ascii_digit()) || !w.chars().any(char::is_alphabetic) {
        return true;
    }
    if w.chars().all(|c| !c.is_lowercase()) && n <= 6 {
        return true;
    }
    if !w.chars().all(|c| !c.is_alphabetic() || is_cyrillic(c)) {
        return true;
    }
    if w.contains('-') {
        return w.split('-').all(|p| p.chars().count() <= 1 || known(d, p));
    }
    known(d, w)
}

fn known(d: &AffixDict, w: &str) -> bool {
    if d.check(w) || crate::in_user_dictionary(&w.to_lowercase()) {
        return true;
    }
    if w.contains(['ё', 'Ё']) {
        let e = w.replace('ё', "е").replace('Ё', "Е");
        return d.check(&e);
    }
    false
}

/// Spelling suggestions for a misspelled Russian word, best first (at most `max`).
pub fn suggest(word: &str, max: usize) -> Vec<String> {
    crate::pl::suggest::suggest_in(&dictionary(), word, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled() -> AffixDict {
        load_bundled().unwrap_or_default()
    }

    #[test]
    fn bundled_dictionary_loads() {
        let d = bundled();
        assert!(d.len() > 100_000, "{}", d.len());
    }

    #[test]
    fn russian_words_are_correct() {
        let d = bundled();
        for w in [
            "привет",
            "Привет",
            "мама",
            "мыла",
            "раму",
            "программа",
            "документов",
            "редактирования",
            "Москва",
            "Москве",
            "ПРИВЕТ",
            "кто-нибудь",
            "ещё",
            "еще",
            "ёлка",
            "МГУ",
            "2026",
            "я",
            "красивая",
            "сделали",
            "пишущий",
        ] {
            assert!(is_correct_with(&d, w), "{w}");
        }
    }

    #[test]
    fn misspellings_are_flagged() {
        let d = bundled();
        for w in ["превет", "карова", "сабака", "москва", "праграмма", "жывот"] {
            assert!(!is_correct_with(&d, w), "{w}");
        }
        for w in ["hello", "xqzv"] {
            assert!(is_correct_with(&d, w), "other scripts are not Russian: {w}");
        }
    }

    #[test]
    fn suggestions_find_the_word() {
        let d = bundled();
        assert!(crate::pl::suggest::suggest_in(&d, "превет", 5).contains(&"привет".to_string()));
        assert!(crate::pl::suggest::suggest_in(&d, "сабака", 5).contains(&"собака".to_string()));
    }
}
