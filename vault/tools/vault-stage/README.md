# vault-stage

Deterministic Windows delivery of owner-staged material for local static study. The model selects scope and
reports evidence; Rust inventories, scans, copies, independently verifies and atomically publishes eligible
original bytes. This tool never runs source code, starts intake, or claims a copied batch became a vault document.

This package has its own `[workspace]`, `Cargo.lock` and build target. It does not modify or build the vault's
root workspace, core, server, extractor or benchmark packages. Keep builds outside OneDrive and the live repository.
Windows is the supported staging platform; other operating systems are not established by these checks.

## Build and install executables

The source is `C:\Users\sethy\Projects\tutor\vault\tools\vault-stage` on this PC. Choose a session-owned scratch
target; use the standalone manifest explicitly. Dependencies are pinned in this package's own lockfile.

```powershell
$packageCargo = 'C:\Users\sethy\Projects\tutor\vault\tools\vault-stage\Cargo.toml'
$scratchBuild = Join-Path $env:TEMP 'vault-stage-build'
cargo build --offline --locked --release --manifest-path $packageCargo --target-dir $scratchBuild
cargo test --offline --locked --manifest-path $packageCargo --target-dir $scratchBuild
cargo clippy --offline --locked --all-targets --manifest-path $packageCargo --target-dir $scratchBuild -- -D warnings
```

Copy only the three resulting release executables into this package's gitignored `bin\` when installing:
`vault-stage.exe`, `vault-stage-eval.exe`, `vault-stage-agent-eval.exe`. Do not copy the build target into the repository.
An offline build requires the pinned crates already cached; missing dependencies are an explicit setup failure.

## Delivery

Read `../../skills/ingest/SKILL.md` and the selected vault's instructions before a real delivery. Supply the
resolved source/inbox paths and the skill's trusted scanner config. Commands accept explicit option/value pairs:

```text
vault-stage --version
vault-stage --help
vault-stage plan --source <existing-directory>
vault-stage stage --source <existing-directory> --work <unused-directory> --inbox <existing-directory> --scanner <trusted-gitleaks.exe> --scanner-config <trusted.toml>
vault-stage verify --work <directory>
vault-stage publish --work <directory> --inbox <same-inbox> --name <YYYY-MM-DD-slug>
```

Source, work and inbox must be disjoint, non-reparse roots. The caller also validates work outside the entire
selected vault and any separately configured hot/cold roots; these are not implicit arguments to the engine.
The work's parent must already exist on the inbox
volume; the tool exclusively allocates work and checks available allocation space before scanning. A preview
runs only `plan`. A prepare-only request ends after `stage` and `verify`. Delivery publishes a complete copy
under a new date-slug of at most 40 characters. An existing destination, even empty, is an error.

Every eligible regular file is preserved under `batch/payload/<source-relative-path>`, including opaque binaries,
archives, Unicode/hidden/empty files, alternate formats, supplied metadata and nested controls. The generated
root `manifest.jsonl` contains `file`, `bytes`, `sha256`. An empty `_BATCH_COMPLETE` is written last. Independent
verification re-enumerates source/payload, compares exact path/directory sets, size and SHA-256, and checks controls.
Counts alone cannot establish fidelity. Supplied `SOURCE.toml` is never overwritten or silently generated.

`receipt.json` stays in the external work directory and binds the source inventory, eligible hashes, scanner and
config hashes, exclusions, controls and bounded Git observations. Origin strings are withheld and represented by
their digest. A HEAD observation is not a clean-worktree assertion; unsupported indirection is explicitly unknown.
Do not claim the future intake pipeline consumes this external receipt.

`plan` returns `inventoried`, with `scanner_completed:false`. Successful stages/verification/publication return
`prepared`/`verified`/`published`, `pending_intake:true`, `ingested:false`, measured file/byte counts and at most
20 path-only exclusion samples. `excluded` counts disposition entries, including held trees, not every descendant
file in a held tree. Full dispositions are in the receipt. Failures are JSON with `ok:false` and a nonzero exit code.

