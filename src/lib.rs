//! Approximate search in a list of strings.
//!
//! - Each query word is searched on its own, anywhere in the text and in any order, with a few
//!   errors at most (Damerau-Levenshtein distance: insertions, deletions, substitutions, swaps of
//!   two adjacent letters).
//! - Case, accents and separators do not matter: "multi-layer", "Multi Layer" and "multilayer"
//!   are the same thing, in the text and in the query.
//! - The score of an entry is the base-10 logarithm of a likelihood ratio: how much more likely
//!   what was found is if the entry is the wanted one than if it is any entry. It drops with the
//!   depth of the key in the text, with the errors, and with how easy it is to find that piece by
//!   chance (measured on the list itself).
//! - Each entry also gets a probability of being relevant, estimated on the list itself; the
//!   filter only excludes entries whose probability is below a threshold ε (1% by default). With
//!   ε = 1% the share of relevant entries among the excluded ones stayed below 1% in every test
//!   (see the README); much smaller probabilities are conservative only on average, so a much
//!   stricter threshold does not carry the same guarantee.
//!
//! ```
//! use probable::{Index, Options};
//!
//! let index = Index::new(["Multilayer pipe 20 mm", "Brass T fitting", "Ball valve"]);
//! let hits = index.search("brass fiting", &Options::default());
//! assert_eq!(hits[0].index, 1);
//! ```
//!
//! The reasons behind the formulas are in the README.

mod fold;
mod model;
mod myers;
mod table;

use std::collections::HashMap;

use fold::UNKNOWN;
use model::{Constants, Depth, MAX_ERRORS, NONE, Occurrences, Shape, Stats, WordWeight};
use myers::Pattern;

/// How to search.
#[derive(Clone, Debug)]
pub struct Options {
    /// The maximum number of errors per word (0..=3). Short words get fewer, as in Lucene: none
    /// up to 2 letters, one up to 5, two up to 9, then three.
    pub max_errors: u8,
    /// How much depth matters (β ≥ 0): a key starting at word `i` of the text (0 is the first)
    /// is worth `(i + 1)^β` times less than one at the start. 0: depth does not matter.
    pub depth: f64,
    /// The filter threshold ε: entries whose probability of being relevant is below ε are
    /// excluded. `None`: no filter, every entry with at least one word is kept. Tested with
    /// ε = 1%; see the README before choosing a much smaller one.
    pub exclude_below: Option<f64>,
    /// Keep only the entries that have every word (within tolerance), like a classic "and"
    /// search; without it, an entry missing one word may stay if the others are strong enough.
    pub require_all_words: bool,
    /// ρ: the probability of mistyping a letter.
    pub typo_rate: f64,
    /// η: the probability that the wanted entry lacks one of the typed words.
    pub missing_word: f64,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            max_errors: 2,
            depth: 1.0,
            exclude_below: Some(0.01),
            require_all_words: false,
            typo_rate: 0.03,
            missing_word: 0.01,
        }
    }
}

/// An entry found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Its position in the list given to [`Index::new`].
    pub index: usize,
    /// `log10` of the likelihood ratio: 0 means the entry is neither more nor less likely than
    /// the others, 3 a thousand times more likely, −2 a hundred times less.
    pub score: f64,
    /// The probability that it is relevant.
    pub probability: f64,
}

/// The strings, ready to be searched: folded once.
pub struct Index {
    /// The folded texts, one after the other.
    text: Vec<u8>,
    /// Where each text starts in `text`; one more at the end.
    starts: Vec<u32>,
    /// One bit per letter of `text`: a word starts here.
    word_starts: Vec<u64>,
    /// The number of words of each text.
    word_counts: Vec<u32>,
    /// The codes of non-Latin letters.
    alphabet: HashMap<char, u8>,
    /// The frequency of each byte in `text`.
    frequencies: [f64; 256],
    mean_letters: f64,
    mean_words: f64,
    longest: usize,
    /// The entries from the shortest to the longest: they are scanned in groups of similar
    /// length.
    by_length: Vec<u32>,
    /// For each entry, the letters it has: one bit per [`letter_bit`].
    letter_sets: Vec<u64>,
    /// How many fields each entry has, and where each starts in its entry: (letter, word).
    field_count: usize,
    field_starts: Vec<(u32, u32)>,
    /// For each field, the logarithm of its share of the weights.
    field_ln_weights: Vec<f64>,
}

