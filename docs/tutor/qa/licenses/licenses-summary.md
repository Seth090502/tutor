# License summary (2026-09-30)

Sources: `licenses-rust.csv` (cargo metadata, all targets, includes dev/build deps), `licenses-npm.csv` (apps/desktop/node_modules walk), `licenses-python.csv` (importlib.metadata over the .venv, all installed distributions).
Flag rules: license string contains GPL/AGPL/LGPL/GNU/MPL/Mozilla/EPL/SSPL/CC-BY-SA/ShareAlike/CDDL/EUPL/OSL/CPL, or is missing/unknown/custom. `OR` expressions are flagged but annotated when a permissive alternative exists. License strings are as declared in metadata; nothing was verified against license texts.

## rust: 856 packages, 39 distinct license strings (after alias merge)

| license | count |
|---|---|
| MIT OR Apache-2.0 | 453 |
| MIT | 206 |
| Apache-2.0 | 81 |
| Unicode-3.0 | 22 |
| Zlib OR Apache-2.0 OR MIT | 18 |
| Unlicense OR MIT | 9 |
| Unlicense/MIT | 7 |
| BSD-3-Clause | 6 |
| MPL-2.0 | 5 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 5 |
| ISC | 4 |
| Zlib | 3 |
| Apache-2.0 OR ISC OR MIT | 3 |
| BSD-2-Clause | 2 |
| Apache-2.0 AND MIT | 2 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 2 |
| MIT OR Apache-2.0 OR Zlib | 2 |
| MIT OR Zlib OR Apache-2.0 | 2 |
| BSD-3-Clause OR MIT OR Apache-2.0 | 2 |
| MIT OR Apache-2.0 OR LGPL-2.1-or-later | 2 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 2 |
| 0BSD OR MIT OR Apache-2.0 | 1 |
| CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | 1 |
| BSD-3-Clause AND MIT | 1 |
| BSD-3-Clause/MIT | 1 |
| (missing) | 1 |
| (Apache-2.0 OR MIT) AND BSD-3-Clause | 1 |
| zlib-acknowledgement OR MIT | 1 |
| Apache-2.0 / MIT / MPL-2.0 | 1 |
| MIT AND BSD-3-Clause | 1 |
| (MIT OR Apache-2.0) AND Apache-2.0 | 1 |
| Apache-2.0 AND ISC | 1 |
| MIT/BSD-3-Clause | 1 |
| Apache-2.0 OR BSL-1.0 | 1 |
| Apache-2.0 WITH LLVM-exception | 1 |
| CC0-1.0 | 1 |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 |
| CDLA-Permissive-2.0 | 1 |
| BSL-1.0 | 1 |

## npm: 564 packages, 14 distinct license strings (after alias merge)

| license | count |
|---|---|
| MIT | 490 |
| ISC | 23 |
| Apache-2.0 | 18 |
| BSD-3-Clause | 9 |
| BSD-2-Clause | 6 |
| MIT OR Apache-2.0 | 4 |
| MPL-2.0 | 4 |
| OFL-1.1 | 3 |
| BlueOak-1.0.0 | 2 |
| (Apache-2.0 AND BSD-3-Clause) | 1 |
| Python-2.0 | 1 |
| CC-BY-4.0 | 1 |
| 0BSD | 1 |
| (missing) | 1 |

## python: 212 packages, 28 distinct license strings (after alias merge)

| license | count |
|---|---|
| MIT | 111 |
| Apache-2.0 | 35 |
| BSD-3-Clause | 20 |
| BSD (unspecified) | 9 |
| BSD-2-Clause | 6 |
| PSF-2.0 | 5 |
| ISC | 4 |
| MPL-2.0 | 2 |
| Apache-2.0 AND MIT | 1 |
| MIT-0 | 1 |
| Apache-2.0 OR BSD-3-Clause | 1 |
| MIT AND BSD-3-Clause | 1 |
| MIT AND PSF-2.0 | 1 |
| BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0 | 1 |
| BSD-2-Clause or Apache-2.0 | 1 |
| MIT-CMU | 1 |
| BSD, Public Domain | 1 |
| Dual Licensed - GNU AFFERO GPL 3.0 or Artifex Commercial License | 1 |
| BSD-3-Clause, Apache-2.0, dependency licenses | 1 |
| GNU Lesser General Public License v3 (LGPLv3) | 1 |
| Dual License | 1 |
| LGPL-3.0-only | 1 |
| Tencent | 1 |
| Apache-2.0 AND CNRI-Python | 1 |
| BSD license (see license.txt for details), Copyright (c) 2000-2025, ReportLab Inc. | 1 |
| MIT OR Apache-2.0 | 1 |
| SEE-LICENSE-TEXT: MIT License | 1 |
| MPL-2.0 AND MIT | 1 |

