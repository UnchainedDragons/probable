//! Approximate search in a list of strings.
//!
//! - Each query word is searched on its own, anywhere in the text and in any order, with a few
//!   errors at most (Damerau-Levenshtein distance: insertions, deletions, substitutions, swaps of
//!   two adjacent letters).
//! - Case, accents and separators do not matter: "multi-layer", "Multi Layer" and "multilayer"
//!   are the same thing, in the text and in the query.
//! - One model of how people type a query (a word of the wanted entry, whole or only begun or a
//!   piece of it, with typos, sometimes without a space or with one too many) gives both numbers
//!   of every entry: the **score**, how much more likely the query is if this is the wanted entry
//!   than if it is any entry, which orders the results; and the **probability** that the entry
//!   has what each typed word stands for, which the filter compares with a threshold ε (1% by
//!   default). Both are estimated on the list itself.
//! - The filter asks whether the entry has the word typed in any of its forms ("tubi" for
//!   "tubo"), with the probability that two forms are the same word ([`Endings`]); the order
//!   stays that of the model.
//!
//! ```
//! use probable::{Index, Options};
//!
//! let index = Index::new(["Multilayer pipe 20 mm", "Brass T fitting", "Ball valve"]);
//! let hits = index.search("brass fiting", &Options::default());
//! assert_eq!(hits[0].index, 1);
//! ```
//!
//! The model and the reasons behind it are in the README.

mod endings;
mod fold;
mod model;
mod myers;
mod table;

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};

use fold::UNKNOWN;
use model::{Channel, Depth, FOREIGN_LISTED, MAX_ERRORS, SPACE, new_string};
use myers::Pattern;

pub use endings::Endings;

/// How to search.
#[derive(Clone, Debug)]
pub struct Options {
    /// The maximum number of errors per word (0..=3). Short words get fewer: none up to 3
    /// letters, one up to 5, two up to 9, then three. (With one error, three letters typed so far
    /// are near the beginning of almost every frequent word, and those take probability from the
    /// right entries: the test bench is better without.)
    pub max_errors: u8,
    /// How much depth matters (β ≥ 0): the word at position `i` of a field (0 is the first) is
    /// `(i + 1)^β` times less likely to be the one typed than the first. 0: depth does not matter.
    pub depth: f64,
    /// The filter threshold ε: entries whose probability is below ε are excluded. `None`: no
    /// filter. Tested with ε = 1%; see the README before choosing a much smaller one.
    pub exclude_below: Option<f64>,
    /// Every typed word must stand for something in the entry, like a classic "and" search. Without
    /// it, a word that is in no entry (or, rarely, in other entries only) may be left out.
    pub require_all_words: bool,
    /// ρ: the probability of mistyping a letter.
    pub typo_rate: f64,
    /// η: the probability that a typed word is not in the wanted entry.
    pub missing_word: f64,
    /// Numbers in the query must appear whole in the entry: "150" does not find "1500", "acm12"
    /// does not find "ACM-18". For lists where numbers identify things (codes, sizes,
    /// measures); typos stay allowed in the letters.
    pub exact_numbers: bool,
    /// The endings of the language of the list: an entry has the word typed also in another form
    /// ("tubi" for "tubo"), with the probability that the two are the same word. Only the filter
    /// uses it, never the order. [`Endings::NONE`]: the same letters are needed.
    pub endings: &'static Endings,
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
            exact_numbers: false,
            endings: &Endings::ITALIAN,
        }
    }
}

/// An entry found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Its position in the list given to [`Index::new`].
    pub index: usize,
    /// `log10` of the likelihood ratio: how much more likely the query is if this is the wanted
    /// entry than if it is any entry. 3: a thousand times more likely; −2: a hundred times less.
    pub score: f64,
    /// The probability that the entry has what each typed word stands for.
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
    /// One bit per letter of `text`: a word starts here after a separator (not at a change
    /// between letters and digits or from lowercase to uppercase).
    hard_starts: Vec<u64>,
    /// The number of words of each text.
    word_counts: Vec<u32>,
    /// The codes of non-Latin letters.
    alphabet: HashMap<char, u8>,
    /// The frequency of each byte in `text`.
    frequencies: [f64; 256],
    /// A: the perplexity of the letters, how many letters a mistake may be.
    letter_choices: f64,
    mean_letters: f64,
    /// The mean length of a word, in letters.
    mean_word: f64,
    longest: usize,
    /// The entries from the shortest to the longest: they are scanned in groups of similar
    /// length.
    by_length: Vec<u32>,
    /// For each entry, the letters it has: one bit per [`letter_bit`].
    letter_sets: Vec<u64>,
    /// The numbers of each entry (runs of digits, field by field), hashed and sorted;
    /// `number_starts` says where each entry's start, one more at the end.
    numbers: Vec<u64>,
    number_starts: Vec<u32>,
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

/// The numbers in a text, hashed (FNV-1a): runs of digits, split by anything else except a dot or
/// a comma between two digits ("12.000" is 12000, "150 1000" are 150 and 1000).
fn numbers(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut hash: Option<u64> = None;
    let mut chars = text.chars().peekable();
    while let Some(x) = chars.next() {
        if x.is_ascii_digit() {
            hash = Some(fnv_step(hash.unwrap_or(FNV_START), x as u8));
        } else if (x == '.' || x == ',') && hash.is_some() && chars.peek().is_some_and(char::is_ascii_digit) {
            // A thousands or decimal separator: the number goes on.
        } else if let Some(h) = hash.take() {
            out.push(h);
        }
    }
    out.extend(hash);
    out
}

