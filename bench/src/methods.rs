//! The search methods the library is compared with.

use std::cell::RefCell;
use std::cmp::Reverse;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32String, chars};

pub trait Method {
    /// The entries kept, most relevant first.
    fn search(&self, query: &str) -> Vec<usize>;
}

/// nucleo-matcher (Helix's matcher, with fzf's rules): each word as a subsequence (its letters
/// in order, possibly scattered), all of them required, fzf's score.
pub struct Nucleo {
    texts: Vec<Utf32String>,
    matcher: RefCell<Matcher>,
}

fn fold_nucleo(c: char) -> char {
    chars::to_lower_case(chars::normalize(c))
}

impl Nucleo {
    pub fn new(entries: &[String]) -> Nucleo {
        let texts = entries
            .iter()
            .map(|t| {
                let p: String = t.chars().map(fold_nucleo).collect();
                if p.is_ascii() {
                    Utf32String::Ascii(p.into_boxed_str())
                } else {
                    Utf32String::Unicode(p.chars().collect())
                }
            })
            .collect();
        Nucleo {
            texts,
            matcher: RefCell::new(Matcher::new(Config::DEFAULT)),
        }
    }
}

impl Method for Nucleo {
    fn search(&self, query: &str) -> Vec<usize> {
        let folded: String = query.chars().map(|c| if c.is_whitespace() { ' ' } else { fold_nucleo(c) }).collect();
        let pattern = Pattern::parse(&folded, CaseMatching::Ignore, Normalization::Never);
        let mut matcher = self.matcher.borrow_mut();
        let mut found: Vec<(Reverse<u32>, usize)> = self
            .texts
            .iter()
            .enumerate()
            .filter_map(|(i, t)| Some((Reverse(pattern.score(t.slice(..), &mut matcher)?), i)))
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, i)| i).collect()
    }
}

/// A term engine like classic search engines (with a single field): each word must be an equal
/// term, a prefix or, from 4 letters, a term at Levenshtein distance 1 (2 from 8 letters) with
/// the same first letters; all required; IDF score, typos weighted 0.35.
pub struct Terms {
    terms: Vec<Vec<String>>,
}

pub fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let mut before: Vec<usize> = (0..=b.len()).collect();
    let mut now = vec![0; b.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        now[0] = i + 1;
        for (j, &y) in b.iter().enumerate() {
            now[j + 1] = (before[j] + usize::from(x != y)).min(before[j + 1] + 1).min(now[j] + 1);
        }
        std::mem::swap(&mut before, &mut now);
    }
    before[b.len()]
}

impl Terms {
    pub fn new(entries: &[String]) -> Terms {
        Terms {
            terms: entries
                .iter()
                .map(|v| {
                    let mut t = probable::words(v);
                    t.sort();
                    t.dedup();
                    t
                })
                .collect(),
        }
    }

    fn weight(terms: &[String], w: &str) -> f64 {
        let mut best: f64 = 0.0;
        let n = w.chars().count();
        for t in terms {
            if t == w || t.starts_with(w) {
                return 1.0;
            }
            if n >= 4 {
                let (dist, head) = if n >= 8 { (2, 2) } else { (1, 1) };
                if t.len().abs_diff(w.len()) <= dist
                    && t.as_bytes().get(..head) == w.as_bytes().get(..head)
                    && levenshtein(t.as_bytes(), w.as_bytes()) <= dist
                {
                    best = 0.35;
                }
            }
        }
        best
    }
}

impl Method for Terms {
    fn search(&self, query: &str) -> Vec<usize> {
        let words = probable::words(query);
        if words.is_empty() {
            return (0..self.terms.len()).collect();
        }
        let n = self.terms.len() as f64;
        let mut points = vec![0.0f64; self.terms.len()];
        let mut alive = vec![true; self.terms.len()];
        for w in &words {
            let weights: Vec<f64> = self.terms.iter().map(|t| Self::weight(t, w)).collect();
            let df = weights.iter().filter(|&&p| p > 0.0).count().max(1) as f64;
            let idf = (1.0 + n / df).ln();
            for i in 0..weights.len() {
                if weights[i] == 0.0 {
                    alive[i] = false;
                }
                points[i] += idf * weights[i];
            }
        }
        let mut out: Vec<usize> = (0..points.len()).filter(|&i| alive[i]).collect();
        out.sort_by(|&a, &b| points[b].total_cmp(&points[a]).then(a.cmp(&b)));
        out
    }
}

/// The simplest search: each word contained in the text as it is (no case, no accents); the
/// entries with the words nearest the start first.
pub struct Contains {
    texts: Vec<String>,
}

impl Contains {
    pub fn new(entries: &[String]) -> Contains {
        Contains {
            texts: entries.iter().map(|v| probable::words(v).join(" ")).collect(),
        }
    }
}

impl Method for Contains {
    fn search(&self, query: &str) -> Vec<usize> {
        let words = probable::words(query);
        let mut out: Vec<(usize, usize)> = self
            .texts
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let mut sum = 0;
                for w in &words {
                    sum += t.find(w.as_str())?;
                }
                Some((sum, i))
            })
            .collect();
        out.sort_unstable();
        out.into_iter().map(|(_, i)| i).collect()
    }
}
