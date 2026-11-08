//! Behaviour tests, on made-up texts.

use super::*;

fn indexes(index: &Index, query: &str) -> Vec<usize> {
    index
        .search(query, &Options::default())
        .into_iter()
        .map(|h| h.index)
        .collect()
}

const ENTRIES: [&str; 8] = [
    "ALPHA PLUMBING LTD TUB20 MULTILAYER PIPE 20 MM ROLL 50 M",
    "BETA SUPPLIES INC FIT20 T FITTING 20 MM BRASS",
    "GAMMA HEATING CO VAL34 BALL VALVE 3/4 BRASS",
    "ALPHA PLUMBING LTD ELB80 ELBOW 90° 80 MM STAINLESS STEEL",
    "DELTA ELECTRIC LTD CAB6 SOLAR CABLE 6 MM2 RED",
    "BETA SUPPLIES INC FIT100 STRAIGHT FITTING 100 MM BRASS",
    "GAMMA HEATING CO SLV20 SLEEVE 20 MM PVC GREY",
    "DELTA ELECTRIC LTD INV5 HYBRID INVERTER 5 KW",
];

#[test]
fn empty_query_gives_everything_in_order() {
    let index = Index::new(ENTRIES);
    assert_eq!(indexes(&index, ""), (0..8).collect::<Vec<_>>());
    assert_eq!(indexes(&index, " \t ,"), (0..8).collect::<Vec<_>>());
    assert!(
        Index::new(Vec::<String>::new())
            .search("pipe", &Options::default())
            .is_empty()
    );
}

#[test]
fn words_in_any_order_and_with_typos() {
    let index = Index::new(ENTRIES);
    assert_eq!(indexes(&index, "solar cable")[0], 4);
    assert_eq!(indexes(&index, "cable solar")[0], 4);
    // A wrong letter, a missing one, two swapped. Entries 1 and 5 are two brass fittings,
    // equally good.
    for query in ["fitting brass", "fittimg brass", "fiting brass", "fititng brsas"] {
        let i = indexes(&index, query);
        assert!([1, 5].contains(&i[0]) && [1, 5].contains(&i[1]), "{query}: {i:?}");
    }
    assert_eq!(indexes(&index, "invertr")[0], 7);
}

#[test]
fn words_split_and_joined() {
    let index = Index::new(ENTRIES);
    assert_eq!(indexes(&index, "multi layer")[0], 0);
    assert_eq!(indexes(&index, "multilayerpipe")[0], 0);
    assert_eq!(indexes(&index, "20mm sleeve")[0], 6);
    let joined = Index::new(["MULTI-LAYER", "MONOLAYER"]);
    assert_eq!(indexes(&joined, "multilayer"), [0]);
}

#[test]
fn deeper_means_fewer_points() {
    let index = Index::new([
        "x x x x x x x x x x x x valve",
        "x x x x valve x x x x x x x x",
        "valve x x x x x x x x x x x x",
    ]);
    let h = index.search("valve", &Options::default());
    assert_eq!(h.iter().map(|h| h.index).collect::<Vec<_>>(), [2, 1, 0]);
    assert!(h[0].score > h[1].score && h[1].score > h[2].score);
    // Without depth, almost equal: all that is left is how rare it is for the first chance
    // occurrence to fall that deep.
    let flat = Options {
        depth: 0.0,
        ..Options::default()
    };
    let h = index.search("valve", &flat);
    assert!((h[0].score - h[2].score).abs() < 0.1, "{h:?}");
}

#[test]
fn exact_before_one_error_and_word_start_before_inside() {
    let index = Index::new(["UNDERBUSHING", "BUSHING", "BASHING"]);
    let i = indexes(&index, "bushing");
    assert_eq!(i[0], 1);
    let h = index.search("bushing", &Options::default());
    assert!(h[0].score > h[1].score + 0.5, "{h:?}");
}

#[test]
fn the_filter_removes_noise_and_keeps_the_good_ones() {
    let index = Index::new(ENTRIES);
    let h = index.search("fitting brass", &Options::default());
    let i: Vec<_> = h.iter().map(|h| h.index).collect();
    assert!(i.contains(&1) && i.contains(&5), "{h:?}");
    // Entries with neither word are not there.
    assert!(!i.contains(&4) && !i.contains(&7), "{h:?}");
    for h in &h {
        assert!(h.probability >= 0.01);
    }
    // Without the filter, those with one word only stay too, after.
    let all = index.search(
        "fitting brass",
        &Options {
            exclude_below: None,
            ..Options::default()
        },
    );
    assert!(all.len() >= h.len());
}

