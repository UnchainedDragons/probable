# Bench results

Produced with `cargo run --release -p probable-bench -- evaluate <list> <queries> <seed>` (seeds 11, 12, 13; 11 and 12 for the dictionary) and summarised per list and typo level: mean over the seeds, worst and best seed in brackets. The lists are not published. The `ms/query` column of these runs is noisy (other work was running on the machine); the speed tables at the end are the reference.


#### purchases — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.42% (92.8–97.1) | 0.089% | 89.8% | 93.4% | 0.487 | 572 | 4.23 |
| probable, no filter | 95.49% (92.8–97.2) | 0.106% | 89.8% | 93.4% | 0.487 | 1347 | 4.23 |
| probable, all words | 94.58% (92.1–96.5) | 0.100% | 89.4% | 92.6% | 0.482 | 332 | 4.37 |
| nucleo (fzf) | 94.60% (93.1–96.2) | 0.105% | 85.3% | 89.4% | 0.450 | 585 | 1.75 |
| terms + Levenshtein | 87.19% (84.7–89.6) | 0.227% | 76.7% | 80.4% | 0.401 | 99 | 1.13 |
| contains | 90.91% (88.4–94.4) | 0.161% | 73.2% | 79.5% | 0.413 | 123 | 0.50 |

#### purchases — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.90% (94.2–97.5) | 0.073% | 86.1% | 91.2% | 0.477 | 559 | 1.89 |
| probable, no filter | 96.58% (94.6–97.9) | 0.073% | 86.1% | 91.2% | 0.477 | 1343 | 1.89 |
| probable, all words | 95.18% (93.5–96.5) | 0.082% | 85.2% | 89.6% | 0.469 | 305 | 1.94 |
| nucleo (fzf) | 86.85% (83.2–89.8) | 0.237% | 71.3% | 76.4% | 0.366 | 553 | 1.07 |
| terms + Levenshtein | 82.66% (80.9–85.3) | 0.286% | 67.9% | 71.5% | 0.346 | 91 | 0.83 |
| contains | 77.42% (73.4–80.8) | 0.372% | 55.9% | 61.2% | 0.306 | 102 | 0.29 |

#### purchases — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 94.46% (91.6–96.5) | 0.106% | 78.9% | 85.9% | 0.441 | 556 | 0.87 |
| probable, no filter | 95.80% (93.7–98.0) | 0.098% | 79.0% | 86.0% | 0.442 | 1348 | 0.87 |
| probable, all words | 94.10% (92.6–95.2) | 0.108% | 77.1% | 82.8% | 0.422 | 325 | 0.82 |
| nucleo (fzf) | 79.25% (78.2–80.9) | 0.404% | 56.5% | 60.4% | 0.273 | 573 | 0.78 |
| terms + Levenshtein | 69.50% (66.2–74.9) | 0.534% | 54.6% | 57.7% | 0.264 | 83 | 0.60 |
| contains | 61.36% (56.8–67.7) | 0.679% | 37.4% | 41.4% | 0.193 | 95 | 0.25 |

#### catalogues — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.72% (94.8–96.6) | 0.168% | 86.6% | 88.1% | 0.233 | 4746 | 2.60 |
| probable, no filter | 95.99% (94.9–96.6) | 0.211% | 86.6% | 88.2% | 0.233 | 8550 | 2.60 |
| probable, all words | 95.49% (94.5–96.2) | 0.155% | 86.5% | 87.9% | 0.233 | 2659 | 2.36 |
| nucleo (fzf) | 95.45% (94.7–95.9) | 0.156% | 79.0% | 84.7% | 0.213 | 2671 | 5.00 |
| terms + Levenshtein | 85.12% (83.4–87.7) | 0.452% | 71.3% | 76.0% | 0.191 | 646 | 1.98 |
| contains | 92.51% (91.6–94.2) | 0.234% | 70.8% | 76.5% | 0.203 | 776 | 0.82 |

#### catalogues — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.88% (96.1–97.8) | 0.125% | 82.3% | 84.2% | 0.216 | 4811 | 3.11 |
| probable, no filter | 97.85% (96.8–98.8) | 0.116% | 82.3% | 84.2% | 0.216 | 8408 | 3.11 |
| probable, all words | 96.36% (95.2–97.6) | 0.127% | 81.9% | 83.8% | 0.212 | 2609 | 2.87 |
| nucleo (fzf) | 87.81% (85.2–91.3) | 0.428% | 69.3% | 73.4% | 0.173 | 2643 | 5.49 |
| terms + Levenshtein | 83.43% (80.8–86.0) | 0.509% | 64.1% | 68.2% | 0.159 | 647 | 2.12 |
| contains | 79.85% (77.9–83.5) | 0.629% | 54.1% | 58.8% | 0.139 | 707 | 0.87 |

