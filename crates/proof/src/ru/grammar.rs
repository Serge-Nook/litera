//! Russian grammar and punctuation checks: cheap, rule-based and tuned to stay quiet when unsure
//! (a missed comma costs less than a wrong mark on correct text). Messages are in Russian.
//!
//! - **Repeated word** (`не не`), except interjections and words Russian doubles on purpose
//!   (`так так`, `да да`, `очень очень`).
//! - **Space before punctuation** (`так ,`) and **double spaces**.
//! - **Capital letter at the start of a sentence**, knowing Russian abbreviations (`т. е.`, `г.`,
//!   `см.`, `и т. д.`), initials and ordinal numbers.
//! - **Missing comma** before conjunctions and relative pronouns that open a clause: `что`,
//!   `чтобы`, `потому`, `если`, `хотя`, `будто`, `словно`, `но` and the forms of `который` —
//!   unless a conjunction or particle comes first (`и что`, `только если`, `потому что`) or, for
//!   `который`, a preposition (`в котором`: the comma goes before it).
//! - **Doubled punctuation** (`,,`, `..`, `;;`).

use crate::{Issue, IssueKind, ProofLang, words};

/// Words Polish repeats on purpose: interjections, onomatopoeia and emphatic doubling.
const ALLOWED_REPEATS: &[&str] = &[
    "так",
    "да",
    "нет",
    "ну",
    "ха",
    "хи",
    "хе",
    "ой",
    "ай",
    "эй",
    "ох",
    "ах",
    "эх",
    "тук",
    "бум",
    "чик",
    "кап",
    "очень",
    "долго",
    "давно",
    "далеко",
    "много",
    "скорей",
    "скорее",
    "быстро",
];

/// Abbreviations written with a final period after which a sentence goes on in lower case
/// (lower case, without the period).
const ABBREVIATIONS: &[&str] = &[
    "т",
    "е",
    "д",
    "п",
    "г",
    "гг",
    "в",
    "вв",
    "см",
    "ср",
    "др",
    "пр",
    "им",
    "ул",
    "д",
    "кв",
    "стр",
    "рис",
    "табл",
    "тыс",
    "млн",
    "млрд",
    "руб",
    "коп",
    "к",
    "н",
    "э",
    "обл",
    "р",
    "пос",
    "просп",
    "пер",
    "пл",
    "м",
    "км",
    "кг",
    "ч",
    "мин",
    "сек",
    "тел",
    "напр",
    "проф",
    "акад",
    "доц",
    "канд",
    "англ",
    "лат",
    "нем",
    "франц",
    "рус",
    "т.е",
    "т.д",
    "т.п",
    "т.к",
    "н.э",
    "и.о",
    "др",
];

/// Subordinating conjunctions and relative pronouns that open a clause: a comma comes before
/// them.
const CLAUSE_OPENERS: &[&str] = &[
    "что",
    "чтобы",
    "потому",
    "если",
    "хотя",
    "будто",
    "словно",
    "который",
    "которая",
    "которое",
    "которые",
    "которого",
    "которой",
    "которому",
    "которую",
    "которым",
    "которых",
    "которыми",
    "котором",
    "но",
];

/// Words after which a clause opener takes no comma of its own: conjunctions (`i że`), particles
/// (`tylko jeśli`, `zwłaszcza że`) and the first halves of compound conjunctions (`mimo że`,
/// `chyba że`, `tak aby`, `jak gdyby`, `na wypadek gdyby`, `z tym że`).
const NO_COMMA_AFTER: &[&str] = &[
    "и",
    "а",
    "но",
    "да",
    "или",
    "либо",
    "ни",
    "не",
    "лишь",
    "только",
    "даже",
    "именно",
    "так",
    "то",
    "что",
    "чем",
    "ли",
    "же",
    "разве",
    "почти",
    "вот",
    "всё",
    "все",
    "ведь",
    "потому",
    "оттого",
    "затем",
    "тогда",
    "как",
    "прежде",
    "раньше",
    "несмотря",
    "после",
    "до",
    "перед",
];

/// Prepositions: a relative pronoun after one takes the comma before the preposition instead
/// (`dom, w którym`).
const PREPOSITIONS: &[&str] = &[
    "в",
    "во",
    "на",
    "о",
    "об",
    "обо",
    "с",
    "со",
    "к",
    "ко",
    "по",
    "у",
    "из",
    "изо",
    "от",
    "ото",
    "до",
    "за",
    "под",
    "подо",
    "над",
    "надо",
    "при",
    "про",
    "для",
    "без",
    "безо",
    "через",
    "между",
    "перед",
    "передо",
    "после",
    "около",
    "возле",
    "вокруг",
    "среди",
    "из-за",
    "из-под",
    "благодаря",
    "согласно",
    "вследствие",
];

