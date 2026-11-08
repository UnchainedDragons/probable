//! The test bench of the `probable` library, on real lists and simulated queries.
//!
//! `cargo run --release -p probable-bench -- evaluate <list.txt> [queries] [seed]`: a list is a
//! text file with one entry per line. The queries imitate someone looking for an entry: they
//! pick one to three words of the entry (the *intended* ones), type some only in part, make
//! typos, change the order, split or join words, and sometimes type a word that is not in the
//! entry.
//!
//! **Relevant**, independently of any method: an entry whose words include all the intended
//! words. Recall, precision, P(relevant | excluded) and the calibration of the library's
//! probabilities are measured against this.
//!
//! `cargo run --release -p probable-bench -- speed <list.txt> [times]`: timings, also on the list
//! repeated.

mod methods;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use methods::{Contains, Method, Nucleo, Terms};
use probable::{Index, Options, words};

struct Random(u64);

impl Random {
    fn u(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn f(&mut self) -> f64 {
        (self.u() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn n(&mut self, n: usize) -> usize {
        (self.u() % n as u64) as usize
    }
}

#[derive(Clone, Copy)]
struct Noise {
    /// Probability of mistyping each letter.
    typo: f64,
    /// A word typed only in part.
    partial: f64,
    /// A word replaced with one that is not in the entry.
    foreign: f64,
    /// A long word split in two.
    split: f64,
    /// Two words typed together.
    join: f64,
    /// The order of the words changed.
    shuffle: f64,
}

struct Query {
    text: String,
    intended: Vec<String>,
    target: usize,
}

const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

fn dirty(word: &str, noise: &Noise, random: &mut Random) -> String {
    let mut c: Vec<char> = word.chars().collect();
    if c.len() >= 4 && random.f() < noise.partial {
        let minimum = (c.len().div_ceil(2)).max(2);
        let keep = minimum + random.n(c.len() - minimum);
        c.truncate(keep);
    }
    let any = |random: &mut Random, x: char| {
        if x.is_ascii_digit() {
            char::from(b'0' + random.n(10) as u8)
        } else {
            char::from(LETTERS[random.n(26)])
        }
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if random.f() < noise.typo {
            match random.n(4) {
                0 => out.push(any(random, c[i])),
                1 => {}
                2 => {
                    out.push(c[i]);
                    out.push(any(random, c[i]));
                }
                _ => {
                    if i + 1 < c.len() {
                        out.push(c[i + 1]);
                        out.push(c[i]);
                        i += 1;
                    } else {
                        out.push(c[i]);
                    }
                }
            }
        } else {
            out.push(c[i]);
        }
        i += 1;
    }
    if out.is_empty() {
        out.push(c.first().copied().unwrap_or('a'));
    }
    out.into_iter().collect()
}

fn generate(entries: &[Vec<String>], noise: &Noise, random: &mut Random) -> Option<Query> {
    let target = random.n(entries.len());
    let mut candidates: Vec<&String> = Vec::new();
    for p in &entries[target] {
        if p.chars().count() >= 2 && !candidates.contains(&p) {
            candidates.push(p);
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let r = random.f();
    let how_many = (if r < 0.3 { 1 } else if r < 0.75 { 2 } else { 3 }).min(candidates.len());
    let mut chosen: Vec<usize> = Vec::new();
    while chosen.len() < how_many {
        let i = random.n(candidates.len());
        if !chosen.contains(&i) {
            chosen.push(i);
        }
    }
    chosen.sort_unstable();
    let intended: Vec<String> = chosen.iter().map(|&i| candidates[i].clone()).collect();
    let mut typed: Vec<String> = intended.iter().map(|p| dirty(p, noise, random)).collect();
    if random.f() < noise.foreign {
        // A word that is not in the entry, taken from another entry.
        for _ in 0..20 {
            let other = &entries[random.n(entries.len())];
            if other.is_empty() {
                continue;
            }
            let p = &other[random.n(other.len())];
            if p.len() >= 3 && !entries[target].contains(p) {
                let i = random.n(typed.len());
                typed[i] = p.clone();
                break;
            }
        }
    }
    if random.f() < noise.shuffle {
        for i in (1..typed.len()).rev() {
            let j = random.n(i + 1);
            typed.swap(i, j);
        }
    }
    if random.f() < noise.split
        && let Some(i) = typed.iter().position(|p| p.chars().count() >= 6)
    {
        let c: Vec<char> = typed[i].chars().collect();
        let at = 2 + random.n(c.len() - 3);
        let (a, b) = c.split_at(at);
        typed[i] = format!("{} {}", a.iter().collect::<String>(), b.iter().collect::<String>());
    }
    if typed.len() >= 2 && random.f() < noise.join {
        let i = random.n(typed.len() - 1);
        let b = typed.remove(i + 1);
        typed[i].push_str(&b);
    }
    let mut text = typed.join(" ");
    if random.f() < 0.3 {
        text = text.to_uppercase();
    }
    Some(Query { text, intended, target })
}

/// The relevant entries: those with all the intended words.
struct Relevance {
    words: Vec<Vec<String>>,
    postings: HashMap<String, Vec<usize>>,
}

impl Relevance {
    fn new(entries: &[String]) -> Relevance {
        let words: Vec<Vec<String>> = entries
            .iter()
            .map(|v| {
                let mut p = words(v);
                p.sort();
                p.dedup();
                p
            })
            .collect();
        let mut postings: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, ps) in words.iter().enumerate() {
            for p in ps {
                postings.entry(p.clone()).or_default().push(i);
            }
        }
        Relevance { words, postings }
    }

    fn relevant(&self, intended: &[String]) -> Vec<usize> {
        let mut lists: Vec<&Vec<usize>> = intended.iter().map(|p| &self.postings[p]).collect();
        lists.sort_by_key(|l| l.len());
        lists[0]
            .iter()
            .copied()
            .filter(|&i| intended.iter().all(|p| self.words[i].binary_search(p).is_ok()))
            .collect()
    }
}

#[derive(Default, Clone)]
struct Counts {
    queries: usize,
    relevant: usize,
    relevant_kept: usize,
    kept: usize,
    excluded: usize,
    relevant_excluded: usize,
    target_kept: usize,
    first_relevant: usize,
    top_ten: f64,
    reciprocal: f64,
    time: Duration,
}

impl Counts {
    fn add(&mut self, kept: &[usize], relevant: &[bool], how_many_relevant: usize, target: usize, time: Duration) {
        let n = relevant.len();
        let rk = kept.iter().filter(|&&i| relevant[i]).count();
        self.queries += 1;
        self.relevant += how_many_relevant;
        self.relevant_kept += rk;
        self.kept += kept.len();
        self.excluded += n - kept.len();
        self.relevant_excluded += how_many_relevant - rk;
        if let Some(pos) = kept.iter().position(|&i| i == target) {
            self.target_kept += 1;
            self.reciprocal += 1.0 / (pos + 1) as f64;
        }
        if kept.first().is_some_and(|&i| relevant[i]) {
            self.first_relevant += 1;
        }
        let ten = kept.iter().take(10).filter(|&&i| relevant[i]).count();
        self.top_ten += ten as f64 / how_many_relevant.clamp(1, 10) as f64;
        self.time += time;
    }

    fn row(&self, name: &str) -> String {
        let q = self.queries as f64;
        format!(
            "| {name} | {:.2}% | {:.4}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.3} | {:.0} | {:.2} |",
            100.0 * self.relevant_kept as f64 / self.relevant as f64,
            100.0 * self.relevant_excluded as f64 / self.excluded.max(1) as f64,
            100.0 * self.relevant_kept as f64 / self.kept.max(1) as f64,
            100.0 * self.target_kept as f64 / q,
            100.0 * self.first_relevant as f64 / q,
            100.0 * self.top_ten / q,
            self.reciprocal / q,
            self.kept as f64 / q,
            self.time.as_secs_f64() * 1000.0 / q,
        )
    }
}

const HEADER: &str = "| method | recall | P(relevant \\| excluded) | precision | target kept | 1st relevant | top 10 | target MRR | entries kept | ms/query |\n|---|---|---|---|---|---|---|---|---|---|";

/// The calibration steps: probabilities given by the library.
const STEPS: [f64; 10] = [0.0, 1e-4, 1e-3, 1e-2, 0.1, 0.3, 0.5, 0.7, 0.9, 0.99];

#[derive(Default, Clone)]
struct Calibration {
    /// Per step (the last: entries with no word found): entries, sum of the probabilities,
    /// actually relevant.
    entries: [usize; 11],
    expected: [f64; 11],
    actual: [usize; 11],
}

fn read(path: &str) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("the list")
        .lines()
        .map(str::to_string)
        .filter(|l| !l.trim().is_empty())
        .collect()
}

fn evaluate(path: &str, how_many: usize, seed: u64) {
    let entries = read(path);
    let n = entries.len();
    let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy();
    println!("## {name}: {n} entries, {how_many} queries per noise level\n");
    let start = Instant::now();
    let index = Index::new(&entries);
    println!("index: {:?}", start.elapsed());
    let nucleo = Nucleo::new(&entries);
    let terms = Terms::new(&entries);
    let contains = Contains::new(&entries);
    let relevance = Relevance::new(&entries);
    let entry_words: Vec<Vec<String>> = entries.iter().map(|v| words(v)).collect();
    let unfiltered = Options {
        exclude_below: None,
        ..Options::default()
    };
    let strict = Options {
        exclude_below: None,
        require_all_words: true,
        ..Options::default()
    };
    let levels = [
        ("no typos", 0.0),
        ("3% typos per letter (what the model assumes)", 0.03),
        ("8% typos per letter (more than the model)", 0.08),
    ];
    let mut calibration = Calibration::default();
    for (level_name, typo) in levels {
        let noise = Noise {
            typo,
            partial: 0.2,
            foreign: 0.03,
            split: 0.1,
            join: 0.1,
            shuffle: 0.5,
        };
        let mut random = Random(seed ^ (typo * 1000.0) as u64);
        let mut counts: Vec<Counts> = vec![Counts::default(); 6];
        let mut relevant = vec![false; n];
        let mut done = 0;
        while done < how_many {
            let Some(q) = generate(&entry_words, &noise, &mut random) else {
                continue;
            };
            done += 1;
            let rel = relevance.relevant(&q.intended);
            for &i in &rel {
                relevant[i] = true;
            }
            // The library without the filter: the probabilities of every entry with a word.
            let t0 = Instant::now();
            let hits = index.search(&q.text, &unfiltered);
            let time = t0.elapsed();
            let filtered: Vec<usize> = hits.iter().filter(|h| h.probability >= 0.01).map(|h| h.index).collect();
            if done == 1 {
                let from_library: Vec<usize> = index.search(&q.text, &Options::default()).iter().map(|h| h.index).collect();
                assert_eq!(from_library, filtered);
            }
            let all: Vec<usize> = hits.iter().map(|h| h.index).collect();
            counts[0].add(&filtered, &relevant, rel.len(), q.target, time);
            counts[1].add(&all, &relevant, rel.len(), q.target, time);
            let t0 = Instant::now();
            let strict_hits: Vec<usize> = index.search(&q.text, &strict).iter().map(|h| h.index).collect();
            counts[2].add(&strict_hits, &relevant, rel.len(), q.target, t0.elapsed());
            for (k, method) in [&nucleo as &dyn Method, &terms, &contains].into_iter().enumerate() {
                let t0 = Instant::now();
                let r = method.search(&q.text);
                counts[3 + k].add(&r, &relevant, rel.len(), q.target, t0.elapsed());
            }
            // Calibration: every entry, with its probability.
            let mut seen = 0;
            for h in &hits {
                let g = STEPS.iter().rposition(|&g| h.probability >= g).unwrap_or(0);
                calibration.entries[g] += 1;
                calibration.expected[g] += h.probability;
                calibration.actual[g] += usize::from(relevant[h.index]);
                seen += usize::from(relevant[h.index]);
            }
            calibration.entries[10] += n - hits.len();
            calibration.actual[10] += rel.len() - seen;
            for &i in &rel {
                relevant[i] = false;
            }
        }
        println!("\n### {level_name}\n\n{HEADER}");
        let names = [
            "**probable, filter ε = 1%**",
            "probable, no filter",
            "probable, all words",
            "nucleo (fzf)",
            "terms + Levenshtein",
            "contains",
        ];
        for (c, name) in counts.iter().zip(names) {
            println!("{}", c.row(name));
        }
    }
    println!("\n### Calibration of the probabilities (all levels together)\n");
    println!("| probability given | entries | expected relevant | actual relevant |\n|---|---|---|---|");
    for g in 0..11 {
        let name = if g == 10 {
            "no word found".to_string()
        } else if g == 9 {
            format!("≥ {}", STEPS[g])
        } else {
            format!("[{}, {})", STEPS[g], STEPS[g + 1])
        };
        let expected = if g == 10 { "—".to_string() } else { format!("{:.1}", calibration.expected[g]) };
        println!("| {name} | {} | {expected} | {} |", calibration.entries[g], calibration.actual[g]);
    }
    println!();
}

fn speed(path: &str, times: usize) {
    let base = read(path);
    let entries: Vec<String> = (0..times)
        .flat_map(|k| base.iter().map(move |v| if k == 0 { v.clone() } else { format!("{v} {k}") }))
        .collect();
    let letters: usize = entries.iter().map(String::len).sum();
    let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy();
    println!("## {name} ×{times}: {} entries, {:.1} MB", entries.len(), letters as f64 / 1e6);
    let t0 = Instant::now();
    let index = Index::new(&entries);
    println!("index: {:?}", t0.elapsed());
    let nucleo = Nucleo::new(&entries);
    let mut random = Random(7);
    let entry_words: Vec<Vec<String>> = base.iter().map(|v| words(v)).collect();
    let noise = Noise {
        typo: 0.03,
        partial: 0.2,
        foreign: 0.0,
        split: 0.1,
        join: 0.1,
        shuffle: 0.5,
    };
    let queries: Vec<String> = std::iter::from_fn(|| generate(&entry_words, &noise, &mut random))
        .take(200)
        .map(|q| q.text)
        .collect();
    let o = Options::default();
    let t0 = Instant::now();
    for q in &queries {
        std::hint::black_box(index.search(q, &o));
    }
    let ours = t0.elapsed() / queries.len() as u32;
    let t0 = Instant::now();
    for q in &queries {
        std::hint::black_box(nucleo.search(q));
    }
    let fzf = t0.elapsed() / queries.len() as u32;
    println!("probable: {ours:?} per query; nucleo: {fzf:?} per query\n");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let number = |i: usize, default: usize| a.get(i).map_or(default, |s| s.parse().expect("a number"));
    match a.get(1).map(String::as_str) {
        Some("evaluate") => evaluate(&a[2], number(3, 1000), number(4, 1) as u64),
        Some("speed") => speed(&a[2], number(3, 1)),
        _ => eprintln!("probable-bench evaluate <list> [queries] [seed] | probable-bench speed <list> [times]"),
    }
}
