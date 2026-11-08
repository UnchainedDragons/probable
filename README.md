# probable

Approximate search in a list of strings, in Rust, with no dependencies.

- **Typos.** Each query word is searched on its own, anywhere in the text and in any order, with
  a few typos allowed. The distance is Damerau-Levenshtein: insertion, deletion, substitution or
  swap of two adjacent letters, one error each. Short words allow fewer errors.
- **Case, accents and separators don't matter**, in the text or in the query: "multilayer"
  finds "MULTI-LAYER" and "multi layer" finds "Multilayer". Two words typed together are found
  whether or not they are next to each other in the text ("smith27315").
- **One model of how people type a query** gives both numbers of every entry: the **score**,
  which orders the results, and the **probability** that the entry has what each typed word
  stands for, which the filter compares with a threshold ε (1%).

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
| `max_errors` | 2 | maximum errors per word; fewer for short words (0 up to 3 letters, 1 up to 5, 2 up to 9, then 3) |
| `depth` | 1.0 | β: the word at position `i` of a field is `(i+1)^β` times less likely to be the one typed than the first; 0 turns it off |
| `exclude_below` | `Some(0.01)` | the filter's ε; `None` keeps every entry where something was found |
| `require_all_words` | `false` | every typed word must stand for something in the entry (the classic "and" search); without it a word found in no entry is left out |
| `typo_rate` | 0.03 | ρ, the probability of mistyping a letter |
| `missing_word` | 0.01 | η, the probability that a typed word is not in the wanted entry |
| `endings` | `&Endings::ITALIAN` | the endings of the language of the list: the filter counts "tubi" as the word "tubo", with the probability that they are the same word (see "Other forms of a word"); `&Endings::NONE` for the same letters only |
| `exact_numbers` | `false` | numbers typed must appear whole: "150" does not find "1500", "acm12" does not find "ACM-18"; for catalogues where numbers are sizes or codes (it gets in the way of typing a number digit by digit) |

## How it works

### 1. Folded text

Every letter or digit becomes one or more lowercase ASCII bytes ("È" → `e`, "ß" → `ss`,
"²" → `2`, from the NFKD decomposition); everything else disappears. Word starts stay marked:
after a separator, between letters and digits ("RAC100MF" → `rac` `100` `mf`), and from
lowercase to uppercase. Non-Latin letters stay themselves, with a code of their own.

### 2. The model

Someone wants an entry `k`. For each word they type:

- they pick a word `t` of `k`. Words near the start of their field and in heavier fields are
  picked more often: `P(t | k) ∝ w_field · (i + 1)^-β`, normalised over the field (`i` is the
  position of `t` in its field);
- they type it **whole**, or only its **beginning** because the search runs while they type
  (probability μ = 0.3; the length typed so far is any of `1..=L`, the whole word included), or a
  **piece from inside** it, as with codes (ι = 0.1; start and length uniform);
- they may type it **together with the next word**, the space left out, or with a **space too
  many** inside, or type **two words of the entry as one** (σ = 0.05 per space);
- every letter is mistyped with probability ρ = 3%, and a mistake is one of `A` letters (`A` is
  the perplexity of the letters of the list, about 20). A piece at distance `d` from the typed
  word of `m` letters costs `(1 − ρ)^(m − d) (ρ / A)^d`;
- or, with probability η = 1%, they type something that is not in `k`: a word of some other entry
  (λ = ½), or a new string (letters with the frequencies of the list, length geometric with the
  mean length of a word).

So, for a typed word `w`, with `S_k = Σ_t P(t | k) P(w | t)` over the pieces of `k` and `S` its
mean over the `N` entries searched:

```
P(w | k) = (1 − η) S_k + η (λ S + (1 − λ) P(w as a new string))
```

and for the query, summing over the ways of reading it (each word alone, two words as one, one
word as two), each with its prior:

```
P(query | k) = Σ_readings prior · Π_units P(unit | k)
```

**The score** is `log10 P(query | k) / P(query)`, with `P(query)` the mean of `P(query | k)`
over the entries searched: how much more likely the query is if `k` is the wanted entry than if
it is any entry. It orders the results; on equal score the given order stays.