#### catalogues — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 95.35% (94.7–95.8) | 0.192% | 74.1% | 77.6% | 0.200 | 4602 | 3.34 |
| probable, no filter | 96.02% (94.8–97.2) | 0.220% | 74.1% | 77.7% | 0.200 | 8423 | 3.34 |
| probable, all words | 94.78% (94.0–95.3) | 0.188% | 73.2% | 76.3% | 0.196 | 2497 | 2.85 |
| nucleo (fzf) | 78.16% (75.2–80.2) | 0.787% | 56.6% | 61.0% | 0.138 | 2512 | 5.77 |
| terms + Levenshtein | 74.25% (71.9–77.3) | 0.836% | 51.2% | 55.8% | 0.128 | 618 | 2.48 |
| contains | 65.28% (60.7–70.2) | 1.129% | 38.3% | 42.4% | 0.093 | 627 | 0.99 |

#### dictionary — no typos (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.85% (97.3–98.4) | 0.000% | 84.5% | 93.8% | 0.875 | 406 | 1.90 |
| probable, no filter | 97.85% (97.3–98.4) | 0.000% | 84.5% | 93.8% | 0.875 | 2142 | 1.90 |
| probable, all words | 97.85% (97.3–98.4) | 0.000% | 84.5% | 93.8% | 0.875 | 848 | 1.76 |
| nucleo (fzf) | 97.85% (97.3–98.4) | 0.000% | 84.5% | 94.2% | 0.875 | 278 | 2.18 |
| terms + Levenshtein | 91.25% (90.4–92.1) | 0.000% | 78.7% | 87.8% | 0.816 | 10 | 2.54 |
| contains | 97.85% (97.3–98.4) | 0.000% | 84.8% | 94.3% | 0.879 | 29 | 7.12 |

#### dictionary — 3% typos per letter (what the model assumes) (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 96.53% (95.7–97.3) | 0.000% | 81.8% | 94.0% | 0.857 | 450 | 1.91 |
| probable, no filter | 96.53% (95.7–97.3) | 0.000% | 81.8% | 94.0% | 0.857 | 2776 | 1.91 |
| probable, all words | 96.53% (95.7–97.3) | 0.000% | 82.0% | 94.0% | 0.859 | 784 | 1.75 |
| nucleo (fzf) | 80.46% (80.1–80.8) | 0.000% | 69.8% | 78.3% | 0.724 | 291 | 2.14 |
| terms + Levenshtein | 81.62% (80.5–82.8) | 0.000% | 70.3% | 79.8% | 0.736 | 18 | 2.48 |
| contains | 77.98% (77.5–78.5) | 0.000% | 68.7% | 76.2% | 0.710 | 87 | 7.02 |

#### dictionary — 8% typos per letter (more than the model) (seeds: 2)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 94.88% (94.0–95.7) | 0.000% | 74.3% | 87.7% | 0.790 | 406 | 2.12 |
| probable, no filter | 95.21% (94.4–96.0) | 0.000% | 74.3% | 87.7% | 0.790 | 2584 | 2.12 |
| probable, all words | 94.54% (93.7–95.4) | 0.000% | 74.2% | 87.3% | 0.788 | 858 | 1.97 |
| nucleo (fzf) | 57.86% (57.8–58.0) | 0.000% | 46.4% | 54.0% | 0.490 | 128 | 2.08 |
| terms + Levenshtein | 68.59% (66.9–70.3) | 0.000% | 54.5% | 66.2% | 0.591 | 6 | 2.46 |
| contains | 49.92% (49.0–50.8) | 0.000% | 41.0% | 47.2% | 0.431 | 16 | 7.10 |

#### sales — no typos (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.73% (97.4–98.2) | 0.162% | 91.0% | 93.8% | 0.436 | 253 | 0.26 |
| probable, no filter | 98.04% (97.7–98.4) | 0.167% | 91.0% | 93.9% | 0.436 | 400 | 0.26 |
| probable, all words | 97.07% (97.0–97.2) | 0.183% | 90.9% | 93.4% | 0.434 | 127 | 0.24 |
| nucleo (fzf) | 96.84% (96.5–97.1) | 0.211% | 87.7% | 91.6% | 0.361 | 191 | 0.30 |
| terms + Levenshtein | 87.78% (86.5–88.9) | 0.721% | 78.6% | 81.3% | 0.312 | 64 | 0.12 |
| contains | 93.60% (92.4–94.3) | 0.379% | 77.3% | 81.0% | 0.362 | 72 | 0.06 |

