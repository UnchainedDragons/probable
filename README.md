# probable

Approximate search in a list of strings, in Rust, with no dependencies.

- **Typos.** Each query word is searched on its own, anywhere in the text and in any order, with
  a few typos allowed. The distance is Damerau-Levenshtein: insertion, deletion, substitution or
  swap of two adjacent letters, one error each. Short words allow fewer errors.
- **Case, accents and separators don't matter**, in the text or in the query: "multilayer"
  finds "MULTI-LAYER" and "multi layer" finds "Multilayer". A word found nowhere is also tried
  split in two, for the forgotten space ("smith27315").
- **The score is the logarithm of a likelihood ratio.** It drops with the depth of the key, with
  the typos, and with how easy it is to find that piece by chance.
- **Every entry gets a probability of being relevant.** The filter only excludes entries below a
  threshold ε (1%).

```rust
use probable::{Index, Options};

let index = Index::new(["Multilayer pipe 20 mm", "Brass T fitting", "Ball valve"]);
for hit in index.search("brass fiting", &Options::default()) {
    println!("{} {:.2} {:.3}", hit.index, hit.score, hit.probability);
}
// The letters to highlight, as character positions in the original text.
let letters = probable::highlight("Brass T fitting", "brass fiting", 2);
```

Entries made of fields (name, code, description…) go through `Index::with_fields`: fields are
given most important first, each with a weight, and depth starts again in each field, so a long
name does not push the code down. `search_among` searches only the entries the program chooses
(for example those of the current view), keeping their order on equal score.

```rust
let clients = Index::with_fields(
    [["Mario Rossi", "333 1234567", "Turin"], ["Rossi Plumbing Ltd", "011 5550123", "Leeds"]],
    &[12.0, 11.0, 6.0], // name, phones, town
);
let in_view = clients.search_among("rossi", &[1, 0], &Options::default());
```

`Index::new` folds the texts once, in 5 to 35 ms per 100 000 entries depending on their length.
`search` takes `&self`, so an index can be shared between threads. Above 256 KB of text the
search uses all processors.

| option | default | what |
|---|---|---|
| `max_errors` | 2 | maximum errors per word; fewer for short words (0 up to 2 letters, 1 up to 5, 2 up to 9, then 3), like Lucene's "AUTO" |
| `depth` | 1.0 | β: a key at word `i` is worth `(i+1)^β` times less than one at the start; 0 turns it off |
| `exclude_below` | `Some(0.01)` | the filter's ε; `None` keeps every entry with at least one word |
| `require_all_words` | `false` | keep only entries with every word (the classic "and" search) |
| `typo_rate` | 0.03 | ρ, the probability of mistyping a letter |
| `missing_word` | 0.01 | η, the probability that the wanted entry lacks one of the typed words |

## How it works

### 1. Folded text

Every letter or digit becomes one or more lowercase ASCII bytes ("È" → `e`, "ß" → `ss`,
"²" → `2`, from the NFKD decomposition); everything else disappears. Word starts stay marked:
after a separator, between letters and digits ("RAC100MF" → `rac` `100` `mf`), and from
lowercase to uppercase. Non-Latin letters stay themselves, with a code of their own.

### 2. The distance at every position: Myers and Hyyrö

For each query word (up to 64 letters), Myers' automaton (1999), in Hyyrö's form with
transpositions (2003), scans the text and gives, at every position, the minimum distance between
the word and a piece of text ending there. One column of the distance table fits in a `u64`:
about a dozen operations per character, whatever the tolerance. For each entry the first piece of
each kind is kept, for each number of errors:

- **whole**: a whole word of the text;
- **prefix**: the beginning of a word (someone typing part of it);
- **inner**: inside a word (someone typing part of a code);
- **spanning**: across two words (someone typing as one word what the text splits).

Three tricks make it fast without changing the result:

- **missing letters**: each letter of the word that the entry lacks costs at least one error, so
  an entry missing more letters than the tolerance is not scanned;
- **several entries at once**: each character depends on the previous one, but entries are
  independent; scanning six of similar length interleaved keeps the processor busy in parallel;
- **several processors**: above 256 KB of text.

The tests compare the automaton with the full table on 200 000 random cases, and the
interleaved, serial and parallel scans with scanning one entry at a time.

### 3. The score: a likelihood ratio

For each typed word and each entry: how much more likely is what was found if the entry is the
wanted one (R) than if it is not (¬R)?

```
Λ = η + (1 − η) · G / C
```