**The probability** is that entry `i` has what each typed word stands for. The words come from the
same wanted entry, so this is a sum over the entries `k` that may be the wanted one:

```
P(i has it) = Σ_k P(k | query) · P(the pieces chosen in k are also in i)
            = Σ_k Σ_readings prior · Π_units (Σ of P(t | k) P(w | t) over the pieces t of k
              whose letters i has too) / Σ_k P(query | k)
```

Without `require_all_words`, "not in the wanted entry" also counts as had. Every number in these
formulas is a probability of the model: nothing is fitted to the test bench, and the constants
were fixed before the final measures.

Two consequences, with no rule written for them:

- **Typos only when needed.** "term" finds "termostato" and "terme": "thermostat" and
  "tergicristallo" are one error away, which costs about `ρ / A ≈ 1/600`, and they are left out
  unless they are hundreds of times more common. "termostsato" is in no entry, so the nearest
  words come back.
- **Next to another word, a short word narrows.** In "valvola sf", "sf" begins many words of the
  list, but next to "valvola" it stands for a word found with "valvola".

### Other forms of a word

In many languages a word changes its ending for number and gender: "tubo", "tubi"; "rosso",
"rossa". For the model above "tubi" is "tubo" with a typo, and when "tubo" is in the list exactly,
the entries that only have "tubi" fall below the filter. They have the word typed, though: what
they lack is the same letters.

So the filter asks whether the entry has *the word*, in any form. A piece of the entry that is
another form of a piece the query stands for counts with the probability that the two are the
same word, instead of zero:

```
logit P(same word) = α[ending a ↔ ending b] + β[letters in the stem]
```

For Italian (`Endings::ITALIAN`) these are 31 numbers: 23 pairs of endings (e↔i, o↔i, a↔e, a↔o,
co↔chi, …) and 8 corrections for short stems, where two forms are more often different words
("caso", "casa"). They are counted, by maximum likelihood, on the 163 882 pairs of nouns,
adjectives and past participles of the Morph-it! lexicon (University of Bologna) that differ only
in their ending; only the counts are kept. Out of sample they are worth as much as a table with a
probability for every pair of endings and every stem length (0.121 against 0.120 bits per pair;
without the stem lengths, 0.138). Some values: tubo/tubi 0.98, rosso/rossa 0.95, costo/costa
0.84, spa/spi 0.56.

Only the filter changes: the score and the order stay those of the model, so the entries with the
form typed come first. And since a piece can only add to an entry's probability, the filter
never leaves out an entry it would keep with `Endings::NONE`; the tests check it on random lists.
Words of different meaning with the same stem (costo/costa) are kept, at the bottom, with the
probability the endings give them.

Another language needs the same 31 numbers counted on its own forms: from an inflected lexicon
(UniMorph has inflection tables for more than 150 languages), or from real searches (a word typed
with one ending, an entry chosen with another).

### 3. Finding the pieces: Myers and Hyyrö

For each query word (up to 64 letters), Myers' automaton (1999), in Hyyrö's form with
transpositions (2003), scans the text and gives, at every position, the minimum distance between
the word and a piece of text ending there. One column of the distance table fits in a `u64`:
about a dozen operations per character, whatever the tolerance. Where that distance is within the
tolerance, the same automaton anchored at the nearby word starts gives the distance of the word
whole, begun and with the following words; and run inside the word, the distance of the pieces
from inside. A piece within the tolerance ends where the free scan is within it too, and starts
within `k` letters of `j + 1 − m`: so every piece is found, and the tests check it against all the
pieces by definition.

Three tricks make it fast without changing the result:

- **missing letters**: each letter of the word that the entry lacks costs at least one error, so
  an entry missing more letters than the tolerance is not scanned;
- **several entries at once**: each character depends on the previous one, but entries are
  independent; scanning six of similar length interleaved keeps the processor busy in parallel;
- **several processors**: above 256 KB of text.

What is not worked out, because it is worth too little to change a result: pieces with more
errors than the tolerance; readings with three or more mistakes about spaces (σ³); for a word
typed as two, cuts other than the one with both halves exact in the most entries, and any cut when
the word itself is in some entry exactly; per entry and reading, more than 256 combinations of
pieces (the least likely go).

