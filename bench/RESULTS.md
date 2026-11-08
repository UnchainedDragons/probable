# Bench results

Produced with `cargo run --release -p probable-bench -- evaluate <list> <queries> <seed>` (seeds 11, 12, 13; 11 and 12 for the dictionary) and summarised per list and typo level: mean over the seeds, worst and best seed in brackets. The lists are not published. The `ms/query` column of these runs is indicative (other work was running on the machine); the speed tables at the end are the reference.


#### purchases — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.17% (92.3–97.2) | 0.086% | 92.1% | 95.0% | 0.495 | 155 | 0.71 |
| probable, no filter | 95.33% (92.4–97.2) | 0.097% | 92.1% | 95.0% | 0.495 | 836 | 0.71 |
| probable, all words | 94.21% (91.7–96.5) | 0.104% | 91.2% | 93.7% | 0.488 | 211 | 0.60 |
| nucleo (fzf) | 94.60% (93.1–96.2) | 0.105% | 85.3% | 89.4% | 0.450 | 585 | 0.74 |
| terms + Levenshtein | 87.19% (84.7–89.6) | 0.227% | 76.7% | 80.4% | 0.401 | 99 | 0.55 |
| contains | 90.91% (88.4–94.4) | 0.161% | 73.2% | 79.5% | 0.413 | 123 | 0.24 |

#### purchases — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 94.46% (91.7–97.3) | 0.091% | 88.9% | 92.7% | 0.491 | 147 | 0.78 |
| probable, no filter | 95.09% (92.1–97.6) | 0.094% | 88.9% | 92.7% | 0.491 | 869 | 0.78 |
| probable, all words | 93.60% (90.9–96.1) | 0.106% | 87.5% | 90.6% | 0.477 | 190 | 0.65 |
| nucleo (fzf) | 86.85% (83.2–89.8) | 0.237% | 71.3% | 76.4% | 0.366 | 553 | 0.73 |
| terms + Levenshtein | 82.66% (80.9–85.3) | 0.286% | 67.9% | 71.5% | 0.346 | 91 | 0.56 |
| contains | 77.42% (73.4–80.8) | 0.372% | 55.9% | 61.2% | 0.306 | 102 | 0.24 |

#### purchases — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 92.71% (90.7–93.8) | 0.129% | 82.0% | 87.7% | 0.453 | 172 | 0.84 |
| probable, no filter | 92.98% (90.9–94.2) | 0.145% | 82.0% | 87.9% | 0.453 | 873 | 0.84 |
| probable, all words | 89.58% (86.4–91.6) | 0.185% | 79.3% | 83.2% | 0.427 | 198 | 0.72 |
| nucleo (fzf) | 79.25% (78.2–80.9) | 0.404% | 56.5% | 60.4% | 0.273 | 573 | 0.71 |
| terms + Levenshtein | 69.50% (66.2–74.9) | 0.534% | 54.6% | 57.7% | 0.264 | 83 | 0.55 |
| contains | 61.36% (56.8–67.7) | 0.679% | 37.4% | 41.4% | 0.193 | 95 | 0.24 |

#### catalogues — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.24% (94.5–95.7) | 0.147% | 90.0% | 92.1% | 0.240 | 847 | 2.36 |
| probable, no filter | 95.63% (94.7–96.3) | 0.170% | 90.0% | 92.1% | 0.240 | 4761 | 2.36 |
| probable, all words | 95.13% (94.3–95.6) | 0.155% | 89.8% | 91.7% | 0.239 | 1380 | 1.85 |
| nucleo (fzf) | 95.45% (94.7–95.9) | 0.156% | 79.0% | 84.7% | 0.213 | 2671 | 4.42 |
| terms + Levenshtein | 85.12% (83.4–87.7) | 0.452% | 71.3% | 76.0% | 0.191 | 646 | 1.83 |
| contains | 92.51% (91.6–94.2) | 0.234% | 70.8% | 76.5% | 0.203 | 776 | 0.77 |