- **G**, if the entry is the wanted one: the word is there, typed with `d` typos
  (`Bin(m, ρ)`, ρ = 3%), in one of the four kinds (whole 0.8·0.7, prefix 0.8·0.3, inner 0.1,
  spanning 0.1), at depth `i` with weight `(i+1)^-β` normalised over the length of the entry.
- **C**, by chance: the probability that the *first* similar piece falls there, `r (1 − r)^i`,
  where `r` is the frequency per position of similar pieces. `r` is measured on the list itself,
  **inside words** and **in the entries that lack the whole word**. There a similar piece is
  almost never the wanted word, so the measure has the real structure of the language ("on"
  inside "condition") without being skewed by the relevant entries. With few texts, 20 imaginary
  entries with the letter frequencies only are added.
- **η** = 1%: the wanted entry may lack a typed word (a synonym, a typo too big). So Λ never goes
  below η, and an entry without the word gets `η + (1 − η) P(more than k typos)`.

The score of the entry is `Σ log10 Λ` over the words: 0 is neutral, 3 a thousand times more
likely, −2 a hundred times less. Depth lowers it by `β log10(i + 1)`. No weight is hand-tuned:
each factor is a probability, fixed before the tests.

### 4. The probability and the filter

The list is seen as a mixture: a share π of relevant entries, the rest not (Efron's "two-groups
model"). For each entry:

```
P(relevant) = π Λ / (π Λ + 1 − π)
```

π is estimated on every search from the list itself: the maximum-likelihood estimate, but never
less than the share of entries with Λ ≥ 1. The reason: if the model underestimates the Λ of good
entries (two-letter, ambiguous queries), maximum likelihood collapses and good entries would be
excluded; a larger π can only remove exclusions. The filter excludes entries with P < ε.

## The tests

Everything can be rerun with `cargo test --release` and
`cargo run --release -p probable-bench -- evaluate <list.txt>`.

### Correctness tests

33 tests, including:

- the automaton against the full distance table on 200 000 random cases (words up to 64
  letters), and the full table against the definition;
- the interleaved scan against scanning one entry at a time, and the parallel scan against the
  serial one;
- 4 000 searches on random lists (several alphabets, emoji, very long words, empty texts): no
  panic, right order, probabilities between 0 and 1, the filter removes and never adds.

### The bench

Four real lists (not published here):

| list | entries | what they are |
|---|---|---|
| purchases | 5 147 | purchase invoice lines in Italian: supplier, code, description |
| sales | 1 167 | sales invoice lines in Italian |
| catalogues | 19 498 | items from four supplier catalogues: title, code, measures |
| dictionary | 235 976 | macOS `/usr/share/dict/words`, in English |

The queries imitate someone searching. From a random entry, one to three words are picked (the
*intended* ones). Then:

- typos (0%, 3% or 8% per letter, of four kinds);
- words typed only in part (20%);
- order changed (50%);
- one word split (10%) or two joined (10%);
- a word that is not in the entry (3%);
- all uppercase (30%).

**Relevant**, independently of any method: an entry whose words include all the intended ones.

The model was tuned looking only at the purchases. Sales, catalogues and dictionary were tested
once, with the model frozen. Below is the mean over 3 seeds (2 for the dictionary), 1 000 queries
per seed and level (300 for the dictionary); the worst and best seed are in brackets.

The other methods:

- **nucleo**: Helix's matcher with fzf's rules (each word as a subsequence, all required);
- **terms**: whole words, prefixes and Levenshtein 1–2 on words of at least 4 letters, IDF score;
- **contains**: each word contained as it is.

#### With 3% typos per letter (what the model assumes)

| list | method | recall | 1st entry relevant | MRR of the source entry | entries kept |
|---|---|---|---|---|---|
| purchases | **probable, filter 1%** | **95.9%** (94.2–97.5) | **86.1%** | **0.477** | 559 |
| | nucleo | 86.9% (83.2–89.8) | 71.3% | 0.366 | 553 |
| | terms | 82.7% | 67.9% | 0.346 | 91 |
| | contains | 77.4% | 55.9% | 0.306 | 102 |
| sales | **probable, filter 1%** | **97.3%** (95.8–98.2) | **88.0%** | **0.413** | 253 |
| | nucleo | 90.8% (89.6–92.4) | 78.2% | 0.290 | 191 |
| | terms | 85.0% | 70.4% | 0.262 | 64 |
| | contains | 81.0% | 59.2% | 0.263 | 67 |
| catalogues | **probable, filter 1%** | **96.9%** (96.1–97.8) | **82.3%** | **0.216** | 4 811 |
| | nucleo | 87.8% (85.2–91.3) | 69.3% | 0.173 | 2 643 |
| | terms | 83.4% | 64.1% | 0.159 | 647 |
| | contains | 79.9% | 54.1% | 0.139 | 707 |
| dictionary | **probable, filter 1%** | **96.5%** (95.7–97.3) | **81.8%** | **0.857** | 450 |
| | nucleo | 80.5% (80.1–80.8) | 69.8% | 0.724 | 291 |
| | terms | 81.6% | 70.3% | 0.736 | 18 |
| | contains | 78.0% | 68.7% | 0.710 | 87 |

Without typos the search equals nucleo or does better: recall 95.4–97.9% against 94.6–97.9%,
first entry relevant 84.5–91.0% against 84.5–87.7%. With 8% typos the gap widens: recall
94.5–95.4% against 57.9–80.8% for nucleo.

Precision (how many kept entries are relevant) is low for every method that tolerates typos
(6–25%): someone typing "pl" may want "plus" or "plastic", and no search can know. That is why
the order matters: the first relevant entry is at the top 82–91% of the time.

#### The filter

**How many relevant entries there are among those with probability below ε**, over all seeds and
typo levels:

| ε | purchases | sales | catalogues | dictionary |
|---|---|---|---|---|
| 10% | 0.114% | 0.506% | 0.149% | 0.000% |
| **1%** | **0.077%** | **0.348%** | **0.099%** | **0.000%** |
| 0.1% | 0.029% | 0.237% | 0.118% | 0.000% |
| 0.01% | 0.001% | 0.133% | 0.187% | 0.000% |

Band by band below 1%, the worst share is 0.602% (sales, probability between 0.1% and 1%). With
ε = 1%, every excluded entry was irrelevant **at least 99.4% of the time**, in every list and
every band, and the same holds for ε = 10%.

The cost: with ε = 1% the filter removes half of the entries or more (catalogues: 4 811 entries
instead of 8 408), and loses 0 to 1.3 points of recall.

**What is not guaranteed:**

- **Probabilities below 0.1% are too small.** Where the library says 0.001%, in the catalogues
  the actual share is 0.19%. With ε = 0.1% or 0.01% the promise does not hold (sales and
  catalogues): don't use them.
- **At the top, probabilities are too large.** Among entries given above 99%, 15–41% are
  relevant: the probability says "this entry has what you typed", not "this is the one you
  meant" (see "pl").
- **The guarantee holds for queries like the simulated ones.** It was measured on these four
  lists and these queries; with very different lists or queries, measure it again with the bench.

### Speed

Apple M5, 10 processors. Milliseconds per query, mean over 200 queries with 3% typos. nucleo uses
one processor; the search uses all of them above 256 KB of text, and is also measured on one.

| list | entries | text | probable | probable, 1 processor | nucleo |
|---|---|---|---|---|---|
| sales | 1 167 | 0.1 MB | 0.23 | 0.23 | 0.30 |
| purchases | 5 147 | 0.4 MB | 0.67 | 0.94 | 0.77 |
| catalogues | 19 498 | 1.6 MB | 1.74 | 4.14 | 4.50 |
| dictionary | 235 976 | 2.3 MB | 1.78 | 3.41 | 2.09 |
| catalogues ×10 | 194 980 | 16 MB | 14.8 | 43.8 | 45.1 |
| catalogues ×50 | 974 900 | 81 MB | 80.3 | — | 223.6 |

On one processor the search is slower than nucleo on short entries (dictionary: 1.65 times) and
on par on long ones, while searching with typos. The index is built in 0.3 s for a million
entries.

## Limits

- At most 3 errors per word; words longer than 64 letters use the full table, which is slow.
- Query words are scored independently: two equal words count twice.
- No operators (`"phrase"`, `-word`, `^start`): they can be added on top.
- Splitting joined words tries a single cut, with both pieces exact.
- In an entry with fields, the first occurrence of a word counts: give the fields most important
  first.
- Domain rules stay with the caller: for example, a catalogue where a number is a measure (so
  "150" must not find "1500"), or phone numbers typed with the country prefix.

The details of every list, level and probability band are in
[bench/RESULTS.md](bench/RESULTS.md).

## License

MIT, see [LICENSE](LICENSE).