#### sales — 3% typos per letter (what the model assumes) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 97.29% (95.8–98.2) | 0.194% | 88.0% | 90.6% | 0.413 | 253 | 0.26 |
| probable, no filter | 98.19% (97.6–98.5) | 0.156% | 88.1% | 90.7% | 0.413 | 402 | 0.26 |
| probable, all words | 96.49% (95.5–97.0) | 0.223% | 87.3% | 89.5% | 0.405 | 131 | 0.24 |
| nucleo (fzf) | 90.75% (89.6–92.4) | 0.623% | 78.2% | 80.3% | 0.290 | 191 | 0.30 |
| terms + Levenshtein | 85.01% (82.8–86.2) | 0.898% | 70.4% | 72.9% | 0.262 | 64 | 0.13 |
| contains | 81.04% (78.0–83.6) | 1.136% | 59.2% | 62.3% | 0.263 | 67 | 0.06 |

#### sales — 8% typos per letter (more than the model) (seeds: 3)

| method | recall | P(relevant \| excluded) | 1st relevant | top 10 | target MRR | entries kept | ms/query |
|---|---|---|---|---|---|---|---|
| **probable, filter ε = 1%** | 94.70% (93.6–95.8) | 0.381% | 82.5% | 87.4% | 0.406 | 235 | 0.25 |
| probable, no filter | 95.80% (95.5–96.4) | 0.355% | 82.6% | 87.7% | 0.408 | 379 | 0.25 |
| probable, all words | 93.00% (91.8–93.9) | 0.445% | 81.4% | 85.3% | 0.396 | 119 | 0.24 |
| nucleo (fzf) | 80.84% (78.3–82.5) | 1.278% | 64.9% | 66.6% | 0.230 | 169 | 0.28 |
| terms + Levenshtein | 73.40% (70.8–76.5) | 1.579% | 56.8% | 59.4% | 0.220 | 55 | 0.13 |
| contains | 63.37% (61.3–64.5) | 2.182% | 39.2% | 41.4% | 0.167 | 52 | 0.06 |

### Calibration (all seeds and levels)


#### purchases

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 4084958 | 68.1 | 55 | 0.001% |
| [0.0001, 0.001) | 1619833 | 615.6 | 1623 | 0.100% |
| [0.001, 0.01) | 1349952 | 5080.8 | 3738 | 0.277% |
| [0.01, 0.1) | 1201938 | 49323.8 | 3961 | 0.330% |
| [0.1, 0.3) | 525507 | 93370.4 | 7812 | 1.487% |
| [0.3, 0.5) | 227738 | 89461.3 | 7830 | 3.438% |
| [0.5, 0.7) | 238452 | 146343.9 | 16702 | 7.004% |
| [0.7, 0.9) | 391305 | 312926.8 | 37006 | 9.457% |
| [0.9, 0.99) | 618104 | 594286.7 | 56230 | 9.097% |
| ≥ 0.99 | 1855238 | 1853670.8 | 622417 | 33.549% |
| no word found | 34209975 | 0.0 | 31589 | 0.092% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 4084958 | 55 | 0.001% |
| 0.001 | 5704791 | 1678 | 0.029% |
| 0.01 | 7054743 | 5416 | 0.077% |
| 0.1 | 8256681 | 9377 | 0.114% |

#### catalogues

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 13608625 | 323.2 | 25390 | 0.187% |
| [0.0001, 0.001) | 9100954 | 3647.2 | 1345 | 0.015% |
| [0.001, 0.01) | 10954175 | 43425.4 | 6714 | 0.061% |
| [0.01, 0.1) | 9292993 | 329557.5 | 30734 | 0.331% |
| [0.1, 0.3) | 3685618 | 678161.1 | 27303 | 0.741% |
| [0.3, 0.5) | 2062872 | 805631.2 | 68111 | 3.302% |
| [0.5, 0.7) | 1817724 | 1101302.4 | 75689 | 4.164% |
| [0.7, 0.9) | 4385610 | 3581504.8 | 185188 | 4.223% |
| [0.9, 0.99) | 3099653 | 2952117.2 | 514633 | 16.603% |
| ≥ 0.99 | 18134065 | 18126711.1 | 4211183 | 23.222% |
| no word found | 99339711 | 0.0 | 181528 | 0.183% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 13608625 | 25390 | 0.187% |
| 0.001 | 22709579 | 26735 | 0.118% |
| 0.01 | 33663754 | 33449 | 0.099% |
| 0.1 | 42956747 | 64183 | 0.149% |