#### catalogues — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.71% (96.2–97.0) | 0.105% | 85.2% | 87.8% | 0.220 | 931 | 2.46 |
| probable, no filter | 97.16% (96.6–97.5) | 0.115% | 85.2% | 87.9% | 0.220 | 4864 | 2.46 |
| probable, all words | 96.56% (96.1–96.9) | 0.112% | 84.6% | 87.0% | 0.216 | 1365 | 1.95 |
| nucleo (fzf) | 87.81% (85.2–91.3) | 0.428% | 69.3% | 73.4% | 0.173 | 2643 | 4.21 |
| terms + Levenshtein | 83.43% (80.8–86.0) | 0.509% | 64.1% | 68.2% | 0.159 | 647 | 1.78 |
| contains | 79.85% (77.9–83.5) | 0.629% | 54.1% | 58.8% | 0.139 | 707 | 0.75 |

#### catalogues — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 93.07% (92.2–94.5) | 0.230% | 78.6% | 81.2% | 0.207 | 1041 | 2.77 |
| probable, no filter | 93.77% (93.0–94.9) | 0.263% | 78.6% | 81.2% | 0.207 | 5014 | 2.77 |
| probable, all words | 92.49% (91.5–94.1) | 0.253% | 77.2% | 79.1% | 0.200 | 1352 | 2.23 |
| nucleo (fzf) | 78.16% (75.2–80.2) | 0.787% | 56.6% | 61.0% | 0.138 | 2512 | 4.19 |
| terms + Levenshtein | 74.25% (71.9–77.3) | 0.836% | 51.2% | 55.8% | 0.128 | 618 | 1.83 |
| contains | 65.28% (60.7–70.2) | 1.129% | 38.3% | 42.4% | 0.093 | 627 | 0.74 |

#### dictionary — no typos (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.85% (97.3–98.4) | 0.000% | 86.2% | 95.7% | 0.891 | 22 | 0.66 |
| probable, no filter | 97.85% (97.3–98.4) | 0.000% | 86.2% | 95.7% | 0.891 | 1079 | 0.66 |
| probable, all words | 97.85% (97.3–98.4) | 0.000% | 86.2% | 95.7% | 0.891 | 434 | 0.54 |
| nucleo (fzf) | 97.85% (97.3–98.4) | 0.000% | 84.5% | 94.2% | 0.875 | 278 | 2.12 |
| terms + Levenshtein | 91.25% (90.4–92.1) | 0.000% | 78.7% | 87.8% | 0.816 | 10 | 2.45 |
| contains | 97.85% (97.3–98.4) | 0.000% | 84.8% | 94.3% | 0.879 | 29 | 6.96 |

#### dictionary — 3% typos per letter (what the model assumes) (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.36% (95.7–97.0) | 0.000% | 84.8% | 94.7% | 0.879 | 92 | 1.19 |
| probable, no filter | 96.53% (95.7–97.3) | 0.000% | 84.8% | 94.7% | 0.879 | 1506 | 1.19 |
| probable, all words | 96.53% (95.7–97.3) | 0.000% | 84.8% | 94.7% | 0.879 | 472 | 1.06 |
| nucleo (fzf) | 80.46% (80.1–80.8) | 0.000% | 69.8% | 78.3% | 0.724 | 291 | 2.13 |
| terms + Levenshtein | 81.62% (80.5–82.8) | 0.000% | 70.3% | 79.8% | 0.736 | 18 | 2.46 |
| contains | 77.98% (77.5–78.5) | 0.000% | 68.7% | 76.2% | 0.710 | 87 | 6.92 |

#### dictionary — 8% typos per letter (more than the model) (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 92.23% (91.1–93.4) | 0.000% | 76.3% | 89.7% | 0.808 | 26 | 1.71 |
| probable, no filter | 94.55% (94.0–95.0) | 0.000% | 76.3% | 90.5% | 0.811 | 816 | 1.71 |
| probable, all words | 94.38% (93.7–95.0) | 0.000% | 76.3% | 90.5% | 0.811 | 381 | 1.58 |
| nucleo (fzf) | 57.86% (57.8–58.0) | 0.000% | 46.4% | 54.0% | 0.490 | 128 | 2.13 |
| terms + Levenshtein | 68.59% (66.9–70.3) | 0.000% | 54.5% | 66.2% | 0.591 | 6 | 2.48 |
| contains | 49.92% (49.0–50.8) | 0.000% | 41.0% | 47.2% | 0.431 | 16 | 7.21 |