#[test]
fn require_all_words_like_classic_search() {
    let index = Index::new(ENTRIES);
    let o = Options {
        require_all_words: true,
        exclude_below: None,
        ..Options::default()
    };
    let mut i: Vec<_> = index
        .search("fitting brass", &o)
        .into_iter()
        .map(|h| h.index)
        .collect();
    i.sort_unstable();
    assert_eq!(i, [1, 5]);
}

#[test]
fn non_latin_letters() {
    let index = Index::new(["Ωμέγα αλφα", "omega alfa"]);
    assert_eq!(indexes(&index, "ΩΜΕΓΑ")[0], 0);
    assert_eq!(indexes(&index, "omega")[0], 1);
    // A letter the index has never seen equals nothing, but one error is one error.
    assert_eq!(indexes(&index, "ωμέγж")[0], 0);
}

#[test]
fn very_long_words() {
    let long = "a".repeat(30) + &"b".repeat(50);
    let index = Index::new([format!("x {long} y"), "x y".to_string()]);
    assert_eq!(indexes(&index, &long), [0]);
    let with_error = "a".repeat(30) + "c" + &"b".repeat(49);
    assert_eq!(indexes(&index, &with_error), [0]);
}

#[test]
fn equal_scores_keep_the_order() {
    let index = Index::new(["zeta b", "alfa a", "zeta b", "alfa a", "zeta b"]);
    assert_eq!(indexes(&index, "zeta"), [0, 2, 4]);
}

#[test]
fn letters_to_highlight() {
    assert_eq!(
        highlight("Café FITTING", "fiting", 2),
        (5..12).collect::<Vec<_>>()
    );
    assert_eq!(highlight("Café FITTING", "cafe", 2), (0..4).collect::<Vec<_>>());
    assert_eq!(
        highlight("Pipe Multi-Layer", "multilayer", 2),
        [5, 6, 7, 8, 9, 11, 12, 13, 14, 15]
    );
    assert!(highlight("Pipe", "zzzz", 2).is_empty());
    // A word start before a piece inside a word.
    assert_eq!(
        highlight("underbushing bushing", "bushing", 2),
        (13..20).collect::<Vec<_>>()
    );
}

#[test]
fn a_large_index_is_searched_in_parallel_with_the_same_result() {
    let entries: Vec<String> = (0..60_000)
        .map(|i| format!("entry {i} fitting {} brass {}", i % 97, i % 13))
        .collect();
    let index = Index::new(&entries);
    assert!(index.text.len() > PARALLEL_MIN);
    let h = index.search("entry 12345", &Options::default());
    assert_eq!(h[0].index, 12345);
    let small = Index::new(&entries[12_000..12_500]);
    assert_eq!(small.search("entry 12345", &Options::default())[0].index, 345);
}

/// `LIST=… cargo test --release measure_the_scan -- --ignored --nocapture`
#[test]
#[ignore]
fn measure_the_scan() {
    use std::time::Instant;
    let entries: Vec<String> = std::fs::read_to_string(std::env::var("LIST").unwrap())
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    let index = Index::new(&entries);
    println!("{} folded letters", index.text.len());
    for (word, k) in [("fitting", 2u32), ("pipe", 1), ("pl", 0), ("valve", 2)] {
        let p = word.as_bytes();
        let pattern = Pattern::new(p);
        let times = 50;
        let t0 = Instant::now();
        let mut n = 0usize;
        for _ in 0..times {
            pattern.scan(&index.text, k, |_, _| {
                n += 1;
                true
            });
        }
        let raw = t0.elapsed() / times;
        let t0 = Instant::now();
        for _ in 0..times {
            std::hint::black_box(index.occurrences(p, k as usize, &index.by_length));
        }
        let full = t0.elapsed() / times;
        println!(
            "{word} k={k}: automaton alone {raw:?} ({:.2} ns/letter, {} places ≤ k), occurrences {full:?}",
            raw.as_nanos() as f64 / index.text.len() as f64,
            n / times as usize
        );
    }
}

/// A small, repeatable generator.
struct Random(u64);

impl Random {
    fn n(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) % n as u64) as usize
    }
    fn text(&mut self, length: usize) -> String {
        const PIECES: &[&str] = &[
            "a", "b", "c", "o", "r", "t", "1", "8", "0", " ", " ", "-", ".", "/", "È", "à", "ß", "Æ", "Ω",
            "ж", "中", "😀", "ＡＢ", "²", "\t", "'", "fitting", "PIPE", "multi", "layer", "x",
        ];
        (0..length).map(|_| PIECES[self.n(PIECES.len())]).collect()
    }
}

