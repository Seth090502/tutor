# Windows delivery binding

Verified binding: 2026-10-01. Canonical tool source is `C:\Users\sethy\Projects\vault\tools\vault-stage`.
It is an independent Rust package with its own workspace and lockfile. Do not add it to the root workspace or
use the root build target while another agent is building. Installed executables are under its gitignored `bin\`.

## Prerequisites and commands

Use the installed `vault-stage.exe` with separately quoted arguments. Its help/version and all outcomes are
JSON; a failure has `ok:false` and a nonzero exit code. Do not reinterpret a failure as a prepared batch.
Gitleaks is a trusted executable outside source and inbox. This PC's verified version-specific executable is
`C:\Users\sethy\scoop\apps\gitleaks\8.30.1\gitleaks.exe`, version 8.30.1. Use the actual executable, not a
forwarding shim or mutable `current` junction: the engine hashes the file it executes. Resolve the config relative to the loaded
skill directory: `assets\gitleaks.toml`. If a prerequisite is unavailable, stop the affected delivery and report it.

Example in PowerShell; `$skillRoot` is the directory containing the loaded skill, not a source repository:

```powershell
$stageTool = 'C:\Users\sethy\Projects\vault\tools\vault-stage\bin\vault-stage.exe'
$scanner = 'C:\Users\sethy\scoop\apps\gitleaks\8.30.1\gitleaks.exe'
$scanConfig = Join-Path $skillRoot 'assets\gitleaks.toml'
$source = 'E:\Ready to Transfer\notable'
$inbox = 'E:\Stand-In Vault\inbox'
$work = 'E:\vault-stage-work\2026-10-01-notable-7d42'
& $stageTool --version
& $stageTool --help
& $stageTool plan --source $source
& $stageTool stage --source $source --work $work --inbox $inbox --scanner $scanner --scanner-config $scanConfig
& $stageTool verify --work $work
& $stageTool publish --work $work --inbox $inbox --name '2026-10-01-notable-7d42'
```

Inspect every result before the next dependent command. The existing work parent and inbox must be ordinary
directories. Validate work against the entire selected vault root and any separately configured hot/cold roots;
the engine can enforce only its supplied source/work/inbox boundaries. Creating a session-owned work parent
outside all those protected roots is allowed under a delivery request;
the tool creates the actual work directory exclusively. Use a fresh unique suffix on retries. A preview only
runs `plan`; a prepare-only request runs through `verify` without `publish`.

## Exact contract

For every eligible regular source file, the payload has the same source-relative path, length and SHA-256.
Hidden and zero-byte files, opaque binaries, archives and original `SOURCE.toml`, manifests and readiness
files stay unchanged. The wrapper `payload\` prevents delivered controls from becoming batch controls.
The generated root `manifest.jsonl` has `file`, `bytes`, `sha256` rows naming `payload/<relative-path>`.
The generated `_BATCH_COMPLETE` is empty and written last, matching the current intake design.

The receipt lives beside `batch\` in the external work directory. It records complete source inventory,
eligible hashes, exclusions, bounded Git observations, scanner/config digests and control hashes. It may name
machine paths because it stays outside the vault. The generated batch manifest uses relative paths only;
source content and supplied metadata are preserved as received. Do not claim intake consumes the external receipt.
Git HEAD is a bounded static observation, not proof of a clean checkout. Unsupported Git indirection is
explicitly unavailable; origin strings are withheld and identified by their digest. Preserve supplied `SOURCE.toml`
and record absent provenance as unknown rather than fabricating a URL or commit.

Publication requires the same volume and an unused destination. On Windows the tool uses a directory move
without replacement. It has no cross-volume copy fallback. A durable external publication intent lets a read-only
`verify` locate a published batch; a second publish is an explicit error. Keep the external work receipt for audit.

## Holds and errors

- Known credential filenames are held before scanning. Gitleaks findings are held by path and reason, never by
  value. A source-supplied ignore file or allow comment cannot suppress the trusted scan: the scanner receives
  opaque filenames in a separate byte snapshot, an explicit trusted config, an empty ignore and disabled allow comments.
  The trusted config is snapshotted and hash-checked in external work; Windows read locks prevent writes/deletes
  to it and the actual scanner executable while scanning. Configs must be standalone UTF-8 TOML (at most 1 MiB)
  with the shipped `[extend] useDefault = true` form; external imports and unsupported extension syntax fail.
- The scanner snapshot is temporary local quarantine outside the inbox. Its exact bytes are checked and its
  owned raw inputs removed on normal success and handled failures. A forced process termination can leave it
  behind; report the owned path and arrange bounded cleanup, never publish that work. No routine deletes originals.
- Named VCS metadata and dependency install trees are accounted for; a mere folder called `target`, `build`,
  `bin`, `docs`, `venv` or `site-packages` cannot discard legitimate source. Python dependency detection uses
  environment/distribution markers. All reparse entries are held without following them, including cycles.
- An unreadable file, scanner error, changed source, overlapping roots, invalid path, case collision,
  insufficient space or mismatched payload prevents completion. Retain the error and use fresh work after fixing
  the cause. Never repair another writer's directory or weaken verification to proceed.

This proves preservation of eligible ordinary file contents and directory layout. It does not promise NTFS ACLs,
timestamps, alternate data streams, semantic extraction, rights clearance, or exhaustive secret detection inside
every binary/encoded/archive format. Future extraction must apply its own bounded checks before storing/serving text.
