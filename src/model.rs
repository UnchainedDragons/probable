//! The model: how much a word found (or not found) says about whether an entry is relevant, and
//! how likely each entry is to be relevant. The formulas and the reasons behind them are in the
//! README, "How it works".

use std::f64::consts::LN_10;

/// The maximum number of errors searched for, per word.
pub(crate) const MAX_ERRORS: usize = 3;

/// No match found.
pub(crate) const NONE: u16 = u16::MAX;

/// Where a query word first occurs in an entry, for each maximum number of errors `d` and for
/// each of the four ways a piece of text can resemble it. Positions beyond 65 534 stay at 65 534.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Occurrences {
    /// A whole word of the text: which one (0 is the first).
    pub whole: [u16; MAX_ERRORS + 1],
    /// The beginning of a word of the text that goes on further: which word.
    pub prefix: [u16; MAX_ERRORS + 1],
    /// Inside a word, not at its beginning: the letter where it starts.
    pub inner: [u16; MAX_ERRORS + 1],
    /// Across two or more words of the text: the letter where it starts.
    pub spanning: [u16; MAX_ERRORS + 1],
}

impl Occurrences {
    pub(crate) const EMPTY: Occurrences = Occurrences {
        whole: [NONE; MAX_ERRORS + 1],
        prefix: [NONE; MAX_ERRORS + 1],
        inner: [NONE; MAX_ERRORS + 1],
        spanning: [NONE; MAX_ERRORS + 1],
    };

    pub(crate) fn found(&self) -> bool {
        [self.whole, self.prefix, self.inner, self.spanning].iter().any(|a| a.iter().any(|&p| p != NONE))
    }
}

/// The constants of the model.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Constants {
    /// β: the key at position `i` is worth `(i + 1)^-β` relative to the one at the start.
    pub depth: f64,
    /// ρ: the probability of mistyping a letter.
    pub typo_rate: f64,
    /// η: the probability that the wanted entry lacks one of the typed words.
    pub missing_word: f64,
}

/// Where, in the wanted entry, the piece matching the typed word lies: at the start of a word of
/// the text, inside a word (someone typing part of a code), or across two words (someone typing
/// as one word what the text splits, "multilayer" for "MULTI LAYER").
const AT_WORD_START: f64 = 0.8;
const INNER: f64 = 0.1;
const SPANNING: f64 = 0.1;

/// μ: the probability that a word is typed only in part (search as you type).
const PARTIAL: f64 = 0.3;

/// Imaginary entries, with the letter frequencies only, added to the real ones when estimating
/// how often a word occurs by chance: with few texts the estimate does not go to zero.
const IMAGINARY_ENTRIES: f64 = 20.0;

/// The logarithm of `(x + 1)^-β / Σ_{y=1}^{n} y^-β`, the weight of position `x` out of `n`.
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

    fn ln_weight(&self, position: usize, out_of: usize) -> f64 {
        let weight = match self.ln_weights.get(position) {
            Some(&w) => w,
            None => -self.beta * ((position + 1) as f64).ln(),
        };
        weight - self.ln_sum(out_of.max(position + 1))
    }
}

/// The shape of an entry: its length, and where its fields start.
pub(crate) struct Shape<'a> {
    pub letters: usize,
    pub words: usize,
    /// For each field, where it starts in the entry: (letter, word).
    pub fields: &'a [(u32, u32)],
    /// For each field, the logarithm of its share of the weights.
    pub ln_weights: &'a [f64],
}

impl Shape<'_> {
    /// The field holding position `at` (a letter if `letters`, else a word): its log weight, the
    /// position inside it and its length.
    fn locate(&self, at: usize, letters: bool) -> (f64, usize, usize) {
        let pick = |f: &(u32, u32)| if letters { f.0 as usize } else { f.1 as usize };
        let total = if letters { self.letters } else { self.words };
        for (k, field) in self.fields.iter().enumerate() {
            let start = pick(field);
            let end = self.fields.get(k + 1).map_or(total, pick);
            if start <= at && at < end {
                return (self.ln_weights[k], at - start, end - start);
            }
        }
        (self.ln_weights[0], at, total)
    }
}

