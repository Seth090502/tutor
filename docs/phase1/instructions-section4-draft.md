# Draft: replacement for INSTRUCTIONS.md section 4 (source code folders)

Draft for the lead, 2026-10-01. It follows `docs/phase1/CODE-DESIGN.md` and the `/ingest` skill
(`skills/ingest/SKILL.md`). Everything below the line replaces section 4 of `docs/vault/INSTRUCTIONS.md` as-is.
Two neighbouring edits go with it, listed at the end.

---

## 4. Source code folders

Deliver code with `/ingest` (section 3.0), or by hand: copy each repository as **one whole folder** into a
batch folder, then add `_BATCH_COMPLETE`. A folder of repositories is fine; each repository inside it is handled
on its own.

**(Phase 1)** Intake recognises a repository by a `SOURCE.toml` at its root (the `/ingest` skill writes one), or
by its contents: a `.git` folder or build files such as `Cargo.toml`, `CMakeLists.txt`, `package.json`, `go.mod`,
`*.sln`, `SConstruct` or `*.uproject`.

### What happens to a repository

- **Every text file becomes its own vault document, stored verbatim.**
  - The encoding becomes UTF-8, line endings become LF, and a byte-order mark is removed. Nothing else changes:
    tabs, trailing spaces and a missing final newline all stay.
  - So `…#L40-L72` is exactly lines 40-72 as any editor shows them.
  - This includes Markdown, text and other documentation files inside the repository, so their line numbers
    match the repository too.
  - PDFs, EPUBs and notebooks inside a repository go through their normal routes (section 3.5). A notebook's
    line numbers refer to its converted text, not to the `.ipynb` file.
