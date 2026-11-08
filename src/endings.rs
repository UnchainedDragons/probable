//! Words that differ only in their ending: how likely they are to be the same word.
//!
//! In many languages a word changes its ending for number and gender ("tubo", "tubi"; "rosso",
//! "rossa"). Two such forms are the same word with a probability that depends on which endings
//! change and on how long the stem they share is (short stems are often two different words:
//! "caso", "casa"):
//!
//! ```text
//! logit P(same word) = α[ending a ↔ ending b] + β[letters in the stem]
//! ```
//!
//! The filter uses it to answer "does this entry have the word typed, in any form": a piece of
//! the entry that is another form of the piece the query stands for counts with that probability
//! instead of zero. The order of the results does not change.

/// The endings of a language and their weights.
#[derive(Debug)]
pub struct Endings {
    /// Pairs of endings (folded letters, either way round) and α.
    pub(crate) pairs: &'static [(&'static str, &'static str, f64)],
    /// β for stems of 1..=8 letters; 9 or more letters: 0.
    pub(crate) stem: [f64; 8],
}

impl Endings {
    /// No endings: two forms count as the same word only with the same letters.
    pub const NONE: Endings = Endings {
        pairs: &[],
        stem: [0.0; 8],
    };

    pub(crate) fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Italian: nouns, adjectives and past participles. Counted on the 163 882 pairs of forms of
    /// the Morph-it! lexicon (University of Bologna) that differ only in one of these endings,
    /// by maximum likelihood; only the counts are kept here.
    pub const ITALIAN: Endings = Endings {
        pairs: &[
            ("e", "i", 5.021),
            ("i", "o", 7.418),
            ("a", "e", 7.204),
            ("a", "i", 4.554),
            ("a", "o", 4.620),
            ("e", "o", 4.518),
            ("a", "he", 13.459),
            ("", "o", 3.419),
            ("he", "o", 4.587),
            ("he", "i", 4.167),
            ("", "a", 1.286),
            ("", "e", 1.140),
            ("hi", "o", 7.147),
            ("e", "ia", 0.968),
            ("a", "hi", 3.363),
            ("e", "io", -0.075),
            ("", "s", 3.390),
            ("ri", "te", 0.242),
            ("ri", "ta", 0.249),
            ("re", "ti", 1.959),
            ("re", "to", 2.017),
            ("re", "te", 2.074),
            ("re", "ta", 2.109),
        ],
        stem: [-12.493, -4.295, -3.448, -2.990, -1.757, -1.237, -0.875, -0.396],
    };

    /// Calls `visit(other form, P(same word))` for every other form of the folded `word` that the
    /// endings allow. The same form may come more than once, by different endings.
    pub(crate) fn variants(&self, word: &[u8], mut visit: impl FnMut(&[u8], f64)) {
        let mut other = Vec::with_capacity(word.len() + 3);
        for &(x, y, alpha) in self.pairs {
            for (from, to) in [(x.as_bytes(), y.as_bytes()), (y.as_bytes(), x.as_bytes())] {
                if !word.ends_with(from) || word.len() <= from.len() {
                    continue;
                }
                let stem = &word[..word.len() - from.len()];
                let beta = self.stem.get(stem.len() - 1).copied().unwrap_or(0.0);
                other.clear();
                other.extend_from_slice(stem);
                other.extend_from_slice(to);
                visit(&other, 1.0 / (1.0 + (-(alpha + beta)).exp()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &str, b: &str) -> f64 {
        let mut p = 0.0f64;
        Endings::ITALIAN.variants(a.as_bytes(), |o, q| {
            if o == b.as_bytes() {
                p = p.max(q);
            }
        });
        p
    }

    #[test]
    fn number_and_gender() {
        assert!(same("tubo", "tubi") > 0.97);
        assert!(same("raccordo", "raccordi") > 0.99);
        assert!(same("banco", "banchi") > 0.95);
        assert!(
            (0.7..0.9).contains(&same("costo", "costa")),
            "a short stem is less sure"
        );
        assert!(same("spa", "spi") < 0.6);
        assert_eq!(same("tubo", "tubolare"), 0.0, "not an ending");
        assert_eq!(same("o", "i"), 0.0, "no stem");
    }
}
