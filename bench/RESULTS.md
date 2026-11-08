# Bench results

Produced with `cargo run --release -p probable-bench -- evaluate <list> <queries> <seed>` (seeds 11, 12, 13; 11 and 12 for the dictionary) and summarised per list and typo level: mean over the seeds, worst and best seed in brackets. The lists are not published. The `ms/query` column of these runs is indicative (other work was running on the machine); the speed tables at the end are the reference.


#### purchases — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.40% (92.7–97.1) | 0.089% | 89.9% | 93.5% | 0.488 | 524 | 0.82 |
| probable, no filter | 95.50% (92.8–97.3) | 0.105% | 89.9% | 93.5% | 0.488 | 1347 | 0.82 |
| probable, all words | 94.48% (92.0–96.4) | 0.099% | 89.5% | 92.6% | 0.482 | 186 | 0.76 |
| nucleo (fzf) | 94.60% (93.1–96.2) | 0.105% | 85.3% | 89.4% | 0.450 | 585 | 0.73 |
| terms + Levenshtein | 87.19% (84.7–89.6) | 0.227% | 76.7% | 80.4% | 0.401 | 99 | 0.55 |
| contains | 90.91% (88.4–94.4) | 0.161% | 73.2% | 79.5% | 0.413 | 123 | 0.24 |

#### purchases — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.62% (93.3–97.5) | 0.077% | 86.5% | 91.4% | 0.480 | 510 | 1.00 |
| probable, no filter | 96.59% (94.6–97.9) | 0.073% | 86.6% | 91.5% | 0.480 | 1343 | 1.00 |
| probable, all words | 93.70% (91.7–96.0) | 0.105% | 84.8% | 88.8% | 0.464 | 171 | 0.92 |
| nucleo (fzf) | 86.85% (83.2–89.8) | 0.237% | 71.3% | 76.4% | 0.366 | 553 | 0.83 |
| terms + Levenshtein | 82.66% (80.9–85.3) | 0.286% | 67.9% | 71.5% | 0.346 | 91 | 0.62 |
| contains | 77.42% (73.4–80.8) | 0.372% | 55.9% | 61.2% | 0.306 | 102 | 0.26 |

#### purchases — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 93.77% (91.2–95.6) | 0.118% | 79.7% | 86.6% | 0.444 | 505 | 1.11 |
| probable, no filter | 95.80% (93.7–98.0) | 0.098% | 79.8% | 86.9% | 0.445 | 1349 | 1.11 |
| probable, all words | 90.78% (88.3–93.2) | 0.163% | 75.5% | 80.9% | 0.407 | 180 | 1.02 |
| nucleo (fzf) | 79.25% (78.2–80.9) | 0.404% | 56.5% | 60.4% | 0.273 | 573 | 0.91 |
| terms + Levenshtein | 69.50% (66.2–74.9) | 0.534% | 54.6% | 57.7% | 0.264 | 83 | 0.68 |
| contains | 61.36% (56.8–67.7) | 0.679% | 37.4% | 41.4% | 0.193 | 95 | 0.30 |

#### catalogues — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.71% (94.7–96.6) | 0.163% | 86.5% | 88.3% | 0.233 | 4207 | 3.73 |
| probable, no filter | 95.99% (94.9–96.6) | 0.211% | 86.5% | 88.3% | 0.233 | 8559 | 3.73 |
| probable, all words | 95.31% (94.3–96.0) | 0.151% | 86.5% | 87.9% | 0.232 | 1489 | 3.31 |
| nucleo (fzf) | 95.45% (94.7–95.9) | 0.156% | 79.0% | 84.7% | 0.213 | 2671 | 5.35 |
| terms + Levenshtein | 85.12% (83.4–87.7) | 0.452% | 71.3% | 76.0% | 0.191 | 646 | 2.12 |
| contains | 92.51% (91.6–94.2) | 0.234% | 70.8% | 76.5% | 0.203 | 776 | 0.87 |