#### sales — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.70% (96.4–97.1) | 0.200% | 93.0% | 94.5% | 0.445 | 86 | 0.34 |
| probable, no filter | 97.67% (97.2–98.2) | 0.177% | 93.0% | 94.6% | 0.445 | 300 | 0.34 |
| probable, all words | 96.52% (95.9–96.9) | 0.214% | 92.5% | 93.8% | 0.441 | 99 | 0.29 |
| nucleo (fzf) | 96.84% (96.5–97.1) | 0.211% | 87.7% | 91.6% | 0.361 | 191 | 0.30 |
| terms + Levenshtein | 87.78% (86.5–88.9) | 0.721% | 78.6% | 81.3% | 0.312 | 64 | 0.12 |
| contains | 93.60% (92.4–94.3) | 0.379% | 77.3% | 81.0% | 0.362 | 72 | 0.06 |

#### sales — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.24% (95.1–98.1) | 0.235% | 90.2% | 91.9% | 0.422 | 92 | 0.36 |
| probable, no filter | 97.33% (96.4–98.6) | 0.208% | 90.2% | 92.1% | 0.423 | 310 | 0.36 |
| probable, all words | 95.37% (94.6–96.1) | 0.289% | 89.3% | 90.5% | 0.413 | 101 | 0.31 |
| nucleo (fzf) | 90.75% (89.6–92.4) | 0.623% | 78.2% | 80.3% | 0.290 | 191 | 0.30 |
| terms + Levenshtein | 85.01% (82.8–86.2) | 0.898% | 70.4% | 72.9% | 0.262 | 64 | 0.12 |
| contains | 81.04% (78.0–83.6) | 1.136% | 59.2% | 62.3% | 0.263 | 67 | 0.06 |

#### sales — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 93.01% (91.2–94.6) | 0.436% | 85.2% | 88.5% | 0.418 | 92 | 0.36 |
| probable, no filter | 93.74% (92.2–94.9) | 0.479% | 85.2% | 88.8% | 0.419 | 293 | 0.36 |
| probable, all words | 90.53% (89.0–91.7) | 0.588% | 83.6% | 85.7% | 0.402 | 92 | 0.31 |
| nucleo (fzf) | 80.84% (78.3–82.5) | 1.278% | 64.9% | 66.6% | 0.230 | 169 | 0.27 |
| terms + Levenshtein | 73.40% (70.8–76.5) | 1.579% | 56.8% | 59.4% | 0.220 | 55 | 0.12 |
| contains | 63.37% (61.3–64.5) | 2.182% | 39.2% | 41.4% | 0.167 | 52 | 0.06 |

### Calibration (all seeds and levels)


#### purchases

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 3657982 | 28.0 | 1 | 0.000% |
| [0.0001, 0.001) | 1141816 | 620.9 | 371 | 0.032% |
| [0.001, 0.01) | 1511307 | 5749.6 | 2329 | 0.154% |
| [0.01, 0.1) | 339703 | 9424.4 | 3305 | 0.973% |
| [0.1, 0.3) | 58087 | 10729.1 | 4785 | 8.238% |
| [0.3, 0.5) | 37774 | 16244.2 | 5728 | 15.164% |
| [0.5, 0.7) | 31449 | 18235.7 | 13845 | 44.024% |
| [0.7, 0.9) | 39628 | 32621.3 | 22298 | 56.268% |
| [0.9, 0.99) | 155362 | 150753.8 | 133773 | 86.104% |
| ≥ 0.99 | 762556 | 761704.3 | 559346 | 73.351% |
| no word found | 38587336 | 0.0 | 43182 | 0.112% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 3657982 | 1 | 0.000% |
| 0.001 | 4799798 | 372 | 0.008% |
| 0.01 | 6311105 | 2701 | 0.043% |
| 0.1 | 6650808 | 6006 | 0.090% |

#### catalogues

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 17829066 | 234.0 | 128 | 0.001% |
| [0.0001, 0.001) | 4613494 | 1776.3 | 349 | 0.008% |
| [0.001, 0.01) | 13015829 | 54009.3 | 27114 | 0.208% |
| [0.01, 0.1) | 2053599 | 61017.1 | 21907 | 1.067% |
| [0.1, 0.3) | 405152 | 72664.6 | 22603 | 5.579% |
| [0.3, 0.5) | 261026 | 107833.7 | 28035 | 10.740% |
| [0.5, 0.7) | 209752 | 119753.1 | 50878 | 24.256% |
| [0.7, 0.9) | 286095 | 237935.1 | 197258 | 68.948% |
| [0.9, 0.99) | 927736 | 898913.9 | 707392 | 76.249% |
| ≥ 0.99 | 4314260 | 4308548.8 | 4031818 | 93.453% |
| no word found | 131565991 | 0.0 | 240336 | 0.183% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 17829066 | 128 | 0.001% |
| 0.001 | 22442560 | 477 | 0.002% |
| 0.01 | 35458389 | 27591 | 0.078% |
| 0.1 | 37511988 | 49498 | 0.132% |

