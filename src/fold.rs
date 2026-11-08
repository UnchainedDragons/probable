//! Folded text: no case, no accents, no separators.
//!
//! Every letter or digit becomes one or more bytes (Latin letters always in ASCII: "È" → `e`,
//! "ß" → `ss`, "²" → `2`); everything else (spaces, punctuation, symbols) is a separator and
//! disappears. So "MULTI-LAYER", "multi layer" and "Multilayer" all become `multilayer`, and a
//! word split differently in the text and in the query is still found.
//!
//! Where a word began stays marked: after a separator, at a change between letters and digits
//! ("RAC100MF" → `rac` `100` `mf`) and from lowercase to uppercase ("UnitPrice").
//!
//! Non-Latin letters (Greek, Cyrillic…) stay themselves in lowercase and get a code from 128 to
//! 254, assigned by the index in the order it meets them; beyond 127 different ones, and for
//! query letters the index has never seen, the code is [`UNKNOWN`], which equals nothing (not
//! even itself).

use crate::table::TABLE;

/// The code of a character that compares equal to nothing.
pub(crate) const UNKNOWN: u8 = 255;

/// What a character becomes.
enum Piece {
    /// A separator: it disappears.
    Separator,
    /// A lowercase ASCII letter or digit.
    Ascii(u8),
    /// A Latin letter folded into ASCII letters (one or more, from the table).
    Table(&'static str),
    /// Another letter or digit, already lowercase.
    Other(char),
}

fn piece(c: char) -> Piece {
    if c.is_ascii() {
        return if c.is_ascii_alphanumeric() {
            Piece::Ascii(c.to_ascii_lowercase() as u8)
        } else {
            Piece::Separator
        };
    }
    if let Ok(i) = TABLE.binary_search_by_key(&c, |&(k, _)| k) {
        return Piece::Table(TABLE[i].1);
    }
    if c.is_alphanumeric() {
        Piece::Other(c.to_lowercase().next().unwrap_or(c))
    } else {
        Piece::Separator
    }
}

/// Letter or digit, for word starts between the two.
fn is_digit(b: u8, c: char) -> bool {
    if b < 128 { b.is_ascii_digit() } else { c.is_numeric() }
}

/// Folds `text` into `out`, one byte per letter or digit. For each byte written it calls
/// `mark(word_start, index_of_the_original_character)`. `code` gives the code of non-Latin
/// letters (see the module).
pub(crate) fn fold(text: &str, out: &mut Vec<u8>, mut code: impl FnMut(char) -> u8, mut mark: impl FnMut(bool, usize)) {
    let mut after_separator = true;
    let mut digit_before = false;
    let mut lowercase_before = false;
    for (index, c) in text.chars().enumerate() {
        let mut write = |b: u8, first: bool, out: &mut Vec<u8>| {
            let digit = is_digit(b, c);
            let start =
                first && (after_separator || digit != digit_before || (lowercase_before && c.is_uppercase()));
            out.push(b);
            mark(start, index);
            digit_before = digit;
        };
        match piece(c) {
            Piece::Separator => {
                after_separator = true;
                lowercase_before = false;
                continue;
            }
            Piece::Ascii(b) => write(b, true, out),
            Piece::Table(s) => {
                for (i, b) in s.bytes().enumerate() {
                    write(b, i == 0, out);
                }
            }
            Piece::Other(c) => write(code(c), true, out),
        }
        after_separator = false;
        lowercase_before = c.is_lowercase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folded(text: &str) -> (String, String) {
        let mut out = Vec::new();
        let mut starts = String::new();
        fold(text, &mut out, |_| UNKNOWN, |start, _| {
            starts.push(if start { '^' } else { '.' });
        });
        (String::from_utf8_lossy(&out).into_owned(), starts)
    }

    #[test]
    fn no_case_accents_or_separators() {
        assert_eq!(folded("Café NO").0, "cafeno");
        assert_eq!(folded("MULTI-LAYER").0, "multilayer");
        assert_eq!(folded("Straße Æsir").0, "strasseaesir");
        assert_eq!(folded("D. 80 m²").0, "d80m2");
        assert_eq!(folded("Città — l'àncora").0, "cittalancora");
        assert_eq!(folded("ＡＢＣ１").0, "abc1");
        assert_eq!(folded("  ").0, "");
    }

    #[test]
    fn word_starts() {
        assert_eq!(folded("RAC100MF x"), ("rac100mfx".into(), "^..^..^.^".into()));
        assert_eq!(folded("UnitPrice"), ("unitprice".into(), "^...^....".into()));
        assert_eq!(folded("ßa"), ("ssa".into(), "^..".into()));
        assert_eq!(folded("d.80"), ("d80".into(), "^^.".into()));
    }

    #[test]
    fn non_latin_letters_get_a_code() {
        let mut out = Vec::new();
        let mut seen = Vec::new();
        fold(
            "Ωμέγα",
            &mut out,
            |c| {
                seen.push(c);
                200
            },
            |_, _| {},
        );
        assert_eq!(out, [200; 5]);
        assert_eq!(seen, ['ω', 'μ', 'έ', 'γ', 'α']);
    }

    #[test]
    fn the_table_is_sorted() {
        assert!(TABLE.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(TABLE.iter().all(|(_, s)| s.bytes().all(|b| b.is_ascii_alphanumeric() && !b.is_ascii_uppercase())));
    }
}