const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv_step(hash: u64, b: u8) -> u64 {
    (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
}

/// The FNV-1a hash of some folded letters: what tells pieces of text apart.
fn fnv(letters: &[u8]) -> u64 {
    letters.iter().fold(FNV_START, |h, &b| fnv_step(h, b))
}

/// Keys that are already hashes need no more hashing.
#[derive(Default)]
struct AlreadyHashed(u64);

impl Hasher for AlreadyHashed {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = fnv_step(self.0, b);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = x;
    }
}

type ByKey<V> = HashMap<u64, V, BuildHasherDefault<AlreadyHashed>>;

/// The errors allowed for a word of `m` letters.
fn tolerance(m: usize, max_errors: u8) -> usize {
    let by_length = match m {
        0..=3 => 0,
        4..=5 => 1,
        6..=9 => 2,
        _ => 3,
    };
    by_length.min(max_errors as usize).min(MAX_ERRORS)
}

fn set_bit(bits: &mut Vec<u64>, position: usize) {
    if bits.len() <= position / 64 {
        bits.resize(position / 64 + 1, 0);
    }
    bits[position / 64] |= 1 << (position % 64);
}

/// How many entries are scanned interleaved.
const LANES: usize = 6;

/// Above this amount of text the search is split across the processors.
const PARALLEL_MIN: usize = 1 << 18;

/// A piece of an entry that may be what a query word stands for: a word of the entry, or words
/// typed together, from the word starting at `at`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Piece {
    /// Where it starts in its entry, and how many letters it has.
    at: u32,
    len: u32,
    /// The hash of its letters.
    key: u64,
    /// `ln P(this piece is the one chosen | the entry)`: depth, field, spaces left out.
    ln_chosen: f64,
    /// `ln P(the typed word | this piece)`: whole, begun, from inside; errors.
    ln_typed: f64,
}

/// What the pieces of a query word found in some entries: for entry `entries[x]`, the pieces
/// `pieces[ends[x - 1]..ends[x]]`.
#[derive(Default, Debug, PartialEq)]
struct Found {
    entries: Vec<u32>,
    ends: Vec<u32>,
    pieces: Vec<Piece>,
}

/// A query word ready to be looked for.
struct Word<'a> {
    letters: &'a [u8],
    pattern: Option<Pattern>,
    errors: usize,
    channel: Channel,
    depth: &'a Depth,
    /// Set when some entry has the word exactly, whole or begun, from a word start.
    exact: AtomicBool,
}

/// What a query word (or two typed as one, or half of one) is worth in each entry. Every
/// probability is divided by `P(w)`, the mean of `P(w | entry)` over the entries searched.
struct Worth {
    /// `ln P(w)`.
    ln_any: f64,
    /// `P(w | entry) / P(w)` in the entries where it found nothing: `η P(w | not in the entry)`.
    elsewhere: f64,
    /// The share of that which counts as having what `w` stands for: all of it when not every word
    /// is required, nothing otherwise.
    left_out: f64,
    /// The entries where it found something, by position in the pool.
    found: Vec<Seen>,
    /// For each of them, its distinct pieces: (letters, `(1 − η) Σ P(t | entry) P(w | t)` over
    /// the places with those letters, divided by `P(w)`).
    keys: Vec<(u64, f64)>,
    /// Some entry has the word exactly, whole or begun, from a word start.
    exact: bool,
    /// For each piece found, the other forms of the same word that the word also found, with the
    /// probability that they are the same word ([`Options::endings`]).
    forms: ByKey<Vec<(u64, f64)>>,
}

/// An entry where a word found something.
struct Seen {
    /// Its position in the pool.
    at: u32,
    /// `P(w | entry) / P(w)`.
    given: f64,
    /// Its pieces in `Worth::keys`.
    keys: (u32, u32),
}

/// What a search needs for every word.
struct Query<'a> {
    options: &'a Options,
    /// The entries searched, in the caller's order, and the same sorted by length.
    pool: &'a [u32],
    order: &'a [u32],
    /// The position in `pool` of each entry, when `pool` is not simply every entry in order.
    position: Option<Vec<u32>>,
    depth: Depth,
    rho: f64,
    eta: f64,
}

impl Query<'_> {
    fn position(&self, entry: u32) -> u32 {
        self.position.as_ref().map_or(entry, |p| p[entry as usize])
    }
}

/// The entries where some unit found something, by position in the pool, and for each of them
/// where each unit's findings are in its `found` (none: it found nothing there).
fn found_where(units: &[&Worth]) -> (Vec<u32>, Vec<Vec<Option<u32>>>) {
    let mut touched: Vec<u32> = units.iter().flat_map(|w| w.found.iter().map(|f| f.at)).collect();
    touched.sort_unstable();
    touched.dedup();
    let mut cursors = vec![0usize; units.len()];
    let seen = touched
        .iter()
        .map(|&k| {
            units
                .iter()
                .zip(&mut cursors)
                .map(|(w, c)| {
                    while *c < w.found.len() && w.found[*c].at < k {
                        *c += 1;
                    }
                    (w.found.get(*c).is_some_and(|f| f.at == k)).then_some(*c as u32)
                })
                .collect()
        })
        .collect();
    (touched, seen)
}

/// The most choices worked out per entry and reading; beyond, the least likely pieces of each
/// unit are left out (they are worth little next to the others).
const MOST_CHOICES: usize = 256;

