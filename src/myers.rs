//! The distance between a word and the pieces of a text, at every position of the text.
//!
//! For each position `j` of the text: the restricted Damerau-Levenshtein distance (insertions,
//! deletions, substitutions and swaps of two adjacent letters, one error each; also called
//! "optimal string alignment") between the word and the piece of text that ends at `j` and
//! resembles it most. This is Myers' bit-vector approximate search (1999), in Hyyrö's form
//! (2001) with Hyyrö's transpositions (2003): one column of the distance table fits in a `u64`,
//! and each character of the text costs about a dozen operations, whatever the tolerance. Words
//! longer than 64 letters use the full table, one cell at a time (`scan_slow`), which is also
//! the reference in the tests.

use crate::fold::UNKNOWN;

/// A word of 1..=64 letters ready for [`Pattern::scan`].
pub(crate) struct Pattern {
    /// For each byte, the bits of the positions of the word where it occurs.
    equal: [u64; 256],
    letters: usize,
    /// The bit of the last letter.
    top: u64,
}

impl Pattern {
    pub(crate) fn new(word: &[u8]) -> Pattern {
        assert!((1..=64).contains(&word.len()));
        let mut equal = [0u64; 256];
        for (i, &b) in word.iter().enumerate() {
            if b != UNKNOWN {
                equal[b as usize] |= 1 << i;
            }
        }
        Pattern {
            equal,
            letters: word.len(),
            top: 1 << (word.len() - 1),
        }
    }

    /// The state before the text: the column 0, 1, 2, …, m (always +1 going down).
    #[inline(always)]
    pub(crate) fn start(&self) -> State {
        State {
            up: !0,
            down: 0,
            zero_before: 0,
            equal_before: 0,
            distance: self.letters as u32,
        }
    }

    /// One more character: `state.distance` becomes the distance of the best piece ending here.
    /// The top row is all 0 (the word may start anywhere): that is why `plus` and `minus` shift
    /// without setting the low bit.
    #[inline(always)]
    pub(crate) fn step(&self, state: &mut State, c: u8) {
        let top = self.top;
        let equal = self.equal[c as usize];
        let up = state.up;
        let down = state.down;
        let swap = ((!state.zero_before & equal) << 1) & state.equal_before;
        let zero = (((equal & up).wrapping_add(up)) ^ up) | equal | down | swap;
        let plus = down | !(zero | up);
        let minus = up & zero;
        state.distance = state.distance + u32::from(plus & top != 0) - u32::from(minus & top != 0);
        let plus = plus << 1;
        let minus = minus << 1;
        state.up = minus | !(zero | plus);
        state.down = plus & zero;
        state.zero_before = zero;
        state.equal_before = equal;
    }

    /// Calls `visit(j, d)` for each position `j` of the text where a piece ending there is at
    /// distance `d <= tolerance` from the word, in order; stops when `visit` returns `false`.
    #[inline]
    pub(crate) fn scan(&self, text: &[u8], tolerance: u32, mut visit: impl FnMut(usize, u32) -> bool) {
        let mut state = self.start();
        for (j, &c) in text.iter().enumerate() {
            self.step(&mut state, c);
            if state.distance <= tolerance && !visit(j, state.distance) {
                return;
            }
        }
    }
}

/// How far [`Pattern::step`] has got in a text.
#[derive(Clone, Copy)]
pub(crate) struct State {
    up: u64,
    down: u64,
    zero_before: u64,
    equal_before: u64,
    pub distance: u32,
}

fn equal(a: u8, b: u8) -> bool {
    a == b && a != UNKNOWN
}

/// Like [`Pattern::scan`], with the full table: for long words and as the reference.
pub(crate) fn scan_slow(word: &[u8], text: &[u8], tolerance: u32, mut visit: impl FnMut(usize, u32) -> bool) {
    let m = word.len();
    let mut two_before: Vec<u32> = (0..=m as u32).collect();
    let mut before = two_before.clone();
    let mut now = vec![0u32; m + 1];
    for j in 0..text.len() {
        now[0] = 0;
        for i in 1..=m {
            let mut d = (before[i - 1] + u32::from(!equal(word[i - 1], text[j])))
                .min(before[i] + 1)
                .min(now[i - 1] + 1);
            if i >= 2 && j >= 1 && equal(word[i - 1], text[j - 1]) && equal(word[i - 2], text[j]) {
                d = d.min(two_before[i - 2] + 1);
            }
            now[i] = d;
        }
        if now[m] <= tolerance && !visit(j, now[m]) {
            return;
        }
        std::mem::swap(&mut two_before, &mut before);
        std::mem::swap(&mut before, &mut now);
    }
}