/// Random lists and queries: no panic, right order, probabilities between 0 and 1, the filter
/// removes and never adds, "require all words" is a subset, the letters to highlight are in the
/// text.
#[test]
fn random_input_always_holds() {
    let mut random = Random(42);
    for round in 0..400 {
        let entries: Vec<String> = (0..random.n(40))
            .map(|_| {
                let l = random.n(if round % 50 == 0 { 300 } else { 25 });
                random.text(l)
            })
            .collect();
        let index = Index::new(&entries);
        for _ in 0..10 {
            let l = random.n(8);
            let query = random.text(l);
            let max_errors = random.n(5) as u8;
            let unfiltered = Options {
                max_errors,
                exclude_below: None,
                ..Options::default()
            };
            let all = index.search(&query, &unfiltered);
            let filtered = index.search(
                &query,
                &Options {
                    max_errors,
                    ..Options::default()
                },
            );
            let strict = index.search(
                &query,
                &Options {
                    max_errors,
                    require_all_words: true,
                    exclude_below: None,
                    ..Options::default()
                },
            );
            for h in &all {
                assert!(h.index < entries.len());
                assert!(
                    (0.0..=1.0).contains(&h.probability) && h.score.is_finite(),
                    "{h:?}"
                );
            }
            assert!(all.windows(2).all(|w| w[0].score > w[1].score || (w[0].score == w[1].score && w[0].index < w[1].index)));
            let mut seen = std::collections::HashSet::new();
            assert!(all.iter().all(|h| seen.insert(h.index)));
            assert!(filtered.iter().all(|f| all.iter().any(|h| h == f)));
            assert!(strict.iter().all(|f| all.iter().any(|h| h.index == f.index)));
            for e in &entries {
                let letters = highlight(e, &query, max_errors);
                let n = e.chars().count();
                assert!(letters.iter().all(|&p| p < n));
                assert!(letters.windows(2).all(|w| w[0] < w[1]));
            }
        }
    }
}

fn same(x: &Occurrences, y: &Occurrences) -> bool {
    (x.whole, x.prefix, x.inner, x.spanning) == (y.whole, y.prefix, y.inner, y.spanning)
}

/// The same index, searched serially and in parallel, gives the same answers.
#[test]
fn parallel_equals_serial() {
    let mut random = Random(7);
    let entries: Vec<String> = (0..30_000)
        .map(|_| {
            let l = 5 + random.n(30);
            random.text(l)
        })
        .collect();
    let large = Index::new(&entries);
    assert!(large.text.len() > PARALLEL_MIN && large.threads(large.len()) > 1);
    for _ in 0..30 {
        let l = 1 + random.n(10);
        let query = random.text(l);
        let o = Options {
            exclude_below: None,
            ..Options::default()
        };
        let a = large.search(&query, &o);
        for word in &large.query_words(&query) {
            let errors = tolerance(word.len(), o.max_errors);
            let pattern = (word.len() <= 64).then(|| Pattern::new(word));
            let mut serial = vec![Occurrences::EMPTY; large.len()];
            for (i, o) in large.occurrences_in(&large.by_length, word, pattern.as_ref(), errors) {
                serial[i as usize] = o;
            }
            let parallel = large.occurrences(word, errors, &large.by_length);
            assert!(serial.iter().zip(&parallel).all(|(x, y)| same(x, y)));
        }
        assert_eq!(a, large.search(&query, &o));
    }
}

/// Entries scanned in interleaved groups give the same as one at a time.
#[test]
fn interleaved_equals_one_at_a_time() {
    let mut random = Random(11);
    let entries: Vec<String> = (0..3000)
        .map(|_| {
            let l = random.n(40);
            random.text(l)
        })
        .collect();
    let index = Index::new(&entries);
    for word in ["fitting", "pipe", "x", "o1", "multilayer", "ra"] {
        let p = index.query_words(word).remove(0);
        for errors in 0..=2 {
            let pattern = Pattern::new(&p);
            let mut interleaved = vec![Occurrences::EMPTY; index.len()];
            for (i, o) in index.occurrences_in(&index.by_length, &p, Some(&pattern), errors) {
                interleaved[i as usize] = o;
            }
            for (i, y) in interleaved.iter().enumerate() {
                let mut c = index.scan_state(i, errors);
                let text = &index.text[c.start..c.start + c.letters];
                pattern.scan(text, errors as u32, |j, d| index.record(&mut c, &p, j, d));
                assert!(same(&c.occurrences, y), "{word} {i}");
            }
        }
    }
}

#[test]
fn search_among_keeps_the_callers_order() {
    let index = Index::new(["red pipe", "blue", "red pipe", "red pipe", "green pipe"]);
    let o = Options::default();
    let positions = |hits: Vec<Hit>| hits.into_iter().map(|h| h.index).collect::<Vec<_>>();
    // Equal scores keep the order given, not the index order.
    assert_eq!(positions(index.search_among("red", &[3, 0, 2], &o)), [3, 0, 2]);
    // Only the entries given; out of range and repeated ones are ignored.
    assert_eq!(positions(index.search_among("pipe", &[4, 9, 4, 1], &o)), [4]);
    assert_eq!(positions(index.search_among("", &[2, 1, 2, 7], &o)), [2, 1]);
    assert!(index.search_among("red", &[], &o).is_empty());
    // Among everything, the same as `search`.
    assert_eq!(
        index.search_among("pipe", &[0, 1, 2, 3, 4], &o),
        index.search("pipe", &o)
    );
}