/// The bit of a letter in the set of letters of an entry: a–z, 0–9, and the other codes spread
/// over the remaining bits (two letters may share a bit: the set only tells which letters an
/// entry certainly lacks).
fn letter_bit(b: u8) -> u64 {
    1 << match b {
        b'a'..=b'z' => b - b'a',
        b'0'..=b'9' => 26 + b - b'0',
        _ => 36 + b % 28,
    }
}

/// The errors allowed for a word of `m` letters.
fn tolerance(m: usize, max_errors: u8) -> usize {
    let by_length = match m {
        0..=2 => 0,
        3..=5 => 1,
        6..=9 => 2,
        _ => 3,
    };
    by_length.min(max_errors as usize).min(MAX_ERRORS)
}

fn clamp_u16(x: usize) -> u16 {
    x.min(NONE as usize - 1) as u16
}

/// `log10` of the probability of typing two words without the space between them.
const MISSING_SPACE: f64 = -1.3;

/// How many entries are scanned interleaved.
const LANES: usize = 6;

/// An entry while it is being scanned for a word.
struct Scan {
    /// Where it starts in `Index::text`, and how many letters it has.
    start: usize,
    letters: usize,
    occurrences: Occurrences,
    /// For each kind (whole, prefix, inner, spanning): the d already filled are d ≥ unfilled.
    unfilled: [usize; 4],
}

/// Above this amount of text the search is split across the processors.
const PARALLEL_MIN: usize = 1 << 18;