#### catalogues — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.80% (96.1–97.7) | 0.124% | 82.7% | 84.6% | 0.216 | 4279 | 2.87 |
| probable, no filter | 97.85% (96.8–98.8) | 0.116% | 82.8% | 84.7% | 0.216 | 8415 | 2.87 |
| probable, all words | 94.47% (93.9–95.1) | 0.180% | 82.0% | 83.5% | 0.212 | 1478 | 2.63 |
| nucleo (fzf) | 87.81% (85.2–91.3) | 0.428% | 69.3% | 73.4% | 0.173 | 2643 | 4.34 |
| terms + Levenshtein | 83.43% (80.8–86.0) | 0.509% | 64.1% | 68.2% | 0.159 | 647 | 1.87 |
| contains | 79.85% (77.9–83.5) | 0.629% | 54.1% | 58.8% | 0.139 | 707 | 0.76 |

#### catalogues — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 93.26% (91.9–94.0) | 0.268% | 74.4% | 78.0% | 0.202 | 4059 | 2.93 |
| probable, no filter | 96.02% (94.8–97.2) | 0.220% | 74.6% | 78.2% | 0.202 | 8427 | 2.93 |
| probable, all words | 92.66% (90.6–93.7) | 0.250% | 72.4% | 75.1% | 0.193 | 1431 | 2.66 |
| nucleo (fzf) | 78.16% (75.2–80.2) | 0.787% | 56.6% | 61.0% | 0.138 | 2512 | 4.31 |
| terms + Levenshtein | 74.25% (71.9–77.3) | 0.836% | 51.2% | 55.8% | 0.128 | 618 | 1.91 |
| contains | 65.28% (60.7–70.2) | 1.129% | 38.3% | 42.4% | 0.093 | 627 | 0.75 |

#### dictionary — no typos (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.85% (97.3–98.4) | 0.000% | 84.7% | 94.2% | 0.876 | 252 | 1.93 |
| probable, no filter | 97.85% (97.3–98.4) | 0.000% | 84.7% | 94.2% | 0.876 | 2142 | 1.93 |
| probable, all words | 97.85% (97.3–98.4) | 0.000% | 84.7% | 94.2% | 0.876 | 262 | 1.76 |
| nucleo (fzf) | 97.85% (97.3–98.4) | 0.000% | 84.5% | 94.2% | 0.875 | 278 | 2.16 |
| terms + Levenshtein | 91.25% (90.4–92.1) | 0.000% | 78.7% | 87.8% | 0.816 | 10 | 2.53 |
| contains | 97.85% (97.3–98.4) | 0.000% | 84.8% | 94.3% | 0.879 | 29 | 7.04 |

#### dictionary — 3% typos per letter (what the model assumes) (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.53% (95.7–97.3) | 0.000% | 83.8% | 94.5% | 0.871 | 304 | 2.10 |
| probable, no filter | 96.53% (95.7–97.3) | 0.000% | 83.8% | 94.5% | 0.871 | 2776 | 2.10 |
| probable, all words | 96.36% (95.7–97.0) | 0.000% | 84.0% | 94.5% | 0.873 | 298 | 1.91 |
| nucleo (fzf) | 80.46% (80.1–80.8) | 0.000% | 69.8% | 78.3% | 0.724 | 291 | 2.20 |
| terms + Levenshtein | 81.62% (80.5–82.8) | 0.000% | 70.3% | 79.8% | 0.736 | 18 | 2.58 |
| contains | 77.98% (77.5–78.5) | 0.000% | 68.7% | 76.2% | 0.710 | 87 | 7.12 |

#### dictionary — 8% typos per letter (more than the model) (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 94.71% (93.7–95.7) | 0.000% | 75.5% | 89.3% | 0.805 | 263 | 2.23 |
| probable, no filter | 95.21% (94.4–96.0) | 0.000% | 75.7% | 89.5% | 0.806 | 2584 | 2.23 |
| probable, all words | 93.88% (92.4–95.4) | 0.000% | 74.8% | 88.5% | 0.798 | 256 | 2.06 |
| nucleo (fzf) | 57.86% (57.8–58.0) | 0.000% | 46.4% | 54.0% | 0.490 | 128 | 2.12 |
| terms + Levenshtein | 68.59% (66.9–70.3) | 0.000% | 54.5% | 66.2% | 0.591 | 6 | 2.50 |
| contains | 49.92% (49.0–50.8) | 0.000% | 41.0% | 47.2% | 0.431 | 16 | 7.10 |

