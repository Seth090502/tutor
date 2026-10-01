# Runs vector-bench steps one after another, one log per step, from a private copy of the binary
# (so the source can be rebuilt while a pipeline runs). Every step is resumable: re-running a
# finished step is a no-op. Usage:
#   pwsh run.ps1 "gen" "gt" "us-build --kind i8 --m 16" "us-bench --kind i8 --m 16 --threads 12 --recall"
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Steps)
$ErrorActionPreference = 'Continue'
$exe = 'C:\Users\sethy\Projects\tutor\vault\target\release\vector-bench.exe'
$synth = 'C:\Users\sethy\Projects\vault-synth'
$logs = Join-Path $synth 'logs'
$bin = Join-Path $synth 'bin'
New-Item -ItemType Directory -Force $logs, $bin | Out-Null
$copy = Join-Path $bin ("vector-bench-{0}.exe" -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
Copy-Item $exe $copy
foreach ($s in $Steps) {
    $name = ($s -replace '[^A-Za-z0-9_.-]+', '_').Trim('_')
    $t0 = Get-Date
    "$(Get-Date -Format o) START $s" | Add-Content (Join-Path $logs 'pipeline.log')
    & $copy @($s -split ' ') *> (Join-Path $logs "$name.log")
    $rc = $LASTEXITCODE
    "$(Get-Date -Format o) END rc=$rc secs=$([int]((Get-Date) - $t0).TotalSeconds) $s" | Add-Content (Join-Path $logs 'pipeline.log')
}