fn issue(start: usize, end: usize, message: String, suggestions: Vec<String>) -> Issue {
    Issue { start, end, kind: IssueKind::Grammar, message, suggestions, lang: ProofLang::Ru }
}

fn capitalize(w: &str) -> String {
    crate::affix::capitalize(w)
}

/// Polish vowels (and the accented vowels of loanwords).
/// Grammar issues in a paragraph of Russian text, in order.
pub fn check(text: &str) -> Vec<Issue> {
    let ws: Vec<(usize, usize)> = words(text);
    let mut v = Vec::new();
    repeated_words(text, &ws, &mut v);
    spaces(text, &mut v);
    sentence_capitals(text, &ws, &mut v);
    missing_commas(text, &ws, &mut v);
    doubled_punctuation(text, &mut v);
    v.sort_by_key(|i| (i.start, i.end));
    v
}

fn repeated_words(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    for pair in ws.windows(2) {
        let [(a0, b0), (a1, b1)] = pair else { continue };
        let (Some(w0), Some(w1), Some(gap)) = (text.get(*a0..*b0), text.get(*a1..*b1), text.get(*b0..*a1)) else { continue };
        if !gap.is_empty()
            && gap.chars().all(char::is_whitespace)
            && w0.chars().all(char::is_alphabetic)
            && w0.to_lowercase() == w1.to_lowercase()
            && !ALLOWED_REPEATS.contains(&w0.to_lowercase().as_str())
        {
            v.push(issue(*a0, *b1, format!("Повтор слова: «{w1}»"), vec![w0.to_string()]));
        }
    }
}

fn spaces(text: &str, v: &mut Vec<Issue>) {
    for (i, c) in text.char_indices() {
        if c != ' ' {
            continue;
        }
        let rest = text.get(i + 1..).unwrap_or("");
        let mut it = rest.chars();
        let Some(p) = it.next() else { continue };
        let after = it.next();
        let flag = match p {
            ',' => true,
            // `;)` and `;-)` are emoticons.
            ';' => !matches!(after, Some(')' | '(' | '-' | 'P' | 'D')),
            // ` ...` is an ellipsis, ` .5` a number.
            '.' => !matches!(after, Some('.')) && !after.is_some_and(|a| a.is_ascii_digit()),
            // ` :)`, ` :-)`, ` :D` and similar are emoticons; ` ?!` is still a stray space.
            ':' | '!' | '?' => after.is_none_or(|a| a.is_whitespace() || matches!(a, '"' | '”' | '»' | '’' | '!' | '?')),
            _ => false,
        };
        if flag {
            v.push(issue(i, i + 1 + p.len_utf8(), "Лишний пробел перед знаком препинания".into(), vec![p.to_string()]));
        }
    }
    for (i, _) in text.match_indices("  ") {
        if i > 0 && text.get(..i).is_some_and(|t| t.ends_with(' ')) {
            continue;
        }
        v.push(issue(i, i + 2, "Двойной пробел".into(), vec![" ".into()]));
    }
}