/// The same distance between two whole words.
pub(crate) fn distance(a: &[u8], b: &[u8]) -> u32 {
    let (m, n) = (a.len(), b.len());
    let mut t = vec![vec![0u32; n + 1]; m + 1];
    for (i, row) in t.iter_mut().enumerate() {
        row[0] = i as u32;
    }
    for (j, x) in t[0].iter_mut().enumerate() {
        *x = j as u32;
    }
    for i in 1..=m {
        for j in 1..=n {
            let mut d = (t[i - 1][j - 1] + u32::from(!equal(a[i - 1], b[j - 1])))
                .min(t[i - 1][j] + 1)
                .min(t[i][j - 1] + 1);
            if i >= 2 && j >= 2 && equal(a[i - 1], b[j - 2]) && equal(a[i - 2], b[j - 1]) {
                d = d.min(t[i - 2][j - 2] + 1);
            }
            t[i][j] = d;
        }
    }
    t[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The minimum distance at each position, by definition: the minimum of `distance` over all
    /// the pieces of text ending there (the empty one included).
    fn by_definition(word: &[u8], text: &[u8]) -> Vec<u32> {
        (0..text.len())
            .map(|j| (0..=j + 1).map(|s| distance(word, &text[s..j + 1])).min().unwrap())
            .collect()
    }

    fn all(word: &[u8], text: &[u8], slow: bool) -> Vec<u32> {
        let mut out = vec![u32::MAX; text.len()];
        let mut visit = |j, d| {
            out[j] = d;
            true
        };
        if slow {
            scan_slow(word, text, u32::MAX, &mut visit);
        } else {
            Pattern::new(word).scan(text, u32::MAX, &mut visit);
        }
        out
    }

    /// A small, repeatable random number generator.
    struct Random(u64);
    impl Random {
        fn n(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }
        fn word(&mut self, length: usize, letters: usize) -> Vec<u8> {
            (0..length).map(|_| b"abcdefgh"[self.n(letters)]).collect()
        }
    }

    #[test]
    fn hand_made_examples() {
        // "recieve" is "receive" with two letters swapped: one error.
        assert_eq!(distance(b"recieve", b"receive"), 1);
        assert_eq!(distance(b"ab", b"ba"), 1);
        assert_eq!(distance(b"ca", b"abc"), 3);
        assert_eq!(distance(b"", b"abc"), 3);
        assert_eq!(all(b"pipe", b"a ppie red", false)[5], 1);
        assert_eq!(all(b"pipe", b"a pipe red", false)[5], 0);
    }

    /// The full table gives what the definition says…
    #[test]
    fn full_table_matches_definition() {
        let mut random = Random(0x2545_F491_4F6C_DD1D);
        for _ in 0..3000 {
            let letters = 2 + random.n(4);
            let length = 1 + random.n(6);
            let word = random.word(length, letters);
            let length = random.n(12);
            let text = random.word(length, letters);
            assert_eq!(all(&word, &text, true), by_definition(&word, &text), "{word:?} {text:?}");
        }
    }

    /// …and the bit vectors give what the full table gives, up to 64 letters.
    #[test]
    fn bit_vectors_match_full_table() {
        let mut random = Random(0x9E37_79B9_7F4A_7C15);
        for round in 0..200_000 {
            let letters = 2 + random.n(6);
            let length = if round % 100 == 0 { 50 + random.n(15) } else { 1 + random.n(12) };
            let word = random.word(length, letters);
            let length = random.n(40);
            let text = random.word(length, letters);
            assert_eq!(all(&word, &text, false), all(&word, &text, true), "{word:?} {text:?}");
        }
    }

    #[test]
    fn unknown_characters_equal_nothing() {
        assert_eq!(all(&[UNKNOWN], &[UNKNOWN], false), [1]);
        assert_eq!(all(&[UNKNOWN], &[UNKNOWN], true), [1]);
        assert_eq!(distance(&[UNKNOWN, b'a'], &[b'a', UNKNOWN]), 2);
    }

    #[test]
    fn stops_when_asked() {
        let mut seen = Vec::new();
        Pattern::new(b"ab").scan(b"xxabxxabab", 0, |j, _| {
            seen.push(j);
            false
        });
        assert_eq!(seen, [3]);
    }
}