#### sales — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.73% (97.4–98.2) | 0.161% | 91.2% | 93.9% | 0.436 | 240 | 0.51 |
| probable, no filter | 98.04% (97.7–98.4) | 0.167% | 91.2% | 93.9% | 0.436 | 400 | 0.51 |
| probable, all words | 96.80% (96.5–97.0) | 0.194% | 91.0% | 93.4% | 0.434 | 87 | 0.48 |
| nucleo (fzf) | 96.84% (96.5–97.1) | 0.211% | 87.7% | 91.6% | 0.361 | 191 | 0.39 |
| terms + Levenshtein | 87.78% (86.5–88.9) | 0.721% | 78.6% | 81.3% | 0.312 | 64 | 0.15 |
| contains | 93.60% (92.4–94.3) | 0.379% | 77.3% | 81.0% | 0.362 | 72 | 0.08 |

#### sales — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.21% (95.5–98.4) | 0.195% | 88.1% | 90.9% | 0.414 | 239 | 0.53 |
| probable, no filter | 98.19% (97.6–98.5) | 0.156% | 88.2% | 91.0% | 0.414 | 402 | 0.53 |
| probable, all words | 94.89% (93.0–96.3) | 0.310% | 86.9% | 89.1% | 0.403 | 89 | 0.51 |
| nucleo (fzf) | 90.75% (89.6–92.4) | 0.623% | 78.2% | 80.3% | 0.290 | 191 | 0.42 |
| terms + Levenshtein | 85.01% (82.8–86.2) | 0.898% | 70.4% | 72.9% | 0.262 | 64 | 0.16 |
| contains | 81.04% (78.0–83.6) | 1.136% | 59.2% | 62.3% | 0.263 | 67 | 0.08 |

#### sales — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 94.17% (92.7–95.4) | 0.413% | 83.2% | 87.7% | 0.409 | 221 | 0.47 |
| probable, no filter | 95.84% (95.5–96.6) | 0.352% | 83.5% | 88.2% | 0.411 | 379 | 0.47 |
| probable, all words | 89.73% (87.9–90.8) | 0.630% | 80.9% | 84.0% | 0.392 | 81 | 0.43 |
| nucleo (fzf) | 80.84% (78.3–82.5) | 1.278% | 64.9% | 66.6% | 0.230 | 169 | 0.35 |
| terms + Levenshtein | 73.40% (70.8–76.5) | 1.579% | 56.8% | 59.4% | 0.220 | 55 | 0.15 |
| contains | 63.37% (61.3–64.5) | 2.182% | 39.2% | 41.4% | 0.167 | 52 | 0.08 |

### Calibration (all seeds and levels)


#### purchases

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 4674893 | 70.0 | 1731 | 0.037% |
| [0.0001, 0.001) | 1544742 | 594.0 | 1781 | 0.115% |
| [0.001, 0.01) | 1280974 | 4621.3 | 4518 | 0.353% |
| [0.01, 0.1) | 1002510 | 40345.0 | 4348 | 0.434% |
| [0.1, 0.3) | 455248 | 82029.1 | 8328 | 1.829% |
| [0.3, 0.5) | 166096 | 65588.3 | 6971 | 4.197% |
| [0.5, 0.7) | 216524 | 133015.5 | 16657 | 7.693% |
| [0.7, 0.9) | 360482 | 286679.4 | 39966 | 11.087% |
| [0.9, 0.99) | 596226 | 573548.9 | 56058 | 9.402% |
| ≥ 0.99 | 1819370 | 1817872.1 | 617056 | 33.916% |
| no word found | 34205935 | 0.0 | 31549 | 0.092% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 4674893 | 1731 | 0.037% |
| 0.001 | 6219635 | 3512 | 0.056% |
| 0.01 | 7500609 | 8030 | 0.107% |
| 0.1 | 8503119 | 12378 | 0.146% |

#### catalogues

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 17943802 | 380.9 | 55031 | 0.307% |
| [0.0001, 0.001) | 9470431 | 3676.9 | 8340 | 0.088% |
| [0.001, 0.01) | 11155715 | 42050.3 | 10627 | 0.095% |
| [0.01, 0.1) | 7610521 | 257543.8 | 29564 | 0.388% |
| [0.1, 0.3) | 2986125 | 561390.4 | 27296 | 0.914% |
| [0.3, 0.5) | 2043202 | 810069.2 | 55689 | 2.726% |
| [0.5, 0.7) | 2242686 | 1356474.3 | 124890 | 5.569% |
| [0.7, 0.9) | 2897860 | 2308232.0 | 141483 | 4.882% |
| [0.9, 0.99) | 2251900 | 2151842.1 | 533003 | 23.669% |
| ≥ 0.99 | 17602494 | 17595944.5 | 4160539 | 23.636% |
| no word found | 99277264 | 0.0 | 181356 | 0.183% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 17943802 | 55031 | 0.307% |
| 0.001 | 27414233 | 63371 | 0.231% |
| 0.01 | 38569948 | 73998 | 0.192% |
| 0.1 | 46180469 | 103562 | 0.224% |