/// The choices of what the units of a reading stand for in an entry: for each unit one of its
/// pieces there, or "not in the entry" when that counts as having it.
#[derive(Default)]
struct Tuples {
    /// For each unit: (key, weight, factor). The weight orders the choices when there are too
    /// many; the factor is what the choice contributes to the product.
    options: Vec<Vec<(u64, f64, f64)>>,
    at: Vec<usize>,
}

impl Tuples {
    /// Calls `visit(hash of reading r and the choice, prior × Π factors)` for every choice. The
    /// factor of a piece is its worth; with `forms`, each piece of the entry counts once as itself
    /// (factor 1) and once as each other form of the same word found by the unit (factor P(same
    /// word)), and "not in the entry" has factor 1: what the entry has, not what it is worth.
    #[allow(clippy::too_many_arguments)]
    fn each(
        &mut self,
        r: usize,
        prior: f64,
        units: &[usize],
        mine: &[Option<u32>],
        worth: &[&Worth],
        forms: bool,
        mut visit: impl FnMut(u64, f64),
    ) {
        self.options.resize_with(units.len(), Vec::new);
        for (o, &u) in self.options.iter_mut().zip(units) {
            o.clear();
            let w = worth[u];
            if let Some(c) = mine[u] {
                let (a, b) = w.found[c as usize].keys;
                for &(key, weight) in &w.keys[a as usize..b as usize] {
                    if forms {
                        o.push((key, weight, 1.0));
                        for &(k, p) in w.forms.get(&key).map_or(&[][..], |f| &f[..]) {
                            o.push((k, weight * p, p));
                        }
                    } else {
                        o.push((key, weight, weight));
                    }
                }
                if forms {
                    // The same piece by two ways: the more likely.
                    o.sort_unstable_by(|x, y| x.0.cmp(&y.0).then(y.2.total_cmp(&x.2)));
                    o.dedup_by_key(|x| x.0);
                }
            }
            if w.left_out > 0.0 {
                // "Not in the entry": key 0.
                o.push((0, w.left_out, if forms { 1.0 } else { w.left_out }));
            }
            if o.is_empty() {
                return;
            }
        }
        let options = &mut self.options[..units.len()];
        let mut count: usize = options.iter().map(Vec::len).product();
        // Too many: first the other forms go, the least likely first, so that what is left is
        // never less than without them; then the least likely pieces.
        while count > MOST_CHOICES {
            let weakest = options
                .iter()
                .enumerate()
                .flat_map(|(u, o)| o.iter().enumerate().map(move |(x, c)| (u, x, c)))
                .filter(|(_, _, c)| c.2 < 1.0 && forms && c.0 != 0)
                .min_by(|a, b| a.2.1.total_cmp(&b.2.1))
                .map(|(u, x, _)| (u, x));
            let Some((u, x)) = weakest else { break };
            count = count / options[u].len() * (options[u].len() - 1);
            options[u].remove(x);
        }
        while count > MOST_CHOICES {
            let o = options.iter_mut().max_by_key(|o| o.len()).unwrap();
            o.sort_unstable_by(|x, y| y.1.total_cmp(&x.1));
            count = count / o.len() * (o.len() - 1);
            o.pop();
        }
        self.at.clear();
        self.at.resize(units.len(), 0);
        loop {
            let mut key = fnv_step(FNV_START, r as u8) ^ (r as u64).rotate_left(32);
            let mut w = prior;
            for (o, &x) in options.iter().zip(&self.at) {
                key = (key ^ o[x].0).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
                w *= o[x].2;
            }
            visit(key, w);
            // The next choice, as an odometer.
            let mut d = 0;
            loop {
                if d == options.len() {
                    return;
                }
                self.at[d] += 1;
                if self.at[d] < options[d].len() {
                    break;
                }
                self.at[d] = 0;
                d += 1;
            }
        }
    }
}