#### dictionary

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 2511566 | 29.6 | 2 | 0.000% |
| [0.0001, 0.001) | 584343 | 252.8 | 0 | 0.000% |
| [0.001, 0.01) | 648693 | 2171.9 | 0 | 0.000% |
| [0.01, 0.1) | 421960 | 20750.9 | 7 | 0.002% |
| [0.1, 0.3) | 198641 | 30712.9 | 7 | 0.004% |
| [0.3, 0.5) | 62389 | 24338.2 | 6 | 0.010% |
| [0.5, 0.7) | 25993 | 15396.4 | 12 | 0.046% |
| [0.7, 0.9) | 23447 | 18564.5 | 36 | 0.154% |
| [0.9, 0.99) | 13717 | 13050.5 | 117 | 0.853% |
| ≥ 0.99 | 10457 | 10431.1 | 1565 | 14.966% |
| no word found | 420255594 | 0.0 | 63 | 0.000% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 2511566 | 2 | 0.000% |
| 0.001 | 3095909 | 2 | 0.000% |
| 0.01 | 3744602 | 2 | 0.000% |
| 0.1 | 4166562 | 9 | 0.000% |

#### sales

| probability given | entries | expected relevant | actual relevant | actual share |
|---|---|---|---|---|
| [0, 0.0001) | 574659 | 12.9 | 765 | 0.133% |
| [0.0001, 0.001) | 346692 | 134.4 | 1418 | 0.409% |
| [0.001, 0.01) | 401852 | 1555.0 | 2421 | 0.602% |
| [0.01, 0.1) | 344657 | 12856.7 | 3840 | 1.114% |
| [0.1, 0.3) | 148206 | 27425.8 | 3395 | 2.291% |
| [0.3, 0.5) | 85813 | 34319.7 | 3302 | 3.848% |
| [0.5, 0.7) | 92621 | 56033.5 | 6919 | 7.470% |
| [0.7, 0.9) | 209733 | 172144.8 | 23827 | 11.361% |
| [0.9, 0.99) | 209308 | 200054.2 | 65560 | 31.322% |
| ≥ 0.99 | 1128735 | 1128039.1 | 465864 | 41.273% |
| no word found | 6960724 | 0.0 | 15825 | 0.227% |

| ε | entries below ε | relevant among them | share |
|---|---|---|---|
| 0.0001 | 574659 | 765 | 0.133% |
| 0.001 | 921351 | 2183 | 0.237% |
| 0.01 | 1323203 | 4604 | 0.348% |
| 0.1 | 1667860 | 8444 | 0.506% |

## Speed (all processors above 256 KB; nucleo on one)

## sales ×1: 1167 entries, 0.1 MB
index: 419.958µs
probable: 227.496µs per query; nucleo: 295.952µs per query

## purchases ×1: 5147 entries, 0.4 MB
index: 1.791375ms
probable: 668.406µs per query; nucleo: 765.804µs per query

## catalogues ×1: 19498 entries, 1.6 MB
index: 6.858208ms
probable: 1.737817ms per query; nucleo: 4.498376ms per query

## dictionary ×1: 235976 entries, 2.3 MB
index: 11.281875ms
probable: 1.783963ms per query; nucleo: 2.086704ms per query

## catalogues ×10: 194980 entries, 16.0 MB
index: 70.112834ms
probable: 14.810306ms per query; nucleo: 45.122641ms per query

## catalogues ×50: 974900 entries, 81.2 MB
index: 334.33725ms
probable: 80.310366ms per query; nucleo: 223.56455ms per query

## Speed on one processor

## purchases ×1: 5147 entries, 0.4 MB
probable: 937.036µs per query; nucleo: 729.738µs per query
## catalogues ×1: 19498 entries, 1.6 MB
probable: 4.141185ms per query; nucleo: 4.422687ms per query
## dictionary ×1: 235976 entries, 2.3 MB
probable: 3.411706ms per query; nucleo: 2.063006ms per query
## catalogues ×10: 194980 entries, 16.0 MB
probable: 43.824366ms per query; nucleo: 44.649018ms per query