/// Is `prev`, before a period, an abbreviation, an initial, a number or a Roman numeral (after
/// which a lower-case word doesn't start a sentence)?
fn abbreviation_or_number(prev: &str) -> bool {
    prev.chars().count() <= 1
        || prev.chars().any(|c| c.is_ascii_digit())
        || (prev.chars().all(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M')))
        || ABBREVIATIONS.contains(&prev.to_lowercase().as_str())
}

const CLOSING: [char; 8] = ['"', '”', '’', '\'', ')', '»', ']', '\u{1}'];
const OPENING: [char; 8] = ['„', '"', '“', '«', '(', '[', '\'', '‚'];

/// Does the gap between two words end a sentence: a `.`, `!` or `?` (not an ellipsis, not after
/// an abbreviation), then space, with nothing but quotes and brackets around?
fn ends_sentence(gap: &str, prev: &str) -> bool {
    let t = gap.trim_start_matches(CLOSING);
    let Some(end) = t.chars().next() else { return false };
    if !matches!(end, '.' | '!' | '?') || t.starts_with("..") {
        return false;
    }
    if end == '.' && abbreviation_or_number(prev) {
        return false;
    }
    let rest = t.trim_start_matches(['.', '!', '?']).trim_start_matches(CLOSING);
    rest.starts_with(char::is_whitespace) && rest.trim_start().chars().all(|c| OPENING.contains(&c))
}

fn sentence_capitals(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    // List items (`– jabłka,`) may start in lower case.
    let list_item = text.trim_end().ends_with([',', ';']);
    for (k, &(a, b)) in ws.iter().enumerate() {
        let Some(w) = text.get(a..b) else { continue };
        let starts = if k == 0 {
            let before = text.get(..a).unwrap_or("");
            !list_item && before.chars().all(|c| c.is_whitespace() || OPENING.contains(&c))
        } else {
            let Some(&(pa, pb)) = ws.get(k - 1) else { continue };
            ends_sentence(text.get(pb..a).unwrap_or(""), text.get(pa..pb).unwrap_or(""))
        };
        if starts && w.chars().next().is_some_and(char::is_lowercase) && w.chars().all(char::is_alphabetic) {
            v.push(issue(a, b, "Предложение начинается со строчной буквы".into(), vec![capitalize(w)]));
        }
    }
}

fn missing_commas(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    for k in 1..ws.len() {
        let (Some(&(pa, pb)), Some(&(a, b))) = (ws.get(k - 1), ws.get(k)) else { continue };
        let (Some(prev), Some(w), Some(gap)) = (text.get(pa..pb), text.get(a..b), text.get(pb..a)) else { continue };
        if !CLAUSE_OPENERS.contains(&w) || gap.is_empty() || !gap.chars().all(|c| c == ' ' || c == '\u{a0}') {
            continue;
        }
        if !prev.chars().all(char::is_alphabetic) {
            continue;
        }
        let p = prev.to_lowercase();
        let p = p.as_str();
        if NO_COMMA_AFTER.contains(&p) {
            continue;
        }
        if w.starts_with("котор") && (PREPOSITIONS.contains(&p) || p == "то") {
            continue;
        }
        v.push(issue(pb, b, format!("Пропущена запятая перед «{w}»"), vec![format!(", {w}")]));
    }
}

fn doubled_punctuation(text: &str, v: &mut Vec<Issue>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        if !matches!(b, b',' | b'.' | b';') {
            i += 1;
            continue;
        }
        let run = bytes.get(i..).map_or(0, |r| r.iter().take_while(|&&x| x == b).count());
        let flag = match b {
            b'.' => run == 2,
            _ => run >= 2,
        };
        if flag {
            let end = i + run;
            let suggestions: Vec<String> = match b {
                // `,,` before a word, after a space: a typed opening quote.
                b',' if text.get(end..).and_then(|t| t.chars().next()).is_some_and(char::is_alphabetic)
                    && (i == 0 || text.get(..i).is_some_and(|t| t.ends_with(char::is_whitespace))) =>
                {
                    vec!["«".into()]
                }
                b'.' => vec![".".into(), "…".into()],
                _ => vec![char::from(b).to_string()],
            };
            v.push(issue(i, end, "Повтор знака препинания".into(), suggestions));
        }
        i += run.max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(t: &str) -> Vec<String> {
        check(t).iter().map(|i| format!("{}@{}", i.message, i.suggestions.join("|"))).collect()
    }

    fn clean(t: &str) {
        assert!(check(t).is_empty(), "{t}: {:?}", messages(t));
    }

    #[test]
    fn repeated_words_and_spaces() {
        let m = messages("Я не не знаю.  Да ,  так.");
        assert!(m.contains(&"Повтор слова: «не»@не".to_string()), "{m:?}");
        assert!(m.iter().any(|s| s.starts_with("Лишний пробел")), "{m:?}");
        assert!(m.iter().any(|s| s.starts_with("Двойной пробел")), "{m:?}");
        clean("Так так, понятно. Да да, конечно.");
    }

    #[test]
    fn sentence_capitals() {
        let m = messages("Пришёл домой. сел за стол.");
        assert!(m.contains(&"Предложение начинается со строчной буквы@Сел".to_string()), "{m:?}");
        clean("Он живёт в г. Москве, см. рис. 2 и т. д. и т. п. вот так.");
    }

    #[test]
    fn missing_commas() {
        let m = messages("Я знаю что он придёт. Это дом который построил Джек. Приду если смогу.");
        for want in ["Пропущена запятая перед «что»@, что", "Пропущена запятая перед «который»@, который", "Пропущена запятая перед «если»@, если"]
        {
            assert!(m.contains(&want.to_string()), "{want}: {m:?}");
        }
        clean("Я знаю, что он придёт, и что останется.");
        clean("Это дом, в котором я живу. Не пришёл, потому что болел.");
        clean("Что делать? Который час?");
    }

    #[test]
    fn russian_text_is_clean() {
        clean("Мороз и солнце; день чудесный! Ещё ты дремлешь, друг прелестный.");
    }
}