## The tests

Everything can be rerun with `cargo test --release` and
`cargo run --release -p probable-bench -- evaluate <list.txt>`.

### Correctness tests

42 tests, including:

- the automaton against the full distance table on 200 000 random cases (words up to 64
  letters), free and anchored, and the full table against the definition;
- the pieces found from the automaton against **all the pieces by definition** (every word start,
  every way of reading on, every piece inside every word, by brute force) on random lists with
  fields;
- the interleaved scan against scanning one entry at a time, and the parallel scan against the
  serial one;
- the endings never leave out an entry kept without them, on 3 000 random lists and queries;
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

The model was built looking at the purchases. One choice also looked at the other lists: errors
are allowed from four letters, not three, after typing simulated queries letter by letter on
purchases, sales and catalogues (see "Typing"). Below is the mean over 3 seeds (2 for the
dictionary), 1 000 queries per seed and level (300 for the dictionary); the worst and best seed
are in brackets. These figures were measured with `Endings::NONE`. With the Italian endings, on
the three lists of things, recall stays the same within 0.1 points and precision is about one
point lower: the simulated queries use the exact words of the entries, so they hardly test other
forms of a word (see "Other forms of a word").

The other methods:

- **nucleo**: Helix's matcher with fzf's rules (each word as a subsequence, all required);
- **terms**: whole words, prefixes and Levenshtein 1–2 on words of at least 4 letters, IDF score;
- **contains**: each word contained as it is.

#### With 3% typos per letter (what the model assumes)

| list | method | recall | precision | 1st entry relevant | MRR of the source entry | entries kept |
|---|---|---|---|---|---|---|
| purchases | **probable, filter 1%** | **94.5%** (91.7–97.3) | 54.1% | **88.9%** | **0.491** | 147 |
| | nucleo | 86.9% (83.2–89.8) | 13.1% | 71.3% | 0.366 | 553 |
| | terms | 82.7% | 76.3% | 67.9% | 0.346 | 91 |
| | contains | 77.4% | 63.6% | 55.9% | 0.306 | 102 |
| sales | **probable, filter 1%** | **96.2%** (95.1–98.1) | 69.1% | **90.2%** | **0.422** | 92 |
| | nucleo | 90.8% (89.6–92.4) | 31.4% | 78.2% | 0.290 | 191 |
| | terms | 85.0% | 88.3% | 70.4% | 0.262 | 64 |
| | contains | 81.0% | 80.1% | 59.2% | 0.263 | 67 |
| catalogues | **probable, filter 1%** | **96.7%** (96.2–97.0) | 60.6% | **85.2%** | **0.220** | 931 |
| | nucleo | 87.8% (85.2–91.3) | 19.4% | 69.3% | 0.173 | 2 643 |
| | terms | 83.4% | 75.4% | 64.1% | 0.159 | 647 |
| | contains | 79.9% | 65.9% | 54.1% | 0.139 | 707 |
| dictionary | **probable, filter 1%** | **96.4%** (95.7–97.0) | 1.4% | **84.8%** | **0.879** | 92 |
| | nucleo | 80.5% (80.1–80.8) | 0.3% | 69.8% | 0.724 | 291 |
| | terms | 81.6% | 5.1% | 70.3% | 0.736 | 18 |
| | contains | 78.0% | 1.0% | 68.7% | 0.710 | 87 |

Without typos the search equals nucleo: recall 95.2–97.9% against 94.6–97.9%, first entry
relevant 86.2–93.0% against 79.0–87.7%. With 8% typos the gap widens: recall 92.2–93.1% against
57.9–80.8% for nucleo.

Precision (how many kept entries are relevant) is 54–69% on the lists of things, three to four
times nucleo's: the filter keeps what the query points to, not everything that resembles it. In
the dictionary it stays low for every method that tolerates typos: someone typing "pl" may want
"plus" or "plastic", and no search can know. That is why the order matters: the first relevant
entry is at the top 85–90% of the time.

#### Typing