- **Each file is classified by its content and the repository's own markers, never by folder name alone.**
  A `build\`, `bin\` or `docs\` folder that holds real source is kept.

  | Class | Examples | How agents see it |
  |---|---|---|
  | source | `.cpp`, `.rs`, `.cs`, `.gd`, `.hlsl` | Searched and ranked first |
  | build | `CMakeLists.txt`, `Cargo.toml`, `*.Build.cs`, `SConstruct` | Searched; used for the module map |
  | docs | `README.md`, design notes, Godot's `doc/classes/*.xml` | Searched |
  | test | test folders and files (`*_test.go`, `tests\`) | Searched, ranked below source |
  | generated | files with an `@generated` or "DO NOT EDIT" header, `*.gen.*`, `linguist-generated` | Searched, ranked below source |
  | vendored | third-party copies (`thirdparty\`, `linguist-vendored`) | Searched, ranked below source |
  | data | translations (`.po`), large tables, test baselines | Found by exact search (`grep`) and `read` only |

  - The signals are the repository's `.gitignore` and `.gitattributes` (`linguist-vendored`,
    `linguist-generated`, `linguist-language`), Godot's `.gdignore`, generated-file headers, Linguist's list of
    vendored paths, build-artefact signatures, and line length (minified files).
- **Identical files are stored once.** When several repositories carry the same file (a vendored zlib, a
  forked engine), it is one document listed under every repository, with the same address everywhere.
- **A repository document is generated for each delivery:** a metadata block (name, URL, commit, licence,
  languages, collections), the README, the file tree with each file's address and class, binary files listed
  by path, size and type, and a ranked map of the main symbols. `outline` on it shows the tree and the map.
- **Recorded per repository:** name, URL and commit (from `SOURCE.toml`, or read from `.git` if it was
  delivered), the licence, and the language of each file. Each file also gets a Software Heritage identifier
  (SWHID) of the delivered bytes.
- **Symbols:** functions, classes, methods and other definitions are extracted with line ranges for the main
  languages. They power `outline`, `define`, `citations` and symbol-aware ranking. Languages without symbol
  support are still fully searchable with `grep` and `search`; `vault_info` lists the coverage per language.
- **Macro-heavy C++** (Unreal, Source engine, Godot) is parsed with a profile that hides export and annotation
  macros (`ENGINE_API`, `UCLASS(...)`, `GDCLASS(...)`) from the parser only. The stored text is never changed.
  Files the parser cannot fully understand are flagged, and stay searchable.

### `SOURCE.toml`

Optional, at the repository root. `/ingest` writes it; a template is in `inbox\_templates\SOURCE.toml`.
Every field is optional.

| Field | Meaning |
|---|---|
| `name` | Short repository name, used in results and in `repo` filters |
| `url` | Where it came from |
| `commit` | The commit delivered. `/ingest` reads it from `.git` because it does not copy `.git` |
| `license` | SPDX id (`MIT`, `GPL-2.0-only`…), `owned` for your own code, or `unknown`. Licence files are detected anyway |
| `categories` | First one is primary; usually `cs-ai` |
| `collections` | Named sets this repository joins, for example `gamedev`, `compilers`, `rust-exemplars` |
| `level` | `intro`, `graduate`, `landmark` or `recent` |
| `description` | One line on what the code is and why it is in the vault |
| `companion` | DOI or title of a book or paper this code accompanies |
| `profile` | Only if detection gets it wrong: `unreal`, `source-engine`, `godot` or `generic` |

**Licences:** a repository with no licence (decompilations, archive copies, leaked source) is still ingested
and searchable. It is recorded as `unknown` and marked not redistributable, and nothing the vault exports will
include it.

### Collections

- **A collection is a named, persistent set of repositories and documents,** for example `gamedev` = the engines
  and games plus a few books. Agents can search, grep or map a whole collection in one call.
- `SOURCE.toml` `collections` adds a repository at intake. **(Phase 1)** `vault collection create|add|remove|list`
  edits collections later.
- Collections are labels: changing them never changes an address.

### A later commit of the same repository

- Deliver the whole folder again, with the new `commit` in `SOURCE.toml`. It becomes a **new snapshot.**
- Unchanged files keep their documents and addresses. Changed and new files become new documents.
- The old snapshot stays readable, and its addresses keep working. Searches use the newest snapshot of each
  repository unless a request names an older one.
- **History is not ingested,** only the delivered files. Most delivered clones are shallow anyway.

### Excluded by rule

These are never silently dropped. The batch report lists each one with its path, size, BLAKE3 hash and the rule
that matched, and the files stay in `inbox\_done\<batch>\` for you to review. Large exclusions are summarised per
folder.

| Rule | What |
|---|---|
| `vcs-metadata` | `.git\`, `.hg\`, `.svn\` |
| `dependencies` | Package-manager install trees: `node_modules\`, Python virtual environments (a folder with `pyvenv.cfg`), `site-packages\`, `bower_components\` |
| `build-output` | Folders that contain build artefacts (object files, `.pdb`, `CMakeCache.txt`, `.tlog`, `.ninja_log`). Decided by contents, not by the folder's name |
| `gitignored` | Files the repository's own `.gitignore` excludes (usually local build output) |
| `lockfiles` | `Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `poetry.lock` |
| `minified` | Text with very long lines: minified JavaScript or CSS, source maps, packed data |
| `binary` | Non-text files that are not a supported document format: images, models, sounds, compiled files, game data. The repository document still lists them by path, size and type |
| `oversize-text` | Text over 2 MB that is not hand-written source (generated tables, dumps), and any text file over 16 MB. Large hand-written source such as SQLite's single-file `sqlite3.c` is kept |

### Secrets: please remove them before copying

- Intake also scans every file for private keys, tokens, `.env` files and credential files.
- A match is **left in place** in `inbox\_done\<batch>\`: never copied into the vault, never indexed, never
  served. It is recorded as quarantined with reason `suspected_secret`, naming the rule, never the value.
- Open-source projects often carry test keys (TLS test certificates). Those are caught too, and are listed by
  folder so the report stays readable.

### What not to deliver

- **Game builds and asset dumps without source.** They would only be excluded as `binary`; `/ingest` lists them
  in its report instead of copying them.
- **Archives over the unpack limits** (section 3.5): unpack them yourself first, or leave them out.
- **Your own notes about where material came from,** such as an index file in the staging folder. They can hold
  passwords or account details, and everything in the vault is readable by the tutor and other agents.

---

## Neighbouring edits that go with this section

1. **Section 8.2, tools table:** `outline(doc_ids[])` row becomes "Heading tree with line ranges and pages; for
   a repository, the file tree and a ranked symbol map fitted to the budget; for a code file, its symbols";
   `define(terms[])` row adds "and, for code, symbol definitions across repositories"; `citations` row adds
   "for code: imports, includes, calls and references, both directions". Add `collection` and `repo` to the
   filters list, and "a repository or a collection" to the `grep` scope description.
2. **Section 3.5, formats table:** the row "Source code inside a code folder | Stored verbatim (section 4)"
   becomes "Any text file inside a code folder, including Markdown | Stored verbatim (section 4)".
3. **`docs/vault/templates/SOURCE.toml`:** add the optional line
   `profile = ""   # only if detection gets it wrong: unreal | source-engine | godot | generic`, and change the
   `commit` comment to "the delivered commit; `/ingest` fills it from `.git`, which it does not copy".
4. **`skills/ingest/SKILL.md` step 1 (survey):** notes and index files at the staging root (for example
   `INDEX.md`) are not material to deliver; list them in the report. The staged `INDEX.md` today holds a
   plaintext login, which shows why.