/// The statistics of the index needed by [`WordWeight::new`].
#[derive(Clone, Copy)]
pub(crate) struct Stats<'a> {
    pub entries: usize,
    pub mean_letters: f64,
    pub mean_words: f64,
    pub frequencies: &'a [f64; 256],
}

/// `P(Bin(m, ρ) = d)` for d = 0..=MAX_ERRORS, and `P(Bin(m, ρ) ≤ d)`.
fn typos(m: usize, rho: f64) -> ([f64; MAX_ERRORS + 1], [f64; MAX_ERRORS + 1]) {
    let mut exactly = [0.0; MAX_ERRORS + 1];
    let mut at_most = [0.0; MAX_ERRORS + 1];
    let mut term = (1.0 - rho).powi(m as i32);
    let mut sum = 0.0;
    for j in 0..=MAX_ERRORS {
        if j <= m {
            exactly[j] = term;
            sum += term;
            term *= (m - j) as f64 / (j + 1) as f64 * rho / (1.0 - rho);
        }
        at_most[j] = sum.min(1.0);
    }
    (exactly, at_most)
}

/// The probability, from the letter frequencies alone, that a random position starts a piece
/// within d errors of the word: exact for d = 0, for d > 0 an overestimate (any d letters of
/// the word replaced, in one of three ways).
fn by_chance(word: &[u8], frequencies: &[f64; 256]) -> [f64; MAX_ERRORS + 1] {
    let m = word.len();
    let ln_exact: f64 = word.iter().map(|&b| frequencies[b as usize].ln()).sum();
    let ln_mean = ln_exact / m as f64;
    let mut out = [0.0; MAX_ERRORS + 1];
    let mut sum = 0.0;
    let mut combinations = 1.0;
    for (j, f) in out.iter_mut().enumerate() {
        if j <= m {
            sum += combinations * (3.0f64.ln() * j as f64 - ln_mean * j as f64).exp();
            combinations *= (m - j) as f64 / (j + 1) as f64;
        }
        *f = (ln_exact.exp() * sum).min(1.0);
    }
    out
}

/// From the fraction of entries containing the word to a rate per position: with independent
/// positions, `P(at least one in n) = 1 − (1 − r)^n`.
fn rate(fraction: f64, positions: f64) -> f64 {
    let r = -(-fraction).ln_1p() / positions.max(1.0);
    r.min(0.99)
}

/// The logarithm of the probability that the first chance piece falls at place `i`, with
/// probability `r` at each place: `r (1 − r)^i`.
fn ln_first(r: f64, i: usize) -> f64 {
    r.ln() + i as f64 * (-r).ln_1p()
}

/// How much a query word weighs, estimated on the whole index.
pub(crate) struct WordWeight {
    errors: usize,
    /// `ln P(typos = d)`: a piece at distance d is the wanted word typed with d typos.
    ln_typos: [f64; MAX_ERRORS + 1],
    /// For each `d`: the probability that a position of the text starts, by chance, a piece
    /// within d errors. It is measured inside words, where a piece resembling the query word is
    /// almost never the word itself: so chance has the real structure of the language ("on"
    /// inside "condition") without the genuine occurrences of the word.
    r: [f64; MAX_ERRORS + 1],
    /// The share of positions where a word ends: a chance piece that starts a word also ends it
    /// with this probability.
    end: f64,
    /// `log10 Λ` when the word is missing.
    log_missing: f64,
    missing_word: f64,
}