impl Index {
    pub fn new<I, S>(texts: I) -> Index
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Index::with_fields(texts.into_iter().map(|t| [t]), &[1.0])
    }

    /// An index of entries made of fields (name, code, description…), given in the same order for
    /// every entry, the most important first. `weights[f]` is how much field `f` counts: a word in
    /// a field of weight 2 is twice as likely to be the one typed as a word, at the same depth, in
    /// a field of weight 1. Depth starts again in each field, so a long name does not push the
    /// code down. An entry with fewer fields has the others empty.
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
        assert!(
            weights.iter().all(|w| w.is_finite() && *w > 0.0),
            "weights must be positive"
        );
        let total: f64 = weights.iter().sum();
        let mut index = Index {
            text: Vec::new(),
            starts: vec![0],
            word_starts: Vec::new(),
            hard_starts: Vec::new(),
            word_counts: Vec::new(),
            alphabet: HashMap::new(),
            frequencies: [0.0; 256],
            letter_choices: 1.0,
            mean_letters: 0.0,
            mean_word: 1.0,
            longest: 0,
            by_length: Vec::new(),
            letter_sets: Vec::new(),
            numbers: Vec::new(),
            number_starts: vec![0],
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
                if field.as_ref().bytes().any(|b| b.is_ascii_digit()) {
                    index.numbers.extend(numbers(field.as_ref()));
                }
                let alphabet = &mut index.alphabet;
                let (soft, hard) = (&mut index.word_starts, &mut index.hard_starts);
                // The character before: a word start after a skipped one follows a separator.
                let mut before: Option<usize> = None;
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
                    |word_start, character| {
                        if word_start {
                            set_bit(soft, position);
                            if before.is_none_or(|b| character > b + 1) {
                                set_bit(hard, position);
                            }
                            words += 1;
                        }
                        before = Some(character);
                        position += 1;
                    },
                );
            }
            let first = *index.number_starts.last().unwrap() as usize;
            let mine = &mut index.numbers[first..];
            mine.sort_unstable();
            let mut unique = 0;
            for k in 0..mine.len() {
                if k == 0 || mine[k] != mine[unique - 1] {
                    mine[unique] = mine[k];
                    unique += 1;
                }
            }
            index.numbers.truncate(first + unique);
            index.number_starts.push(index.numbers.len() as u32);
            for _ in fields..weights.len() {
                index.field_starts.push(((position - start) as u32, words));
            }
            index.longest = index.longest.max(position - start);
            index
                .letter_sets
                .push(index.text[start..].iter().fold(0, |s, &b| s | letter_bit(b)));
            index
                .starts
                .push(u32::try_from(index.text.len()).expect("at most 4 GB of text"));
            index.word_counts.push(words);
        }
        index.finish();
        index
    }

    /// After the last entry: the order by length, the means and the letter frequencies.
    fn finish(&mut self) {
        self.word_starts.resize(self.text.len() / 64 + 2, 0);
        self.hard_starts.resize(self.text.len() / 64 + 2, 0);
        let mut order: Vec<u32> = (0..self.word_counts.len() as u32).collect();
        order.sort_by_key(|&i| self.starts[i as usize + 1] - self.starts[i as usize]);
        self.by_length = order;
        let entries = self.word_counts.len().max(1) as f64;
        self.mean_letters = self.text.len() as f64 / entries;
        let words: f64 = self.word_counts.iter().map(|&p| f64::from(p)).sum();
        self.mean_word = (self.text.len() as f64 / words.max(1.0)).max(1.0);
        let mut counts = [0usize; 256];
        for &b in &self.text {
            counts[b as usize] += 1;
        }
        // A letter never seen counts as one seen half a time.
        let total = self.text.len() as f64 + 128.0;
        for (f, &c) in self.frequencies.iter_mut().zip(&counts) {
            *f = (c as f64 + 0.5) / total;
        }
        self.frequencies[UNKNOWN as usize] = 0.5 / total;
        let sum: f64 = self.frequencies.iter().sum();
        let entropy: f64 = self
            .frequencies
            .iter()
            .map(|&f| -(f / sum) * (f / sum).ln())
            .sum();
        self.letter_choices = entropy.exp().max(2.0);
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

    fn is_hard_start(&self, position: usize) -> bool {
        self.hard_starts[position / 64] >> (position % 64) & 1 == 1
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
            let mask = if how_many >= 64 {
                !0
            } else {
                (1u64 << how_many) - 1
            };
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
                fold::fold(
                    word,
                    &mut out,
                    |c| self.alphabet.get(&c).copied().unwrap_or(UNKNOWN),
                    |_, _| {},
                );
                out
            })
            .filter(|p| !p.is_empty())
            .collect()
    }

    /// The fields of entry `i`: where each starts in it (letter, word).
    fn fields(&self, i: usize) -> &[(u32, u32)] {
        &self.field_starts[i * self.field_count..(i + 1) * self.field_count]
    }

    /// The field of entry `i` holding letter `at`: its number, where it starts and where it ends.
    fn field_at(&self, i: usize, at: usize) -> (usize, usize, usize) {
        let fields = self.fields(i);
        let letters = (self.starts[i + 1] - self.starts[i]) as usize;
        let f = fields
            .partition_point(|&(s, _)| s as usize <= at)
            .saturating_sub(1);
        // Empty fields start where the next one does: the last of them holds the letter.
        let end = fields.get(f + 1).map_or(letters, |&(s, _)| s as usize);
        (f, fields[f].0 as usize, end)
    }

    /// `ln P(the word of entry i starting at letter at, in field f, is the one chosen)`.
    fn ln_chosen(&self, i: usize, f: usize, field_start: usize, at: usize, depth: &Depth) -> f64 {
        let fields = self.fields(i);
        let a = self.starts[i] as usize;
        let words = fields
            .get(f + 1)
            .map_or(self.word_counts[i], |&(_, w)| w)
            .saturating_sub(fields[f].1) as usize;
        let position = self.count_word_starts(a + field_start, a + at);
        self.field_ln_weights[f] + depth.ln_weight(position, words)
    }

    /// The pieces from the word starting at letter `s` of entry `i`: the word typed whole or begun,
    /// and with the words after it, the spaces left out. `distances` is scratch space.
    fn pieces_from(
        &self,
        i: usize,
        s: usize,
        word: &Word<'_>,
        distances: &mut Vec<u32>,
        out: &mut Vec<Piece>,
    ) {
        let a = self.starts[i] as usize;
        let (f, field_start, field_end) = self.field_at(i, s);
        let m = word.letters.len();
        let k = word.errors as u32;
        let limit = (s + m + word.errors).min(field_end);
        let text = &self.text[a..a + field_end];
        distances.clear();
        if let Some(pattern) = &word.pattern {
            let mut state = pattern.start();
            for &c in &text[s..limit] {
                pattern.step_anchored(&mut state, c);
                distances.push(state.distance);
            }
        } else {
            myers::anchored_slow(word.letters, &text[s..limit], |_, d| distances.push(d));
        }
        let chosen = self.ln_chosen(i, f, field_start, s, word.depth);
        let mut spaces = 0.0;
        let mut begun = u32::MAX;
        let mut emit = |end: usize, whole: u32, begun: u32, spaces: f64| {
            if whole > k && begun > k {
                return;
            }
            if whole == 0 || begun == 0 {
                word.exact.store(true, Ordering::Relaxed);
            }
            let length = end - s;
            let mut typed = 0.0;
            if whole <= k {
                typed += word.channel.whole(length, whole);
            }
            if begun <= k {
                typed += word.channel.beginning(length, begun);
            }
            out.push(Piece {
                at: s as u32,
                len: (end - s) as u32,
                key: fnv(&text[s..end]),
                ln_chosen: chosen + spaces,
                ln_typed: typed.ln(),
            });
        };
        let mut open = false;
        for (offset, &d) in distances.iter().enumerate() {
            let p = s + offset;
            open = !(p + 1 == field_end || self.is_word_start(a + p + 1));
            if open {
                begun = begun.min(d);
            } else {
                emit(p + 1, d, begun, spaces);
                begun = u32::MAX;
                if p + 1 < field_end && self.is_hard_start(a + p + 1) {
                    spaces += SPACE.ln();
                }
            }
        }
        // The last word goes on beyond what was read: it can only have been begun.
        if open && begun <= k {
            let end = (limit..field_end)
                .find(|&p| self.is_word_start(a + p))
                .unwrap_or(field_end);
            emit(end, u32::MAX, begun, spaces);
        }
    }

    /// The piece from inside the word `[start, end)` of entry `i`, if the query word is there.
    fn piece_inside(&self, i: usize, start: usize, end: usize, word: &Word<'_>, out: &mut Vec<Piece>) {
        if end - start < 2 {
            return;
        }
        let a = self.starts[i] as usize;
        let inside = &self.text[a + start + 1..a + end];
        let mut fewest = u32::MAX;
        let mut visit = |_, d: u32| {
            fewest = fewest.min(d);
            true
        };
        match &word.pattern {
            Some(pattern) => pattern.scan(inside, word.errors as u32, &mut visit),
            None => myers::scan_slow(word.letters, inside, word.errors as u32, &mut visit),
        }
        if fewest as usize <= word.errors {
            let (f, field_start, _) = self.field_at(i, start);
            out.push(Piece {
                at: start as u32,
                len: (end - start) as u32,
                key: fnv(&self.text[a + start..a + end]),
                ln_chosen: self.ln_chosen(i, f, field_start, start, word.depth),
                ln_typed: word.channel.inside(end - start, fewest).ln(),
            });
        }
    }

    /// The pieces of entry `i` for the word, given `hits`: the letters where some piece of the
    /// text ending there is within the errors allowed, with how many errors.
    ///
    /// A piece from a word start `s` ending at `j` with `d` errors has `m − d..=m + d` letters, so
    /// `s` is within the errors allowed of `j + 1 − m`; a piece inside a word ending at `j` is at
    /// least as far from the word as the best piece ending there. So every piece within the errors
    /// allowed is found from the hits.
    fn entry_pieces(
        &self,
        i: usize,
        word: &Word<'_>,
        hits: &[(u32, u8)],
        distances: &mut Vec<u32>,
        out: &mut Vec<Piece>,
    ) {
        let a = self.starts[i] as usize;
        let letters = (self.starts[i + 1] as usize) - a;
        let m = word.letters.len();
        let k = word.errors;
        let first = out.len();
        // Word starts are looked at once each; so is each word, for pieces inside it.
        let mut next_start = 0;
        let mut inside_of = usize::MAX;
        let mut word_span = (0, 0);
        for &(j, _) in hits {
            let j = j as usize;
            let from = (j + 1).saturating_sub(m + k).max(next_start);
            let to = (j + 1 + k).saturating_sub(m).min(j);
            for s in from..=to {
                if self.is_word_start(a + s) {
                    self.pieces_from(i, s, word, distances, out);
                }
            }
            next_start = next_start.max(to + 1);
            if j >= word_span.1 {
                let start = (0..=j).rev().find(|&p| self.is_word_start(a + p)).unwrap_or(0);
                let end = (j + 1..letters)
                    .find(|&p| self.is_word_start(a + p))
                    .unwrap_or(letters);
                word_span = (start, end);
            }
            let (start, end) = word_span;
            // A piece inside the word, ending at j with at most k errors, has at least m − k
            // letters after the word start.
            if j - start + k >= m && start != inside_of {
                inside_of = start;
                self.piece_inside(i, start, end, word, out);
            }
        }
        // The same piece in two ways (begun, from inside): either way.
        let mine = &mut out[first..];
        mine.sort_unstable_by_key(|p| (p.at, p.key));
        let mut kept = first;
        for x in first..out.len() {
            if kept > first && out[kept - 1].at == out[x].at && out[kept - 1].key == out[x].key {
                out[kept - 1].ln_typed = model::ln_add(out[kept - 1].ln_typed, out[x].ln_typed);
            } else {
                out[kept] = out[x];
                kept += 1;
            }
        }
        out.truncate(kept);
    }

    /// The pieces of the entries `which` (sorted by length) for the word. With `interleave`, the
    /// entries are scanned in groups of LANES.
    fn scan(&self, which: &[u32], word: &Word<'_>, interleave: bool) -> Found {
        let k = word.errors as u32;
        // Each letter of the word that the entry lacks costs at least one error: with more than
        // k missing letters the entry cannot match, and it is not scanned.
        let mut letters: Vec<(u64, u32)> = Vec::new();
        let mut unknown = 0u32;
        for &b in word.letters {
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
                unknown
                    + letters
                        .iter()
                        .filter(|x| has & x.0 == 0)
                        .map(|x| x.1)
                        .sum::<u32>()
                    <= k
            })
            .collect();
        let mut found = Found::default();
        let mut distances = Vec::new();
        let mut finish = |i: u32, hits: &[(u32, u8)], found: &mut Found| {
            if hits.is_empty() {
                return;
            }
            self.entry_pieces(i as usize, word, hits, &mut distances, &mut found.pieces);
            if found.pieces.len() > found.ends.last().map_or(0, |&e| e as usize) {
                found.entries.push(i);
                found.ends.push(found.pieces.len() as u32);
            }
        };
        let text_of =
            |i: u32| &self.text[self.starts[i as usize] as usize..self.starts[i as usize + 1] as usize];
        let mut hits: [Vec<(u32, u8)>; LANES] = Default::default();
        let (groups, rest) = match (&word.pattern, interleave) {
            (Some(_), true) => kept.as_chunks::<LANES>(),
            _ => (&[][..], &kept[..]),
        };
        if let Some(pattern) = &word.pattern {
            // In groups of LANES entries of similar length, interleaved: each letter depends on
            // the previous one, but the entries are independent and the processor advances them
            // together.
            for group in groups {
                let texts: [&[u8]; LANES] = std::array::from_fn(|l| text_of(group[l]));
                let shortest = texts.iter().map(|t| t.len()).min().unwrap_or(0);
                let mut states = [pattern.start(); LANES];
                for h in &mut hits {
                    h.clear();
                }
                for j in 0..shortest {
                    for l in 0..LANES {
                        // SAFETY: every text has at least `shortest` letters and j < shortest.
                        let c = unsafe { *texts[l].get_unchecked(j) };
                        pattern.step(&mut states[l], c);
                    }
                    for l in 0..LANES {
                        if states[l].distance <= k {
                            hits[l].push((j as u32, states[l].distance as u8));
                        }
                    }
                }
                for l in 0..LANES {
                    let mut state = states[l];
                    for (j, &c) in texts[l].iter().enumerate().skip(shortest) {
                        pattern.step(&mut state, c);
                        if state.distance <= k {
                            hits[l].push((j as u32, state.distance as u8));
                        }
                    }
                    finish(group[l], &hits[l], &mut found);
                }
            }
        }
        for &i in rest {
            let h = &mut hits[0];
            h.clear();
            let mut visit = |j: usize, d: u32| {
                h.push((j as u32, d as u8));
                true
            };
            match &word.pattern {
                Some(pattern) => pattern.scan(text_of(i), k, &mut visit),
                None => myers::scan_slow(word.letters, text_of(i), k, &mut visit),
            }
            let h = std::mem::take(&mut hits[0]);
            finish(i, &h, &mut found);
            hits[0] = h;
        }
        found
    }

    /// How many processors to use for `entries` entries: one when there is little text.
    fn threads(&self, entries: usize) -> usize {
        // The mean length is enough: only the order of magnitude matters.
        if (entries as f64 * self.mean_letters) < PARALLEL_MIN as f64 {
            return 1;
        }
        std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(16)
    }

    /// The pieces of the entries of `order` (sorted by length) for the word, split across the
    /// processors when there is much text.
    fn found(&self, word: &Word<'_>, order: &[u32]) -> Vec<Found> {
        let threads = self.threads(order.len());
        if threads == 1 {
            return vec![self.scan(order, word, true)];
        }
        let per_part = order.len().div_ceil(threads * 4).max(LANES);
        let chunks: Vec<&[u32]> = order.chunks(per_part).collect();
        let next = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|s| {
            let workers: Vec<_> = (0..threads)
                .map(|_| {
                    s.spawn(|| {
                        let mut mine = Vec::new();
                        loop {
                            let n = next.fetch_add(1, Ordering::Relaxed);
                            let Some(chunk) = chunks.get(n) else { break };
                            mine.push((n, self.scan(chunk, word, true)));
                        }
                        mine
                    })
                })
                .collect();
            // In the order of the chunks, whoever scanned them: the sums come out the same.
            let mut parts: Vec<(usize, Found)> =
                workers.into_iter().flat_map(|w| w.join().unwrap()).collect();
            parts.sort_unstable_by_key(|p| p.0);
            parts.into_iter().map(|p| p.1).collect()
        })
    }

    /// What the query word `letters` is worth in each entry searched.
    ///
    /// With the pieces `t` of entry `k`, `P(t | k)` how likely `t` is to be the one chosen and
    /// `P(w | t)` how likely `w` is to be typed for it:
    ///
    /// - `S_k = Σ_t P(t | k) P(w | t)`, and `S` its mean over the `N` entries searched;
    /// - `P(w | not in the entry) = λ S + (1 − λ) P(w as a new string)`;
    /// - `P(w | k) = (1 − η) S_k + η P(w | not in the entry)`, and `P(w)` its mean.
    fn worth(&self, letters: &[u8], query: &Query<'_>) -> Worth {
        let m = letters.len();
        let word = Word {
            letters,
            pattern: (m <= 64).then(|| Pattern::new(letters)),
            errors: tolerance(m, query.options.max_errors),
            channel: Channel::new(m, query.rho, self.letter_choices),
            depth: &query.depth,
            exact: AtomicBool::new(false),
        };
        let parts = self.found(&word, query.order);
        let eta = query.eta;
        let mut found = Vec::new();
        let mut keys: Vec<(u64, f64)> = Vec::new();
        let mut sum = 0.0;
        let mut pieces_letters: ByKey<Vec<u8>> = ByKey::default();
        for part in &parts {
            let mut from = 0;
            for (&i, &end) in part.entries.iter().zip(&part.ends) {
                if !query.options.endings.is_empty() {
                    let a = self.starts[i as usize] as usize;
                    for p in &part.pieces[from..end as usize] {
                        pieces_letters.entry(p.key).or_insert_with(|| {
                            self.text[a + p.at as usize..a + (p.at + p.len) as usize].to_vec()
                        });
                    }
                }
                let first = keys.len();
                keys.extend(
                    part.pieces[from..end as usize]
                        .iter()
                        .map(|p| (p.key, (1.0 - eta) * (p.ln_chosen + p.ln_typed).exp())),
                );
                // The same letters in several places of the entry: one piece, the places added.
                keys[first..].sort_unstable_by_key(|x| x.0);
                let mut kept = first;
                for x in first..keys.len() {
                    if kept > first && keys[kept - 1].0 == keys[x].0 {
                        keys[kept - 1].1 += keys[x].1;
                    } else {
                        keys[kept] = keys[x];
                        kept += 1;
                    }
                }
                keys.truncate(kept);
                let s: f64 = keys[first..].iter().map(|x| x.1).sum();
                sum += s;
                found.push(Seen {
                    at: query.position(i),
                    given: s,
                    keys: (first as u32, kept as u32),
                });
                from = end as usize;
            }
        }
        let n = query.pool.len().max(1) as f64;
        // `sum` already has the factor 1 − η.
        let mean = sum / n / (1.0 - eta);
        let foreign = FOREIGN_LISTED * mean
            + (1.0 - FOREIGN_LISTED) * new_string(letters, &self.frequencies, self.mean_word);
        let not_here = eta * foreign;
        let any = (1.0 - eta) * mean + not_here;
        for x in &mut keys {
            x.1 /= any;
        }
        for f in &mut found {
            f.given = (f.given + not_here) / any;
        }
        found.sort_unstable_by_key(|f| f.at);
        let elsewhere = not_here / any;
        // The other forms of each piece among the pieces found, once for all the entries.
        let mut forms: ByKey<Vec<(u64, f64)>> = ByKey::default();
        for (&key, letters) in &pieces_letters {
            let mut mine: Vec<(u64, f64)> = Vec::new();
            query.options.endings.variants(letters, |other, p| {
                let k = fnv(other);
                if k != key && pieces_letters.contains_key(&k) {
                    mine.push((k, p));
                }
            });
            if !mine.is_empty() {
                mine.sort_unstable_by(|x, y| x.0.cmp(&y.0).then(y.1.total_cmp(&x.1)));
                mine.dedup_by_key(|x| x.0);
                forms.insert(key, mine);
            }
        }
        Worth {
            ln_any: any.ln(),
            elsewhere,
            left_out: if query.options.require_all_words {
                0.0
            } else {
                elsewhere
            },
            found,
            keys,
            exact: word.exact.load(Ordering::Relaxed),
            forms,
        }
    }

    /// Where to split `word` (4..=64 letters) if it is two words of an entry typed without the
    /// space: of all the cuts, the one for which the most entries of `pool` contain, each at the
    /// start of one of their words, both the piece before and the piece after, exactly; on a tie
    /// the one closest to the middle. Only this reading is worked out: the others are worth less.
    /// Two Shift-And passes per entry give at once every prefix and every suffix of the word that
    /// occurs.
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

    /// The entries matching `query`, most relevant first; on equal score in the given order. With
    /// an empty query, all of them, in order.
    pub fn search(&self, query: &str, options: &Options) -> Vec<Hit> {
        let pool: Vec<u32> = (0..self.len() as u32).collect();
        self.run(query, options, &pool, &self.by_length, None)
    }

    /// Like [`Index::search`], but only among the entries `among` (positions in the list given to
    /// [`Index::new`]): on equal score they keep the order of `among`, and with an empty query
    /// they come back in that order. Probabilities are estimated on `among` alone. Positions out
    /// of range and repeated ones are ignored.
    pub fn search_among(&self, query: &str, among: &[usize], options: &Options) -> Vec<Hit> {
        let mut position = vec![u32::MAX; self.len()];
        let mut pool: Vec<u32> = Vec::with_capacity(among.len());
        for &i in among {
            if i < self.len() && position[i] == u32::MAX {
                position[i] = pool.len() as u32;
                pool.push(i as u32);
            }
        }
        let mut order = pool.clone();
        order.sort_by_key(|&i| self.starts[i as usize + 1] - self.starts[i as usize]);
        self.run(query, options, &pool, &order, Some(position))
    }

    /// What each query word is worth, each two adjacent words typed as one, and the halves of
    /// the words that may be two words typed without the space; and where each word's halves are.
    fn units(&self, words: &[Vec<u8>], q: &Query<'_>) -> (Vec<Worth>, Vec<usize>) {
        let mut units: Vec<Worth> = words.iter().map(|w| self.worth(w, q)).collect();
        for two in words.windows(2) {
            units.push(self.worth(&[&two[0][..], &two[1][..]].concat(), q));
        }
        // A word may be two words of an entry typed without the space. When some entry has the
        // word exactly, that reading is worth too little to matter (σ times two probabilities,
        // against a word typed with no error), and it is not worked out.
        let mut half_at = vec![usize::MAX; words.len()];
        for (j, w) in words.iter().enumerate() {
            if units[j].exact || !(4..=64).contains(&w.len()) {
                continue;
            }
            if let Some(cut) = self.split_point(w, q.pool) {
                half_at[j] = units.len();
                units.push(self.worth(&w[..cut], q));
                units.push(self.worth(&w[cut..], q));
            }
        }
        (units, half_at)
    }

    /// Whether entry `i` has all the numbers `wanted` (hashed, sorted).
    fn has_numbers(&self, i: usize, wanted: &[u64]) -> bool {
        let numbers = &self.numbers[self.number_starts[i] as usize..self.number_starts[i + 1] as usize];
        wanted.iter().all(|x| numbers.binary_search(x).is_ok())
    }

    /// The search among the entries of `pool`; `order` is the same entries sorted by length,
    /// `position` the position in `pool` of each entry (none: every entry, in order).
    fn run(
        &self,
        query: &str,
        options: &Options,
        pool: &[u32],
        order: &[u32],
        position: Option<Vec<u32>>,
    ) -> Vec<Hit> {
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
        let q = Query {
            options,
            pool,
            order,
            position,
            depth: Depth::new(options.depth.max(0.0), self.longest + 1),
            rho: options.typo_rate.clamp(1e-6, 0.5),
            eta: options.missing_word.clamp(1e-12, 0.5),
        };
        let n = words.len();
        let (owned, half_at) = self.units(&words, &q);
        let units: Vec<&Worth> = owned.iter().collect();
        // Each reading's prior times `Π P(w)` of its units (the units carry ratios to `P(w)`),
        // relative to the largest: the common factor cancels out of every result.
        let mut readings = model::readings(n, &half_at);
        for r in &mut readings {
            r.0 += r.1.iter().map(|&u| units[u].ln_any).sum::<f64>();
        }
        let top = readings.iter().map(|r| r.0).fold(f64::NEG_INFINITY, f64::max);
        let readings: Vec<(f64, Vec<usize>)> =
            readings.into_iter().map(|(p, u)| ((p - top).exp(), u)).collect();
        let (touched, seen) = found_where(&units);
        // P(query | entry), up to the common factor: the sum over readings of prior × Π P(w | entry).
        let given = |mine: &[Option<u32>]| -> f64 {
            readings
                .iter()
                .map(|(prior, us)| {
                    prior
                        * us.iter()
                            .map(|&u| {
                                mine[u].map_or(units[u].elsewhere, |c| units[u].found[c as usize].given)
                            })
                            .product::<f64>()
                })
                .sum()
        };
        let untouched = (pool.len() - touched.len()) as f64;
        let nowhere: Vec<Option<u32>> = vec![None; units.len()];
        let mut total = untouched * given(&nowhere);
        // M[T]: for each reading and each choice T of what its units stand for (pieces of the
        // entry, or "not in the entry"), the sum over the entries k of prior × Π of what the
        // choices are worth in k.
        let mut shared: ByKey<f64> = ByKey::default();
        let mut scratch = Tuples::default();
        for mine in &seen {
            total += given(mine);
            for (r, (prior, us)) in readings.iter().enumerate() {
                scratch.each(r, *prior, us, mine, &units, false, |key, w| {
                    *shared.entry(key).or_insert(0.0) += w
                });
            }
        }
        if untouched > 0.0 {
            for (r, (prior, us)) in readings.iter().enumerate() {
                scratch.each(r, *prior * untouched, us, &nowhere, &units, false, |key, w| {
                    *shared.entry(key).or_insert(0.0) += w;
                });
            }
        }
        let mean = total / pool.len() as f64;
        let mut wanted = if options.exact_numbers {
            numbers(query)
        } else {
            Vec::new()
        };
        wanted.sort_unstable();
        wanted.dedup();
        let mut out = Vec::new();
        for (&k, mine) in touched.iter().zip(&seen) {
            // The probability that entry i has what the words stand for: the sum over the entries
            // k that may be the wanted one of P(k | query) × P(i has what the words stand for in
            // k), that is Σ_T M[T] over the choices T that i has too, divided by Σ_k P(query | k).
            let mut has = 0.0;
            for (r, (_, us)) in readings.iter().enumerate() {
                scratch.each(r, 1.0, us, mine, &units, true, |key, f| {
                    has += f * shared.get(&key).copied().unwrap_or(0.0)
                });
            }
            let probability = (has / total).min(1.0);
            if probability <= 0.0 || options.exclude_below.is_some_and(|eps| probability < eps) {
                continue;
            }
            let i = pool[k as usize] as usize;
            if !self.has_numbers(i, &wanted) {
                continue;
            }
            let hit = Hit {
                index: i,
                score: ((given(mine) / mean).log10()).clamp(-300.0, 300.0),
                probability,
            };
            out.push((k, hit));
        }
        out.sort_by(|(ka, a), (kb, b)| b.score.total_cmp(&a.score).then(ka.cmp(kb)));
        out.into_iter().map(|(_, h)| h).collect()
    }
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
            Some(
                out.iter()
                    .map(|&b| {
                        if b == UNKNOWN {
                            others.next().unwrap_or('?')
                        } else {
                            b as char
                        }
                    })
                    .collect(),
            )
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
        *alphabet
            .entry(c)
            .or_insert(if n < 127 { 128 + n as u8 } else { UNKNOWN })
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