#### dictionary

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 0 | 0.0 | 0 | 0.000% |
| [0.0001, 0.001) | 4502 | 3.0 | 0 | 0.000% |
| [0.001, 0.01) | 1951213 | 10074.8 | 15 | 0.001% |
| [0.01, 0.1) | 80665 | 1582.7 | 141 | 0.175% |
| [0.1, 0.3) | 2005 | 476.3 | 89 | 4.439% |
| [0.3, 0.5) | 185 | 75.2 | 73 | 39.459% |
| [0.5, 0.7) | 88 | 48.4 | 52 | 59.091% |
| [0.7, 0.9) | 98 | 80.7 | 73 | 74.490% |
| [0.9, 0.99) | 272 | 259.3 | 246 | 90.441% |
| ≥ 0.99 | 1095 | 1093.5 | 1059 | 96.712% |
| no word found | 422716677 | 0.0 | 67 | 0.000% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 0 | 0 | 0.000% |
| 0.001 | 4502 | 0 | 0.000% |
| 0.01 | 1955715 | 15 | 0.001% |
| 0.1 | 2036380 | 156 | 0.008% |

#### sales

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 839498 | 11.7 | 8 | 0.001% |
| [0.0001, 0.001) | 257030 | 110.8 | 264 | 0.103% |
| [0.001, 0.01) | 801935 | 3369.0 | 5279 | 0.658% |
| [0.01, 0.1) | 163322 | 3980.5 | 2303 | 1.410% |
| [0.1, 0.3) | 23577 | 3895.7 | 2509 | 10.642% |
| [0.3, 0.5) | 16652 | 6787.8 | 2947 | 17.698% |
| [0.5, 0.7) | 13356 | 7547.2 | 3793 | 28.399% |
| [0.7, 0.9) | 17815 | 14612.5 | 11641 | 65.344% |
| [0.9, 0.99) | 94653 | 91797.8 | 88471 | 93.469% |
| ≥ 0.99 | 480959 | 480357.6 | 453389 | 94.268% |
| no word found | 7794203 | 0.0 | 22532 | 0.289% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 839498 | 8 | 0.001% |
| 0.001 | 1096528 | 272 | 0.025% |
| 0.01 | 1898463 | 5551 | 0.292% |
| 0.1 | 2061785 | 7854 | 0.381% |

## Speed (all processors above 256 KB; nucleo on one)

## sales ×1: 1167 entries, 0.1 MB
index: 583.875µs
probable: 321.886µs per query; nucleo: 291.549µs per query

## purchases ×1: 5147 entries, 0.4 MB
index: 2.588333ms
probable: 781.254µs per query; nucleo: 713.299µs per query

## catalogues ×1: 19498 entries, 1.6 MB
index: 10.173459ms
probable: 2.296302ms per query; nucleo: 4.469228ms per query

## dictionary ×1: 235976 entries, 2.3 MB
index: 13.625875ms
probable: 1.175186ms per query; nucleo: 2.077218ms per query

## catalogues ×10: 194980 entries, 16.0 MB
index: 103.457ms
probable: 20.928614ms per query; nucleo: 44.235434ms per query

## catalogues ×50: 974900 entries, 81.2 MB
index: 538.687458ms
probable: 111.046232ms per query; nucleo: 219.03723ms per query

## Speed on one processor

## purchases ×1: 5147 entries, 0.4 MB
index: 5.312875ms
probable: 1.243814ms per query; nucleo: 714.946µs per query

## catalogues ×1: 19498 entries, 1.6 MB
index: 10.150625ms
probable: 4.738799ms per query; nucleo: 4.276935ms per query

## dictionary ×1: 235976 entries, 2.3 MB
index: 12.940458ms
probable: 2.180476ms per query; nucleo: 2.073992ms per query

## catalogues ×10: 194980 entries, 16.0 MB
index: 101.733291ms
probable: 49.888909ms per query; nucleo: 42.99498ms per query