impl WordWeight {
    pub(crate) fn new(
        word: &[u8],
        errors: usize,
        inner_counts: &[usize; MAX_ERRORS + 1],
        stats: &Stats<'_>,
        constants: &Constants,
    ) -> WordWeight {
        let (exactly, at_most) = typos(word.len(), constants.typo_rate);
        let chance = by_chance(word, stats.frequencies);
        let entries = stats.entries as f64 + IMAGINARY_ENTRIES;
        let highest = 1.0 - 0.5 / entries;
        let positions = (stats.mean_letters - stats.mean_words).max(1.0);
        let mut r = [1.0; MAX_ERRORS + 1];
        for d in 0..=errors {
            let expected = -(-chance[d] * positions).exp_m1();
            let fraction = ((inner_counts[d] as f64 + IMAGINARY_ENTRIES * expected) / entries).clamp(1e-300, highest);
            r[d] = rate(fraction, positions);
        }
        WordWeight {
            errors,
            ln_typos: exactly.map(f64::ln),
            r,
            end: (stats.mean_words / stats.mean_letters.max(1.0)).clamp(0.01, 1.0),
            log_missing: (constants.missing_word + (1.0 - constants.missing_word) * (1.0 - at_most[errors])).log10(),
            missing_word: constants.missing_word,
        }
    }

    /// `log10(η + (1 − η) e^x)`.
    fn log_lr(&self, x: f64) -> f64 {
        if x > 40.0 {
            ((-self.missing_word).ln_1p() + x) / LN_10
        } else {
            (self.missing_word + (1.0 - self.missing_word) * x.exp()).log10()
        }
    }

    /// `log10 Λ` of the word in the entry, and whether the word is there (within tolerance). For
    /// each piece found, `Λ = η + (1 − η) G / C`: G is the probability of finding it there if
    /// the entry is the wanted one (typos, kind, depth), C that of finding it there by chance.
    /// The best piece counts.
    pub(crate) fn score(&self, o: &Occurrences, shape: &Shape<'_>, depth: &Depth) -> (f64, bool) {
        if !o.found() {
            return (self.log_missing, false);
        }
        let mut best = self.log_missing;
        // The weight of the field and the depth inside it.
        let place = |at: usize, letters: bool| {
            let (ln_field, inside, length) = shape.locate(at, letters);
            ln_field + depth.ln_weight(inside, length)
        };
        // A place equal to the one for d − 1 is the same piece, already counted with fewer errors.
        let new = |a: &[u16; MAX_ERRORS + 1], d: usize| a[d] != NONE && (d == 0 || a[d] != a[d - 1]);
        for d in 0..=self.errors {
            let r = self.r[d];
            let typos = self.ln_typos[d];
            if new(&o.whole, d) {
                let i = o.whole[d] as usize;
                let g = typos + (AT_WORD_START * (1.0 - PARTIAL)).ln() + place(i, false);
                best = best.max(self.log_lr(g - ln_first(r * self.end, i)));
            }
            if new(&o.prefix, d) {
                let i = o.prefix[d] as usize;
                let g = typos + (AT_WORD_START * PARTIAL).ln() + place(i, false);
                best = best.max(self.log_lr(g - ln_first(r * (1.0 - self.end), i)));
            }
            if new(&o.inner, d) {
                let s = o.inner[d] as usize;
                let g = typos + INNER.ln() + place(s, true);
                best = best.max(self.log_lr(g - ln_first(r, s)));
            }
            if new(&o.spanning, d) {
                let s = o.spanning[d] as usize;
                let g = typos + SPANNING.ln() + place(s, true);
                best = best.max(self.log_lr(g - ln_first(r, s)));
            }
        }
        (best, true)
    }
}

/// π, the share of relevant entries: the larger of the maximum-likelihood estimate
/// ([`prior_ml`]) and the share of entries with Λ ≥ 1, those whose evidence leans towards
/// relevance. When the model underestimates the Λ of relevant entries (two-letter, ambiguous
/// queries) the first collapses towards 1/N and good entries would be excluded; the second still
/// counts them. A larger π can only remove exclusions.
pub(crate) fn prior(log_lr: &[f64]) -> f64 {
    if log_lr.is_empty() {
        return 1.0;
    }
    let favourable = log_lr.iter().filter(|&&l| l >= 0.0).count() as f64 / log_lr.len() as f64;
    prior_ml(log_lr).max(favourable)
}