#### dictionary

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 2735953 | 23.8 | 2 | 0.000% |
| [0.0001, 0.001) | 651082 | 280.1 | 0 | 0.000% |
| [0.001, 0.01) | 622772 | 1940.8 | 1 | 0.000% |
| [0.01, 0.1) | 344626 | 14342.8 | 12 | 0.003% |
| [0.1, 0.3) | 79403 | 13150.1 | 13 | 0.016% |
| [0.3, 0.5) | 27348 | 10335.0 | 10 | 0.037% |
| [0.5, 0.7) | 11299 | 6705.7 | 21 | 0.186% |
| [0.7, 0.9) | 12145 | 9573.6 | 37 | 0.305% |
| [0.9, 0.99) | 10054 | 9582.2 | 135 | 1.343% |
| ≥ 0.99 | 6524 | 6508.2 | 1521 | 23.314% |
| no word found | 420255594 | 0.0 | 63 | 0.000% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 2735953 | 2 | 0.000% |
| 0.001 | 3387035 | 2 | 0.000% |
| 0.01 | 4009807 | 3 | 0.000% |
| 0.1 | 4354433 | 15 | 0.000% |

#### sales

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 686539 | 14.4 | 1261 | 0.184% |
| [0.0001, 0.001) | 359985 | 140.4 | 1474 | 0.409% |
| [0.001, 0.01) | 397373 | 1507.7 | 3162 | 0.796% |
| [0.01, 0.1) | 302087 | 11100.5 | 4161 | 1.377% |
| [0.1, 0.3) | 131926 | 24488.7 | 3525 | 2.672% |
| [0.3, 0.5) | 75255 | 30050.2 | 2909 | 3.866% |
| [0.5, 0.7) | 86000 | 52027.1 | 7015 | 8.157% |
| [0.7, 0.9) | 195889 | 160961.2 | 23231 | 11.859% |
| [0.9, 0.99) | 198021 | 189374.5 | 65352 | 33.003% |
| ≥ 0.99 | 1110850 | 1110192.2 | 465301 | 41.887% |
| no word found | 6959075 | 0.0 | 15745 | 0.226% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 686539 | 1261 | 0.184% |
| 0.001 | 1046524 | 2735 | 0.261% |
| 0.01 | 1443897 | 5897 | 0.408% |
| 0.1 | 1745984 | 10058 | 0.576% |

## Speed (all processors above 256 KB; nucleo on one)

## sales ×1: 1167 entries, 0.1 MB
index: 573.125µs
probable: 281.338µs per query; nucleo: 322.211µs per query

## purchases ×1: 5147 entries, 0.4 MB
index: 2.637833ms
probable: 833.446µs per query; nucleo: 774.336µs per query

## catalogues ×1: 19498 entries, 1.6 MB
index: 10.206083ms
probable: 1.965664ms per query; nucleo: 4.611081ms per query

## dictionary ×1: 235976 entries, 2.3 MB
index: 12.426209ms
probable: 1.973241ms per query; nucleo: 2.124125ms per query

## catalogues ×10: 194980 entries, 16.0 MB
index: 102.003292ms
probable: 15.939286ms per query; nucleo: 45.244604ms per query

## catalogues ×50: 974900 entries, 81.2 MB
index: 496.782125ms
probable: 92.262409ms per query; nucleo: 226.621157ms per query

## Speed on one processor

## purchases ×1: 5147 entries, 0.4 MB
index: 5.661417ms
probable: 1.038602ms per query; nucleo: 717.233µs per query

## catalogues ×1: 19498 entries, 1.6 MB
index: 10.127292ms
probable: 4.843689ms per query; nucleo: 4.307688ms per query

## dictionary ×1: 235976 entries, 2.3 MB
index: 12.352ms
probable: 3.642693ms per query; nucleo: 2.0561ms per query

## catalogues ×10: 194980 entries, 16.0 MB
index: 98.5275ms
probable: 51.629042ms per query; nucleo: 44.566871ms per query

