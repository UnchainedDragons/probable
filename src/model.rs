//! The model: how a query word is typed, given what the person wants. The formulas and their
//! reasons are in the README, "The model".
//!
//! Someone wants an entry `k`. For each word they type, they pick a word `t` of `k` (more often
//! one near the start of its field, and in a heavier field) and type it, or (probability η) they
//! type something that is not in `k`. A word of `k` is typed
//!
//! - whole, or only its beginning, because the search runs while they type (probability μ: the
//!   length typed so far is any of 1..=L, the whole word included);
//! - or a piece from inside it (probability ι), as with codes;
//! - or together with the next word, the space left out (probability σ per space).
//!
//! Every letter is mistyped with probability ρ; a mistake is one of about `A` letters (the
//! perplexity of the letters of the list), so a piece at distance `d` from the typed word costs
//! `(1 − ρ)^(m − d) (ρ / A)^d`. Something not in `k` is a word of the list with probability λ, a
//! new string otherwise (letters drawn with the frequencies of the list, length geometric with
//! the mean length of the words of the list).

/// The most errors searched, per word.
pub(crate) const MAX_ERRORS: usize = 3;

/// μ: the probability that a word is being typed and is not finished yet.
const PARTIAL: f64 = 0.3;

/// ι: the probability that the word typed is a piece from inside a word of the entry.
const INNER: f64 = 0.1;

/// σ: the probability of typing two words of the entry without the space between them, or one
/// word of the entry with a space inside.
pub(crate) const SPACE: f64 = 0.05;

/// λ: when the word typed is not in the wanted entry, the probability that it is a word of some
/// other entry rather than a new string.
pub(crate) const FOREIGN_LISTED: f64 = 0.5;

/// The probability of typing a word of `m` letters from a piece of a word of the entry.
pub(crate) struct Channel {
    /// `(1 − ρ)^(m − d) (ρ / A)^d` for d = 0..=MAX_ERRORS.
    typos: [f64; MAX_ERRORS + 1],
}

impl Channel {
    pub(crate) fn new(m: usize, rho: f64, alphabet: f64) -> Channel {
        let mut typos = [0.0; MAX_ERRORS + 1];
        for (d, t) in typos.iter_mut().enumerate() {
            let right = m.saturating_sub(d) as f64;
            *t = (right * (-rho).ln_1p() + d as f64 * (rho / alphabet).ln()).exp();
        }
        Channel { typos }
    }

    /// The word typed whole: `length` letters, `d` errors.
    pub(crate) fn whole(&self, length: usize, d: u32) -> f64 {
        let length = length.max(1) as f64;
        (1.0 - PARTIAL - INNER + PARTIAL / length) * self.typos[d as usize]
    }

    /// Only the beginning of a word of `length` letters, `d` errors.
    pub(crate) fn beginning(&self, length: usize, d: u32) -> f64 {
        PARTIAL / length.max(1) as f64 * self.typos[d as usize]
    }

    /// A piece from inside a word of `length` letters (where it starts and how long it is, both
    /// uniform), `d` errors.
    pub(crate) fn inside(&self, length: usize, d: u32) -> f64 {
        let length = length.max(1) as f64;
        INNER / (length * length) * self.typos[d as usize]
    }
}

/// The probability of the folded `word` as a new string: length geometric with mean `mean`,
/// letters drawn with `frequencies`.
pub(crate) fn new_string(word: &[u8], frequencies: &[f64; 256], mean: f64) -> f64 {
    let mean = mean.max(1.0);
    let ln_length = -mean.ln() + (word.len().max(1) - 1) as f64 * (-1.0 / mean).ln_1p();
    let ln_letters: f64 = word.iter().map(|&b| frequencies[b as usize].ln()).sum();
    (ln_length + ln_letters).exp().max(f64::MIN_POSITIVE)
}

/// The logarithm of `(x + 1)^-β / Σ_{y=1}^{n} y^-β`: how likely word `x` of a field of `n` words
/// is to be the one chosen.
pub(crate) struct Depth {
    beta: f64,
    /// `-β ln(x + 1)` for x = 0, 1, …
    ln_weights: Vec<f64>,
    /// `ln Σ_{y=1}^{n} y^-β` for `n` = 0, 1, …
    ln_sums: Vec<f64>,
    last_sum: f64,
}

impl Depth {
    pub(crate) fn new(beta: f64, up_to: usize) -> Depth {
        let up_to = up_to.clamp(1, 1 << 16);
        let ln_weights = (0..up_to).map(|x| -beta * ((x + 1) as f64).ln()).collect();
        let mut ln_sums = Vec::with_capacity(up_to + 1);
        ln_sums.push(f64::NEG_INFINITY);
        let mut sum = 0.0;
        for y in 1..=up_to {
            sum += (y as f64).powf(-beta);
            ln_sums.push(sum.ln());
        }
        Depth {
            beta,
            ln_weights,
            ln_sums,
            last_sum: sum,
        }
    }