#[test]
fn the_index_can_be_shared_between_threads() {
    fn shareable<T: Send + Sync + 'static>() {}
    shareable::<Index>();
    let index = std::sync::Arc::new(Index::new(ENTRIES));
    let other = std::sync::Arc::clone(&index);
    let from_thread = std::thread::spawn(move || other.search("brass", &Options::default()))
        .join()
        .unwrap();
    assert_eq!(from_thread, index.search("brass", &Options::default()));
}

#[test]
fn depth_starts_again_in_each_field() {
    // The same phone number, after a long name or with no name at all: equally good.
    let index = Index::with_fields(
        [vec!["Giulia Bianchi", "320 0000000"], vec!["", "320 0000000"]],
        &[12.0, 11.0],
    );
    let h = index.search("3200000000", &Options::default());
    assert_eq!(h.iter().map(|h| h.index).collect::<Vec<_>>(), [0, 1]);
    assert!((h[0].score - h[1].score).abs() < 1e-9, "{h:?}");
}

#[test]
fn heavier_fields_count_more() {
    let index = Index::with_fields([["pipe", "valve"], ["valve", "pipe"]], &[1.0, 3.0]);
    assert_eq!(indexes(&index, "valve")[0], 0);
    assert_eq!(indexes(&index, "pipe")[0], 1);
    // Missing fields are empty.
    let short = Index::with_fields([vec!["valve"], vec!["pipe", "valve"]], &[1.0, 3.0]);
    assert_eq!(indexes(&short, "valve"), [1, 0]);
}

#[test]
fn one_field_is_the_same_as_plain_text() {
    let plain = Index::new(ENTRIES);
    let fields = Index::with_fields(ENTRIES.iter().map(|e| [*e]), &[5.0]);
    for q in [
        "fitting brass",
        "solar cable",
        "invertr",
        "multi layer",
        "20mm sleeve",
    ] {
        assert_eq!(
            plain.search(q, &Options::default()),
            fields.search(q, &Options::default()),
            "{q}"
        );
    }
}

#[test]
#[should_panic(expected = "more fields than weights")]
fn too_many_fields_is_an_error() {
    Index::with_fields([["a", "b", "c"]], &[1.0, 1.0]);
}

#[test]
fn a_chance_match_does_not_count_as_the_word() {
    // «liza» with one typo occurs inside «canalizzabile», «mk5» is close to «Mk6»: by chance.
    let index = Index::new([
        "Wood stove Liza Mk5 Stone",
        "Pellet stove Aria Stone Mk6, canalizzabile with front ventilation",
    ]);
    let all_words = Options {
        require_all_words: true,
        exclude_below: None,
        ..Options::default()
    };
    assert_eq!(indexes_with(&index, "liza mk5 stone", &all_words), [0]);
    // An exact piece always counts, even inside a word; the whole word comes first.
    let short = Index::new(["valve a sphere", "valve sphere"]);
    assert_eq!(indexes_with(&short, "valve a", &all_words), [0, 1]);
}

fn indexes_with(index: &Index, query: &str, options: &Options) -> Vec<usize> {
    index
        .search(query, options)
        .into_iter()
        .map(|h| h.index)
        .collect()
}

#[test]
fn exact_numbers_identify_things() {
    let index = Index::new([
        "ACM-09 Acme 9.000 BTU",
        "ACM-12 Acme 12.000 BTU",
        "ACM-18 Acme 18.000 BTU",
        "Pipe 150",
        "Pipe 1500",
    ]);
    let exact = Options {
        exact_numbers: true,
        ..Options::default()
    };
    assert_eq!(indexes_with(&index, "acm-12", &exact), [1]);
    assert_eq!(indexes_with(&index, "ACME 12000", &exact), [1]);
    assert_eq!(indexes_with(&index, "pipe 150", &exact), [3]);
    let measures = Index::new(["TA 1 150 1000 utile", "TB 2 1500"]);
    assert_eq!(indexes_with(&measures, "150", &exact), [0]);
    assert_eq!(indexes_with(&measures, "1000", &exact), [0]);
    // Typos stay allowed in the letters.
    assert_eq!(indexes_with(&index, "acne 18.000", &exact), [2]);
    // Without the option a number is a word like any other.
    assert!(indexes(&index, "pipe 150").contains(&4));
}