## Copyleft / unknown / missing (flagged)

| ecosystem | name | version | license as declared | class | direct_or_transitive | enters via |
|---|---|---|---|---|---|---|
| rust | cssparser | 0.37.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > tauri > tauri-utils > dom_query > cssparser |
| rust | cssparser-macros | 0.7.1 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > tauri > tauri-utils > dom_query > cssparser > cssparser-macros |
| rust | desktop | 0.1.0 | (none) | unknown/missing | workspace-member | desktop |
| rust | dtoa-short | 0.3.5 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > tauri > tauri-utils > dom_query > cssparser > dtoa-short |
| rust | htmlescape | 0.3.1 | Apache-2.0 / MIT / MPL-2.0 | copyleft option, permissive alternative available (OR) | transitive | rag-engine > tantivy > htmlescape |
| rust | option-ext | 0.2.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > tauri > dirs > dirs-sys > option-ext |
| rust | r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | copyleft option, permissive alternative available (OR) | transitive | desktop > tauri > getrandom > r-efi |
| rust | r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | copyleft option, permissive alternative available (OR) | transitive | rag-engine > lancedb > tempfile > getrandom > r-efi |
| rust | selectors | 0.38.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > tauri > tauri-utils > dom_query > selectors |
| npm | lightningcss | 1.32.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > vite > lightningcss |
| npm | lightningcss | 1.33.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > vite > lightningcss |
| npm | lightningcss-win32-x64-msvc | 1.32.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > vite > lightningcss > lightningcss-win32-x64-msvc |
| npm | lightningcss-win32-x64-msvc | 1.33.0 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive | desktop > vite > lightningcss > lightningcss-win32-x64-msvc |
| npm | webgl-constants | 1.1.1 | (none) | unknown/missing | transitive | desktop > @react-three/drei > detect-gpu > webgl-constants |
| python | bidict | 0.24.1 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive_or_dev | deeptutor > python-socketio > bidict |
| python | certifi | 2026.7.22 | MPL-2.0 | copyleft (no permissive alternative stated) | transitive_or_dev | deeptutor > httpx > certifi |
| python | pymupdf | 1.28.2 | Dual Licensed - GNU AFFERO GPL 3.0 or Artifex Commercial License | copyleft or paid commercial license (no permissive alternative) | direct (core) | deeptutor > pymupdf |
| python | pyte | 0.8.2 | GNU Lesser General Public License v3 (LGPLv3) | copyleft (no permissive alternative stated) | direct (core) | deeptutor > pyte |
| python | python-dateutil | 2.9.0.post0 | Dual License | unknown/missing | transitive_or_dev | deeptutor > croniter > python-dateutil |
| python | python-telegram-bot | 22.8 | LGPL-3.0-only | copyleft (no permissive alternative stated) | direct (extra:partners) | deeptutor > python-telegram-bot |
| python | qq-botpy | 1.2.1 | Tencent | unknown/missing | direct (extra:partners) | deeptutor > qq-botpy |
| python | tiktoken | 0.14.0 | SEE-LICENSE-TEXT: MIT License | unknown/missing | direct (core) | deeptutor > tiktoken |
| python | tqdm | 4.70.1 | MPL-2.0 AND MIT | copyleft (no permissive alternative stated) | transitive_or_dev | deeptutor > openai > tqdm |

## Not flagged but worth a look

Declared licenses outside the usual permissive set (not copyleft by the flag rules): SIL OFL-1.1 (fonts, reserved-name and no-sale-by-itself terms), CC-BY-4.0 (attribution), CDLA-Permissive-2.0 (data license), zlib-acknowledgement.

| ecosystem | name | version | license |
|---|---|---|---|
| rust | fastdivide | 0.4.2 | zlib-acknowledgement OR MIT |
| rust | webpki-root-certs | 1.0.9 | CDLA-Permissive-2.0 |
| npm | @fontsource-variable/geist | 5.3.0 | OFL-1.1 |
| npm | @fontsource-variable/geist-mono | 5.3.0 | OFL-1.1 |
| npm | @fontsource-variable/newsreader | 5.3.0 | OFL-1.1 |
| npm | argparse | 2.0.1 | Python-2.0 |
| npm | caniuse-lite | 1.0.30001814 | CC-BY-4.0 |
| python | qq-botpy | 1.2.1 | Tencent |

Notes: python-dateutil declares 'Dual License' (its upstream text is BSD-3-Clause / Apache-2.0); tiktoken declares its license only as text beginning 'MIT License'; rust crate `desktop` is the workspace's own crate with no license field. Python 'direct' uses pyproject.toml core + all optional extras; 'transitive_or_dev' means installed but not named in pyproject.

Flagged total: 23 (rust 9, npm 5, python 9).