    fn ln_sum(&self, n: usize) -> f64 {
        if let Some(&s) = self.ln_sums.get(n) {
            return s;
        }
        // Beyond the table, the integral.
        let f = (self.ln_sums.len() - 1) as f64;
        let n = n as f64;
        let tail = if (self.beta - 1.0).abs() < 1e-9 {
            (n / f).ln()
        } else {
            (n.powf(1.0 - self.beta) - f.powf(1.0 - self.beta)) / (1.0 - self.beta)
        };
        (self.last_sum + tail).ln()
    }

    pub(crate) fn ln_weight(&self, position: usize, out_of: usize) -> f64 {
        let weight = match self.ln_weights.get(position) {
            Some(&w) => w,
            None => -self.beta * ((position + 1) as f64).ln(),
        };
        weight - self.ln_sum(out_of.max(position + 1))
    }
}

/// `ln(e^a + e^b)`.
pub(crate) fn ln_add(a: f64, b: f64) -> f64 {
    let (high, low) = if a >= b { (a, b) } else { (b, a) };
    if low == f64::NEG_INFINITY {
        return high;
    }
    high + (low - high).exp().ln_1p()
}

/// The most typing mistakes about spaces in one reading of the query: readings with more are
/// worth σ³ or less, and are left out.
const MOST_SPACE_MISTAKES: usize = 2;

/// The ways of reading a query of `n` words: each word on its own, two adjacent words as one with
/// a space typed by mistake, or one word as two words of the entry typed without the space
/// between them (probability σ per space added or left out, 1 − σ per space right). Units: word
/// `j` is unit `j`, words `j` and `j + 1` together unit `n + j`, the halves of word `j` units
/// `halves[j]` and `halves[j] + 1` (`usize::MAX`: not split). Each reading: (ln prior, units).
pub(crate) fn readings(n: usize, halves: &[usize]) -> Vec<(f64, Vec<usize>)> {
    fn more(
        j: usize,
        mistakes: usize,
        ln_prior: f64,
        units: &mut Vec<usize>,
        n: usize,
        halves: &[usize],
        out: &mut Vec<(f64, Vec<usize>)>,
    ) {
        if j == n {
            out.push((ln_prior, units.clone()));
            return;
        }
        let gap = if j > 0 { (-SPACE).ln_1p() } else { 0.0 };
        let mistake = gap + SPACE.ln();
        units.push(j);
        more(j + 1, mistakes, ln_prior + gap, units, n, halves, out);
        units.pop();
        if mistakes < MOST_SPACE_MISTAKES {
            if halves[j] != usize::MAX {
                units.extend([halves[j], halves[j] + 1]);
                more(j + 1, mistakes + 1, ln_prior + mistake, units, n, halves, out);
                units.truncate(units.len() - 2);
            }
            if j + 1 < n {
                units.push(n + j);
                more(j + 2, mistakes + 1, ln_prior + mistake, units, n, halves, out);
                units.pop();
            }
        }
    }
    let mut out = Vec::new();
    more(0, 0, 0.0, &mut Vec::new(), n, halves, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reading_once() {
        // Three words, the last one split: a|b|c, a|b|c', ab|c, ab|c', a|bc, and with two
        // mistakes at most not a|bc… (there is no third mistake to make).
        let r = readings(3, &[usize::MAX, usize::MAX, 5]);
        let mut units: Vec<Vec<usize>> = r.iter().map(|x| x.1.clone()).collect();
        units.sort();
        assert_eq!(
            units,
            [
                vec![0, 1, 2],
                vec![0, 1, 5, 6],
                vec![0, 4],
                vec![3, 2],
                vec![3, 5, 6]
            ]
        );
        let q = (-SPACE).ln_1p();
        let prior = |u: &[usize]| r.iter().find(|x| x.1 == u).unwrap().0;
        assert!((prior(&[0, 1, 2]) - 2.0 * q).abs() < 1e-12);
        assert!((prior(&[3, 5, 6]) - (2.0 * SPACE.ln() + q)).abs() < 1e-12);
    }

    #[test]
    fn a_word_typed_whole_beats_its_beginning() {
        let c = Channel::new(5, 0.03, 20.0);
        assert!(c.whole(5, 0) > c.beginning(8, 0));
        assert!(c.beginning(8, 0) > c.inside(8, 0));
        assert!(c.whole(5, 0) > 100.0 * c.whole(5, 1));
    }
}