impl Index {
    pub fn new<I, S>(texts: I) -> Index
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Index::with_fields(texts.into_iter().map(|t| [t]), &[1.0])
    }

    /// An index of entries made of fields (name, code, description…), given in the same order for
    /// every entry, the most important first. `weights[f]` is how much field `f` counts: a key
    /// found in a field of weight 2 is worth twice one found, at the same depth, in a field of
    /// weight 1. Depth starts again in each field, so a long name does not push the code down.
    /// An entry with fewer fields has the others empty.
    ///
    /// # Panics
    ///
    /// If there are no weights, a weight is not positive and finite, or an entry has more fields
    /// than weights.
    pub fn with_fields<I, R, S>(records: I, weights: &[f64]) -> Index
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        assert!(!weights.is_empty(), "at least one field");
        assert!(weights.iter().all(|w| w.is_finite() && *w > 0.0), "weights must be positive");
        let total: f64 = weights.iter().sum();
        let mut index = Index {
            text: Vec::new(),
            starts: vec![0],
            word_starts: Vec::new(),
            word_counts: Vec::new(),
            alphabet: HashMap::new(),
            frequencies: [0.0; 256],
            mean_letters: 0.0,
            mean_words: 0.0,
            longest: 0,
            by_length: Vec::new(),
            letter_sets: Vec::new(),
            field_count: weights.len(),
            field_starts: Vec::new(),
            field_ln_weights: weights.iter().map(|w| (w / total).ln()).collect(),
        };
        let mut next = 128u8;
        for record in records {
            let start = index.text.len();
            let mut position = start;
            let mut words = 0u32;
            let mut fields = 0;
            for field in record {
                assert!(fields < weights.len(), "an entry has more fields than weights");
                fields += 1;
                index.field_starts.push(((position - start) as u32, words));
                let alphabet = &mut index.alphabet;
                let bits = &mut index.word_starts;
                fold::fold(
                    field.as_ref(),
                    &mut index.text,
                    |c| {
                        *alphabet.entry(c).or_insert_with(|| {
                            let code = next;
                            if next < UNKNOWN {
                                next += 1;
                            }
                            code
                        })
                    },
                    |word_start, _| {
                        if word_start {
                            if bits.len() <= position / 64 {
                                bits.resize(position / 64 + 1, 0);
                            }
                            bits[position / 64] |= 1 << (position % 64);
                            words += 1;
                        }
                        position += 1;
                    },
                );
            }
            for _ in fields..weights.len() {
                index.field_starts.push(((position - start) as u32, words));
            }
            index.longest = index.longest.max(position - start);
            index.letter_sets.push(index.text[start..].iter().fold(0, |s, &b| s | letter_bit(b)));
            index.starts.push(u32::try_from(index.text.len()).expect("at most 4 GB of text"));
            index.word_counts.push(words);
        }
        index.word_starts.resize(index.text.len() / 64 + 1, 0);
        let mut order: Vec<u32> = (0..index.word_counts.len() as u32).collect();
        order.sort_by_key(|&i| index.starts[i as usize + 1] - index.starts[i as usize]);
        index.by_length = order;
        let entries = index.word_counts.len().max(1) as f64;
        index.mean_letters = index.text.len() as f64 / entries;
        index.mean_words = index.word_counts.iter().map(|&p| f64::from(p)).sum::<f64>() / entries;
        let mut counts = [0usize; 256];
        for &b in &index.text {
            counts[b as usize] += 1;
        }
        // A letter never seen counts as one seen half a time.
        let total = index.text.len() as f64 + 128.0;
        for (f, &c) in index.frequencies.iter_mut().zip(&counts) {
            *f = (c as f64 + 0.5) / total;
        }
        index.frequencies[UNKNOWN as usize] = 0.5 / total;
        index
    }

    /// How many entries.
    pub fn len(&self) -> usize {
        self.word_counts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.word_counts.is_empty()
    }

    fn is_word_start(&self, position: usize) -> bool {
        self.word_starts[position / 64] >> (position % 64) & 1 == 1
    }

    /// How many words start in `text[from..to]`.
    fn count_word_starts(&self, from: usize, to: usize) -> usize {
        let mut count = 0;
        let mut p = from;
        while p < to {
            let word = p / 64;
            let inside = (to - word * 64).min(64);
            let bits = self.word_starts[word] >> (p % 64);
            let how_many = inside - p % 64;
            let mask = if how_many >= 64 { !0 } else { (1u64 << how_many) - 1 };
            count += (bits & mask).count_ones() as usize;
            p = word * 64 + inside;
        }
        count
    }

    /// The words of the query, folded.
    fn query_words(&self, query: &str) -> Vec<Vec<u8>> {
        query
            .split_whitespace()
            .map(|word| {
                let mut out = Vec::new();
                fold::fold(word, &mut out, |c| self.alphabet.get(&c).copied().unwrap_or(UNKNOWN), |_, _| {});
                out
            })
            .filter(|p| !p.is_empty())
            .collect()
    }

    /// Records in `c` the piece that ends at `j` at distance `d` from the word of `m` letters;
    /// `false` when the entry has nothing more to say.
    #[inline]
    fn record(&self, c: &mut Scan, m: usize, j: usize, d: u32) -> bool {
        let d = d as usize;
        if c.unfilled.iter().all(|&x| d >= x) {
            return true;
        }
        let a = c.start;
        let s = (j + 1).saturating_sub(m);
        // With d errors the piece may start up to d letters earlier or later.
        let near = (0..=d)
            .flat_map(|x| [s.checked_sub(x), Some(s + x)])
            .flatten()
            .find(|&p| p < c.letters && self.is_word_start(a + p));
        let begins = near.unwrap_or(s);
        let spanning = begins < j && self.count_word_starts(a + begins + 1, a + j + 1) > 0;
        let (kind, value) = if spanning {
            (3, begins)
        } else if let Some(p) = near {
            let ends = j + 1 >= c.letters || self.is_word_start(a + j + 1);
            (usize::from(!ends), self.count_word_starts(a, a + p))
        } else {
            (2, s)
        };
        if d < c.unfilled[kind] {
            let places = match kind {
                0 => &mut c.occurrences.whole,
                1 => &mut c.occurrences.prefix,
                2 => &mut c.occurrences.inner,
                _ => &mut c.occurrences.spanning,
            };
            places[d..c.unfilled[kind]].fill(clamp_u16(value));
            c.unfilled[kind] = d;
        }
        // Once the exact whole word is found we stop: the entry is no longer needed to measure
        // chance (see `score_word`), and a piece further in would not be worth more.
        c.unfilled[0] > 0
    }

    fn scan_state(&self, i: usize, errors: usize) -> Scan {
        let start = self.starts[i] as usize;
        Scan {
            start,
            letters: self.starts[i + 1] as usize - start,
            occurrences: Occurrences::EMPTY,
            unfilled: [errors + 1; 4],
        }
    }

    /// Where the word occurs in each of the entries `which`.
    fn occurrences_in(
        &self,
        which: &[u32],
        word: &[u8],
        pattern: Option<&Pattern>,
        errors: usize,
    ) -> Vec<(u32, Occurrences)> {
        let m = word.len();
        let k = errors as u32;
        let mut out = Vec::with_capacity(which.len());
        // Each letter of the word that the entry lacks costs at least one error: with more than
        // k missing letters the entry cannot match, and it is not scanned.
        let mut letters: Vec<(u64, u32)> = Vec::new();
        let mut unknown = 0u32;
        for &b in word {
            if b == UNKNOWN {
                unknown += 1;
            } else if let Some(x) = letters.iter_mut().find(|x| x.0 == letter_bit(b)) {
                x.1 += 1;
            } else {
                letters.push((letter_bit(b), 1));
            }
        }
        let kept: Vec<u32> = which
            .iter()
            .copied()
            .filter(|&i| {
                let has = self.letter_sets[i as usize];
                unknown + letters.iter().filter(|x| has & x.0 == 0).map(|x| x.1).sum::<u32>() <= k
            })
            .collect();
        let which = &kept[..];
        let Some(pattern) = pattern else {
            for &i in which {
                let mut c = self.scan_state(i as usize, errors);
                let text = &self.text[c.start..c.start + c.letters];
                myers::scan_slow(word, text, k, |j, d| self.record(&mut c, m, j, d));
                out.push((i, c.occurrences));
            }
            return out;
        };
        // In groups of LANES entries of similar length, interleaved: each letter depends on the
        // previous one, but the entries are independent and the processor advances them together.
        let (groups, rest) = which.as_chunks::<LANES>();
        for group in groups {
            let mut scans: [Scan; LANES] = std::array::from_fn(|l| self.scan_state(group[l] as usize, errors));
            let mut states = [pattern.start(); LANES];
            let mut alive = [true; LANES];
            let shortest = scans.iter().map(|v| v.letters).min().unwrap_or(0);
            let texts: [&[u8]; LANES] = std::array::from_fn(|l| &self.text[scans[l].start..scans[l].start + shortest]);
            for j in 0..shortest {
                for l in 0..LANES {
                    // SAFETY: every text has `shortest` letters and j < shortest.
                    let c = unsafe { *texts[l].get_unchecked(j) };
                    pattern.step(&mut states[l], c);
                }
                for l in 0..LANES {
                    if states[l].distance <= k && alive[l] {
                        alive[l] = self.record(&mut scans[l], m, j, states[l].distance);
                    }
                }
            }
            for l in 0..LANES {
                let c = &mut scans[l];
                let mut state = states[l];
                if alive[l] {
                    for j in shortest..c.letters {
                        pattern.step(&mut state, self.text[c.start + j]);
                        if state.distance <= k && !self.record(c, m, j, state.distance) {
                            break;
                        }
                    }
                }
                out.push((group[l], c.occurrences));
            }
        }
        for &i in rest {
            let mut c = self.scan_state(i as usize, errors);
            let text = &self.text[c.start..c.start + c.letters];
            pattern.scan(text, k, |j, d| self.record(&mut c, m, j, d));
            out.push((i, c.occurrences));
        }
        out
    }

    /// How many processors to use for `entries` entries: one when there is little text.
    fn threads(&self, entries: usize) -> usize {
        // The mean length is enough: only the order of magnitude matters.
        if (entries as f64 * self.mean_letters) < PARALLEL_MIN as f64 {
            return 1;
        }
        std::thread::available_parallelism().map_or(1, |n| n.get()).min(16)
    }

    /// Where the word occurs in the entries of `order` (sorted by length), indexed by entry.
    fn occurrences(&self, word: &[u8], errors: usize, order: &[u32]) -> Vec<Occurrences> {
        let pattern = (word.len() <= 64).then(|| Pattern::new(word));
        let pattern = pattern.as_ref();
        let mut all = vec![Occurrences::EMPTY; self.len()];
        let threads = self.threads(order.len());
        let parts: Vec<Vec<(u32, Occurrences)>> = if threads == 1 {
            vec![self.occurrences_in(order, word, pattern, errors)]
        } else {
            let per_part = order.len().div_ceil(threads * 4).max(LANES);
            let chunks: Vec<&[u32]> = order.chunks(per_part).collect();
            let next = std::sync::atomic::AtomicUsize::new(0);
            std::thread::scope(|s| {
                let workers: Vec<_> = (0..threads)
                    .map(|_| {
                        s.spawn(|| {
                            let mut mine = Vec::new();
                            loop {
                                let n = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let Some(chunk) = chunks.get(n) else { break };
                                mine.push(self.occurrences_in(chunk, word, pattern, errors));
                            }
                            mine
                        })
                    })
                    .collect();
                workers.into_iter().flat_map(|w| w.join().unwrap()).collect()
            })
        };
        for (i, o) in parts.into_iter().flatten() {
            all[i as usize] = o;
        }
        all
    }

    /// Where to split `word` (2..=64 letters) if it is two words typed without the space: the cut
    /// for which the most entries of `pool` contain, each at the start of one of their words,
    /// both the piece before and the piece after, exactly; on a tie the one closest to the
    /// middle. Two Shift-And passes per entry give at once every prefix and every suffix of the
    /// word that occurs.
    fn split_point(&self, word: &[u8], pool: &[u32]) -> Option<usize> {
        let m = word.len();
        let mut forward = [0u64; 256];
        let mut backward = [0u64; 256];
        for (i, &b) in word.iter().enumerate() {
            if b != UNKNOWN {
                forward[b as usize] |= 1 << i;
                backward[b as usize] |= 1 << (m - 1 - i);
            }
        }
        let mut counts = vec![0usize; m];
        for &v in pool {
            let v = v as usize;
            let (a, b) = (self.starts[v] as usize, self.starts[v + 1] as usize);
            // Bit i of `prefixes`: the first i + 1 letters occur from a word start.
            let (mut state, mut prefixes) = (0u64, 0u64);
            for p in a..b {
                state = ((state << 1) | u64::from(self.is_word_start(p))) & forward[self.text[p] as usize];
                prefixes |= state;
            }
            if prefixes == 0 {
                continue;
            }
            // Bit i of `suffixes`: the last i + 1 letters occur from a word start.
            let (mut state, mut suffixes) = (0u64, 0u64);
            for p in (a..b).rev() {
                state = ((state << 1) | 1) & backward[self.text[p] as usize];
                if self.is_word_start(p) {
                    suffixes |= state;
                }
            }
            for (cut, count) in counts.iter_mut().enumerate().take(m - 1).skip(2) {
                if prefixes >> (cut - 1) & 1 == 1 && suffixes >> (m - cut - 1) & 1 == 1 {
                    *count += 1;
                }
            }
        }
        (2..=m - 2)
            .filter(|&t| counts[t] > 0)
            .max_by_key(|&t| (counts[t], std::cmp::Reverse(t.abs_diff(m - t))))
    }

    /// `log10 Λ` of the word in each entry of `pool` (in the same order), whether it is there,
    /// and whether a word somewhere starts with it with at most one error.
    fn score_word(&self, word: &[u8], query: &Query<'_>) -> (Vec<f64>, Vec<bool>, bool) {
        let errors = tolerance(word.len(), query.options.max_errors);
        let all = self.occurrences(word, errors, query.order);
        // Chance is measured on the entries that lack the exact whole word: those that have it
        // are almost all relevant, and would skew the measure.
        let mut inner_counts = [0usize; MAX_ERRORS + 1];
        let mut without_word = 0;
        let one = errors.min(1);
        let mut good = false;
        for &i in query.pool {
            let o = &all[i as usize];
            good |= o.whole[one] != NONE || o.prefix[one] != NONE;
            if o.whole[0] != NONE {
                continue;
            }
            without_word += 1;
            for (count, &place) in inner_counts.iter_mut().zip(&o.inner).take(errors + 1) {
                *count += usize::from(place != NONE);
            }
        }
        let stats = Stats {
            entries: without_word,
            ..query.stats
        };
        let weight = WordWeight::new(word, errors, &inner_counts, &stats, &query.constants);
        let mut scores = vec![0.0f64; query.pool.len()];
        let mut found = vec![false; query.pool.len()];
        let score = |from: usize, scores: &mut [f64], found: &mut [bool]| {
            for (k, (s, f)) in scores.iter_mut().zip(found.iter_mut()).enumerate() {
                let i = query.pool[from + k] as usize;
                let shape = Shape {
                    letters: (self.starts[i + 1] - self.starts[i]) as usize,
                    words: self.word_counts[i] as usize,
                    fields: &self.field_starts[i * self.field_count..(i + 1) * self.field_count],
                    ln_weights: &self.field_ln_weights,
                };
                (*s, *f) = weight.score(&all[i], &shape, &query.depth);
            }
        };
        let threads = self.threads(query.pool.len());
        if threads == 1 {
            score(0, &mut scores, &mut found);
        } else {
            let per_part = query.pool.len().div_ceil(threads);
            std::thread::scope(|s| {
                for (n, (sc, f)) in scores.chunks_mut(per_part).zip(found.chunks_mut(per_part)).enumerate() {
                    let score = &score;
                    s.spawn(move || score(n * per_part, sc, f));
                }
            });
        }
        (scores, found, good)
    }

    /// The entries matching `query`, most relevant first; on equal score in the given order. With
    /// an empty query, all of them, in order.
    pub fn search(&self, query: &str, options: &Options) -> Vec<Hit> {
        let pool: Vec<u32> = (0..self.len() as u32).collect();
        self.run(query, options, &pool, &self.by_length)
    }

    /// Like [`Index::search`], but only among the entries `among` (positions in the list given to
    /// [`Index::new`]): on equal score they keep the order of `among`, and with an empty query
    /// they come back in that order. Probabilities are estimated on `among` alone. Positions out
    /// of range and repeated ones are ignored.
    pub fn search_among(&self, query: &str, among: &[usize], options: &Options) -> Vec<Hit> {
        let mut seen = vec![false; self.len()];
        let pool: Vec<u32> = among
            .iter()
            .filter(|&&i| i < self.len() && !std::mem::replace(&mut seen[i], true))
            .map(|&i| i as u32)
            .collect();
        let mut order = pool.clone();
        order.sort_by_key(|&i| self.starts[i as usize + 1] - self.starts[i as usize]);
        self.run(query, options, &pool, &order)
    }

    /// The search among the entries of `pool`; `order` is the same entries sorted by length.
    fn run(&self, query: &str, options: &Options, pool: &[u32], order: &[u32]) -> Vec<Hit> {
        let words = self.query_words(query);
        if words.is_empty() {
            return pool
                .iter()
                .map(|&i| Hit {
                    index: i as usize,
                    score: 0.0,
                    probability: 1.0,
                })
                .collect();
        }
        let constants = Constants {
            depth: options.depth.max(0.0),
            typo_rate: options.typo_rate.clamp(1e-6, 0.5),
            missing_word: options.missing_word.clamp(1e-12, 0.5),
        };
        let q = Query {
            options,
            pool,
            order,
            stats: Stats {
                entries: pool.len(),
                mean_letters: self.mean_letters,
                mean_words: self.mean_words,
                frequencies: &self.frequencies,
            },
            constants,
            depth: Depth::new(constants.depth, self.longest + 1),
        };
        let mut totals = vec![0.0f64; pool.len()];
        let mut found_words = vec![0usize; pool.len()];
        for word in &words {
            let (mut scores, mut found, good) = self.score_word(word, &q);
            // A word found well nowhere may be two words typed without the space between them:
            // try splitting it, and keep the split when it beats the whole word.
            if !good
                && (5..=64).contains(&word.len())
                && let Some(a) = self.split_point(word, pool)
            {
                let (s1, f1, _) = self.score_word(&word[..a], &q);
                let (s2, f2, _) = self.score_word(&word[a..], &q);
                for k in 0..scores.len() {
                    scores[k] = scores[k].max(s1[k] + s2[k] + MISSING_SPACE);
                    found[k] |= f1[k] && f2[k];
                }
            }
            for k in 0..scores.len() {
                totals[k] += scores[k];
                found_words[k] += usize::from(found[k]);
            }
        }
        for t in &mut totals {
            *t = t.clamp(-300.0, 300.0);
        }
        let pi = model::prior(&totals);
        let mut out: Vec<(usize, Hit)> = totals
            .iter()
            .zip(&found_words)
            .enumerate()
            .filter(|&(_, (_, &c))| c > 0 && (!options.require_all_words || c == words.len()))
            .map(|(k, (&score, _))| {
                let hit = Hit {
                    index: pool[k] as usize,
                    score,
                    probability: model::posterior(score, pi),
                };
                (k, hit)
            })
            .filter(|(_, h)| options.exclude_below.is_none_or(|eps| h.probability >= eps))
            .collect();
        out.sort_by(|(ka, a), (kb, b)| b.score.total_cmp(&a.score).then(ka.cmp(kb)));
        out.into_iter().map(|(_, h)| h).collect()
    }
}