The results while typing, letter by letter: 300 simulated queries per list (one or two words of a
random entry), counting the keystrokes after which there are *more* results than before.

| list | keystrokes | more results after the keystroke | the wanted entry lost at the end |
|---|---|---|---|
| purchases | 2 704 | 3.3% | 0 |
| sales | 2 935 | 2.2% | 0 |
| catalogues | 2 606 | 1.8% | 0 |

The rest are mostly a very common word of the list (a supplier's name in thousands of entries)
one typo away from what has been typed so far: the model gives those entries 1–5%, and they stay
until the next letter tells them apart.

#### The filter

**How many relevant entries there are among those with probability below ε**, over all seeds and
typo levels:

| ε | purchases | sales | catalogues | dictionary |
|---|---|---|---|---|
| 10% | 0.090% | 0.381% | 0.132% | 0.008% |
| **1%** | **0.043%** | **0.292%** | **0.078%** | **0.001%** |
| 0.1% | 0.008% | 0.025% | 0.002% | 0.000% |
| 0.01% | 0.000% | 0.001% | 0.001% | — |

And band by band below 1%: the worst share is 0.658% (sales, probability between 0.1% and 1%).
With ε = 1%, every excluded entry was irrelevant **at least 99.3% of the time**, in every list and
every band; with ε = 10%, at least 98.6%.

The cost: with ε = 1% the filter removes most entries (catalogues: 931 entries instead of 4 864),
and loses 0.2 to 1.1 points of recall.

**What is not guaranteed:**

- **ε = 0.1% is at the limit.** In the sales, entries given between 0.01% and 0.1% were relevant
  0.103% of the time. Below that the probabilities hold (worst 0.001%).
- **Above 10%, probabilities are too large.** Among entries given above 99%, 73–97% are relevant;
  between 50% and 70%, 24–59%. Part of it is the bench's own definition: it counts as words only
  what lies between separators, so for it "spa" is not in "S.P.A." nor "mm" in "10mm", which the
  search finds on purpose; and a query with a word that is not in the source entry often finds
  entries that do have it.
- **The guarantee holds for queries like the simulated ones.** It was measured on these four
  lists and these queries; with very different lists or queries, measure it again with the bench.

### Speed

Apple M5, 10 processors. Milliseconds per query, mean over 200 queries with 3% typos. nucleo uses
one processor; the search uses all of them above 256 KB of text, and is also measured on one.

| list | entries | text | probable | probable, 1 processor | nucleo |
|---|---|---|---|---|---|
| sales | 1 167 | 0.1 MB | 0.32 | 0.32 | 0.29 |
| purchases | 5 147 | 0.4 MB | 0.78 | 1.24 | 0.71 |
| catalogues | 19 498 | 1.6 MB | 2.30 | 4.74 | 4.47 |
| dictionary | 235 976 | 2.3 MB | 1.18 | 2.18 | 2.08 |
| catalogues ×10 | 194 980 | 16 MB | 20.9 | 49.9 | 44.2 |
| catalogues ×50 | 974 900 | 81 MB | 111 | — | 219 |

On one processor the search is 5–75% slower than nucleo, while searching with typos. The index
is built in 0.54 s for a million entries.

## Limits

- The endings are Italian unless told otherwise; for another language pass `Endings::NONE` until
  its numbers are counted. They also join different words with the same stem (costo/costa) with
  the probability their endings give.
- At most 3 errors per word, none up to 3 letters; words longer than 64 letters use the full
  table, which is slow.
- A word typed as two words of the entry joined is split at a single cut, with both pieces exact
  ("valvebrass" is found, "valvebrsas" is not).
- No operators (`"phrase"`, `-word`, `^start`): they can be added on top.
- `exact_numbers` gets in the way of typing a number digit by digit ("1" does not find "123"):
  use it where numbers are typed whole, like catalogue measures.
- Domain rules stay with the caller: for example, a catalogue where a number is a measure (so
  "150" must not find "1500"), or phone numbers typed with the country prefix.

The details of every list, level and probability band are in
[bench/RESULTS.md](bench/RESULTS.md).

## License

MIT, see [LICENSE](LICENSE).