/// π by maximum likelihood: the π in `[1/N, 1]` that maximises `Σ ln(π Λ_i + 1 − π)`. The
/// function is concave, so bisection on the derivative is enough.
pub(crate) fn prior_ml(log_lr: &[f64]) -> f64 {
    let n = log_lr.len();
    if n == 0 {
        return 1.0;
    }
    let lowest = 1.0 / n as f64;
    // Entries with huge or tiny Λ weigh 1/π and −1/(1 − π), up to 1e-8; the others are counted
    // in bins 0.001 wide in log10 Λ (Λ is off by at most 0.23%, π by less).
    let (mut high, mut low) = (0.0f64, 0.0f64);
    let mut bins = vec![0u32; 16_001];
    for &l in log_lr {
        if l > 8.0 {
            high += 1.0;
        } else if l < -8.0 {
            low += 1.0;
        } else {
            bins[((l + 8.0) * 1000.0).round() as usize] += 1;
        }
    }
    let groups: Vec<(f64, f64)> = bins
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .map(|(c, &n)| (10f64.powf(c as f64 / 1000.0 - 8.0) - 1.0, f64::from(n)))
        .collect();
    let derivative = |pi: f64| {
        let mut g = high / pi - if low > 0.0 { low / (1.0 - pi) } else { 0.0 };
        for &(x, n) in &groups {
            g += n * x / (1.0 + pi * x);
        }
        g
    };
    if low == 0.0 && derivative(1.0) >= 0.0 {
        return 1.0;
    }
    if derivative(lowest) <= 0.0 {
        return lowest;
    }
    let (mut below, mut above) = (lowest.ln(), 0.0f64);
    for _ in 0..60 {
        let middle = 0.5 * (below + above);
        if derivative(middle.exp()) > 0.0 {
            below = middle;
        } else {
            above = middle;
        }
    }
    (0.5 * (below + above)).exp()
}

/// `P(relevant | Λ)` with share `π`.
pub(crate) fn posterior(log_lr: f64, pi: f64) -> f64 {
    if pi >= 1.0 {
        return 1.0;
    }
    let ln_odds = (pi / (1.0 - pi)).ln() + log_lr * LN_10;
    1.0 / (1.0 + (-ln_odds).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typos_are_binomial() {
        let (exactly, at_most) = typos(8, 0.03);
        assert!((exactly[0] - 0.97f64.powi(8)).abs() < 1e-12);
        assert!((exactly[1] - 8.0 * 0.03 * 0.97f64.powi(7)).abs() < 1e-12);
        assert!((at_most[1] - exactly[0] - exactly[1]).abs() < 1e-12);
        assert_eq!(typos(1, 0.03).1[3], 1.0);
        assert_eq!(typos(1, 0.03).0[2], 0.0);
    }

    #[test]
    fn depth_weights_sum_to_one() {
        let p = Depth::new(1.0, 300);
        let sum: f64 = (0..120).map(|x| p.ln_weight(x, 120).exp()).sum();
        assert!((sum - 1.0).abs() < 1e-9);
        // Beyond the table, the integral stays close to the sum.
        let short = Depth::new(0.7, 100);
        let long = Depth::new(0.7, 5000);
        assert!((short.ln_sum(5000) - long.ln_sum(5000)).abs() < 1e-2);
    }

    #[test]
    fn the_prior_is_the_true_one() {
        // 1000 entries: 50 with Λ = 10^4, the others with Λ = 10^-3. True π ≈ 0.05.
        let mut l = vec![4.0; 50];
        l.extend(std::iter::repeat_n(-3.0, 950));
        let pi = prior_ml(&l);
        assert!((pi - 0.05).abs() < 0.002, "{pi}");
        assert_eq!(prior(&l), pi.max(0.05));
        // No entry more likely relevant than not: π at its minimum.
        assert_eq!(prior(&[-1.0; 10]), 0.1);
        // Modest but favourable Λ: π does not drop below their share.
        let mut l = vec![0.5; 100];
        l.extend(std::iter::repeat_n(-1.5, 900));
        assert!(prior(&l) >= 0.1);
        // All very likely: π = 1.
        assert_eq!(prior(&[3.0; 10]), 1.0);
        assert_eq!(posterior(-50.0, 1.0), 1.0);
        assert!((posterior(0.0, 0.5) - 0.5).abs() < 1e-12);
    }
}