/// What a search needs for every word: the options, the entries searched (`pool`, in the
/// caller's order; `order`, the same sorted by length), the statistics and the model.
struct Query<'a> {
    options: &'a Options,
    pool: &'a [u32],
    order: &'a [u32],
    stats: Stats<'a>,
    constants: Constants,
    depth: Depth,
}

/// The words of `text` folded the way the search folds them (lowercase, no accents), split at
/// separators: "Città: D.80" → `citta`, `d`, `80`.
pub fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter_map(|piece| {
            let mut out = Vec::new();
            let mut others = Vec::new();
            fold::fold(
                piece,
                &mut out,
                |c| {
                    others.push(c);
                    UNKNOWN
                },
                |_, _| {},
            );
            if out.is_empty() {
                return None;
            }
            // Non-Latin letters come back as themselves.
            let mut others = others.into_iter();
            Some(out.iter().map(|&b| if b == UNKNOWN { others.next().unwrap_or('?') } else { b as char }).collect())
        })
        .collect()
}

/// The letters of `text` (character positions, sorted, without duplicates) that match the words
/// of `query`, for highlighting: for each word the piece with fewest errors, then the one starting
/// a word, then the first.
pub fn highlight(text: &str, query: &str, max_errors: u8) -> Vec<usize> {
    let mut alphabet: HashMap<char, u8> = HashMap::new();
    let mut code = |c: char| {
        let n = alphabet.len();
        *alphabet.entry(c).or_insert(if n < 127 { 128 + n as u8 } else { UNKNOWN })
    };
    let mut folded = Vec::new();
    let mut originals = Vec::new();
    let mut starts = Vec::new();
    fold::fold(text, &mut folded, &mut code, |start, index| {
        originals.push(index);
        starts.push(start);
    });
    let mut out = Vec::new();
    for word in query.split_whitespace() {
        let mut p = Vec::new();
        fold::fold(word, &mut p, &mut code, |_, _| {});
        if p.is_empty() {
            continue;
        }
        let k = tolerance(p.len(), max_errors);
        let m = p.len();
        let mut best: Option<((usize, bool, usize), usize, usize)> = None;
        let visit = |j: usize, d: u32| {
            let d = d as usize;
            // Where it really starts: among the pieces ending at j at distance d, the one starting
            // a word, otherwise the one closest to j + 1 − m.
            let expected = (j + 1).saturating_sub(m);
            let start = (0..=d)
                .flat_map(|x| [expected.checked_sub(x), Some(expected + x)])
                .flatten()
                .filter(|&s| s <= j && myers::distance(&p, &folded[s..=j]) as usize <= d)
                .min_by_key(|&s| (!starts[s], s.abs_diff(expected)));
            if let Some(s) = start {
                let key = (d, !starts[s], s);
                if best.is_none_or(|(c, _, _)| key < c) {
                    best = Some((key, s, j));
                }
            }
            true
        };
        if m <= 64 {
            Pattern::new(&p).scan(&folded, k as u32, visit);
        } else {
            myers::scan_slow(&p, &folded, k as u32, visit);
        }
        if let Some((_, s, j)) = best {
            out.extend(originals[s..=j].iter().copied());
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests;