Known credential filenames, trusted scanner findings, named VCS/dependency install trees and all reparse entries
are held with reasons. No licence/topic/provenance/group ban exists. Ordinary source in folders named `target`,
`build`, `bin`, `docs`, `venv` or plain `site-packages` is retained; Python install detection uses markers.
Gitleaks scans an exact-byte opaque-name temporary snapshot with a hash-verified trusted policy snapshot, an empty ignore and
disabled allow comments; source-provided suppression cannot govern it. Owned raw scanner inputs are removed on
normal success and handled errors. A forced termination can leave a private incomplete work directory; never
publish it, and clean it only after checking ownership and the exact confined target.
Windows read locks pin the original/snapshot configuration and actual scanner executable during scanning.
The trusted standalone config must be UTF-8, at most 1 MiB, with supported unquoted keys and the canonical
`[extend] useDefault = true` form. External imports or unsupported extension syntax fail instead of being relocated.

Publication is a same-volume directory move without replacement; there is no merge or cross-volume fallback.
An external durable intent supports read-only verification after publication. Repeated publication fails explicitly.
Source/config changes, scanner errors, insufficient space, invalid paths, missing/extra bytes or hash mismatches
stop completion. Preserve the error and use new work after fixing the cause; do not patch receipts/payloads.

The byte contract covers ordinary file contents and directory layout, not ACLs, alternate data streams, timestamps,
universal extraction or exhaustive secret detection in every encoded/binary/container format. Semantic intake
and text indexing are separate downstream outcomes. Do not expose discovered credentials or execute archived code.

## Mechanical evaluation

Use the actual engine and independent grader; run train while developing, then the reserved test split once
after freezing. The seed changes harmless fixture bytes/order, never membership. Every repeat uses fresh scratch.
Explicit scanner options avoid relying on the build machine's paths:

```powershell
$mechanicalEval = 'C:\Users\sethy\Projects\tutor\vault\tools\vault-stage\bin\vault-stage-eval.exe'
$trustedGitleaks = 'C:\Users\sethy\scoop\apps\gitleaks\8.30.1\gitleaks.exe'
$trustedScannerConfig = 'C:\Users\sethy\Projects\tutor\vault\skills\ingest\assets\gitleaks.toml'
& $mechanicalEval --output $newTrainJson --split train --seed 41 --repeat 2 --scanner $trustedGitleaks --scanner-config $trustedScannerConfig
& $mechanicalEval --output $newHeldoutJson --split test --seed 41 --repeat 1 --scanner $trustedGitleaks --scanner-config $trustedScannerConfig
```

Output files must be unused, outside the real staging/vault. `eval/protocol.json` freezes 8 train and 4 test families.
Tests compare source and payload bytes directly as well as hashes, complete path sets, metadata/empty directories,
scanner residue and manifests. Fault cases cover corruption of equal length, missing/extra files, changing source,
scanner/suppression failures, reparse escapes/cycles, overlap, exclusive work and competing/existing destinations.
The release bar is 100% eligible byte/path/source preservation, safe failure and honest pending-intake outcomes.

## Qwen and hillclimbing

See [eval/local-model.md](eval/local-model.md) for `vault-stage-agent-eval`, explicit loopback endpoint/model
arguments, frozen train/test cases, paired baseline/candidate comparisons, receipts and usage accounting.
It offers a constrained `{}` tool broker, so the candidate cannot edit fixtures, access the grader or execute a shell.
The supplied skill drives workflow; the grader independently measures filesystem and reporting outcomes.
Keep model parameters fixed, amend one prompt instruction at a time from train failures, and revert regressions.
Only compare speed/token use after integrity and workflow parity. A mock proves the HTTP/broker path, not Qwen ability.
Never start or change the owner's model/GPU runtime as an incidental part of a delivery.
