<#
.SYNOPSIS
  Stage 1 model comparison on the RTX 5090: one command, whole matrix, resumable.

.DESCRIPTION
  Runs sequentially through llama-server (CUDA build; Vulkan only for the parity checks):
    Qwen3-Embedding 0.6B Q8_0 / 0.6B f16 / 4B Q8_0 / 8B Q8_0, BGE-M3 Q8_0, Harrier-OSS-v1 0.6B Q8_0 (embedders: nDCG@10 and recall@100 on
    BRIGHT psychology+biology+economics at full, 512 and 256 dims; bulk throughput; peak VRAM),
    Qwen3-Reranker 0.6B / 4B / 8B Q8_0 over the fixed 0.6B first-stage top-100 (sanity test first),
    and CUDA-vs-Vulkan embedding parity (500 chunks, cosine >= 0.999) for Qwen3-Embedding-0.6B Q8_0 and f16.

  Resumable: a configuration whose result line (status ok / sanity_failed / parity_failed) is already in
  results.jsonl is skipped. Stops on the first server/infrastructure failure with a clear message. Never leaves
  a llama-server or nvidia-smi running (pid files are checked after every run and in `finally`).
  Exit codes: 0 all done, 1 pre-flight failed, 2 a run failed (server/infrastructure), 4 timeout.

  Requires the GPU to be free: the owner must release it first. Run from any directory:
    pwsh -File C:\Users\sethy\Projects\tutor\vault\bench\model-bench\run-gpu-matrix.ps1

.PARAMETER DryRun   Print the plan and the harness commands without starting anything.
.PARAMETER Only     Wildcard filter on config ids, e.g. -Only 'rerank-*'.
#>
[CmdletBinding()]
param(
  [string]$Build = 'b11310',
  [string]$ModelsDir = 'C:\Users\sethy\Projects\vault-models',
  [string]$Dataset = 'C:\Users\sethy\Projects\vault-eval\external\bright\subset-v1',
  [string]$OutDir = 'C:\Users\sethy\Projects\tutor\vault\docs\phase0\model-bench',
  [string]$Bench = 'C:\Users\sethy\Projects\tutor\vault\bench\model-bench\target\release\model-bench.exe',
  [int]$MinFreeMiB = 18000,
  [int]$RunTimeoutMin = 60,
  [string]$Only = '*',
  [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
$Cargo = 'C:\Users\sethy\.cargo\bin\cargo.exe'
$CudaBin = Join-Path $ModelsDir "llama.cpp\llama-$Build-cuda-13.4\llama-server.exe"
$VulkanBin = Join-Path $ModelsDir "llama.cpp\llama-$Build-vulkan\llama-server.exe"
$Results = Join-Path $OutDir 'results.jsonl'
$Cands = Join-Path $OutDir 'first-stage-candidates.json'
$PidFile = Join-Path $OutDir 'running.pids'
$null = New-Item -ItemType Directory -Force $OutDir

function Say($m) { $l = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $m; Write-Host $l; Add-Content -Path (Join-Path $OutDir 'runner.log') -Value $l }
function Die($code, $m) { Say "ERROR: $m"; Cleanup; exit $code }

# Stop only the processes our harness recorded (name must still match), then drop the pid file.
function Cleanup {
  if (Test-Path $PidFile) {
    foreach ($line in Get-Content $PidFile) {
      $name, $procId = $line -split ' '
      $p = Get-Process -Id ([int]$procId) -ErrorAction SilentlyContinue
      if ($p -and $p.ProcessName -eq $name) { Say "stopping leftover $name (pid $procId)"; Stop-Process -Id $p.Id -Force }
    }
    Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
  }
}

function Is-Done($id) {
  if (-not (Test-Path $Results)) { return $false }
  foreach ($l in Get-Content $Results) {
    if ($l.Contains("`"config_id`":`"$id`"") -and ($l -match '"status":"(ok|sanity_failed|parity_failed)"')) { return $true }
  }
  return $false
}

# ---- model files (names as written by scripts/fetch_assets.py) ----
$E = Join-Path $ModelsDir 'embedders'; $R = Join-Path $ModelsDir 'rerankers'
$M = @{
  'q06q8'  = "$E\Qwen__Qwen3-Embedding-0.6B-GGUF__Qwen3-Embedding-0.6B-Q8_0.gguf"
  'q06f16' = "$E\Qwen__Qwen3-Embedding-0.6B-GGUF__Qwen3-Embedding-0.6B-f16.gguf"
  'q4q8'   = "$E\Qwen__Qwen3-Embedding-4B-GGUF__Qwen3-Embedding-4B-Q8_0.gguf"
  'q8q8'   = "$E\Qwen__Qwen3-Embedding-8B-GGUF__Qwen3-Embedding-8B-Q8_0.gguf"
  'bge'    = "$E\ggml-org__bge-m3-Q8_0-GGUF__bge-m3-q8_0.gguf"
  'har'    = "$E\mradermacher__harrier-oss-v1-0.6b-GGUF__harrier-oss-v1-0.6b.Q8_0.gguf"
  'r06'    = "$R\ggml-org__Qwen3-Reranker-0.6B-Q8_0-GGUF__qwen3-reranker-0.6b-q8_0.gguf"
  'r4'     = "$R\giladgd__Qwen3-Reranker-4B-GGUF__Qwen3-Reranker-4B.Q8_0.gguf"
  'r8'     = "$R\giladgd__Qwen3-Reranker-8B-GGUF__Qwen3-Reranker-8B.Q8_0.gguf"
}

# ---- matrix: order puts cheap, decisive runs first and the 8B runs last ----
# server flags per run: np slots, per-slot context, physical batch (-b = -ub), query format, pooling
$small = @{ np = 32; ctx = 1024; ub = 8192 }; $big = @{ np = 16; ctx = 1024; ub = 4096 }
$rsmall = @{ np = 16; ctx = 2048; ub = 8192 }; $rbig = @{ np = 8; ctx = 2048; ub = 4096 }
$Plan = @(@(
  @{ id = 'embed-qwen3-embedding-0.6b-q8_0-cuda';  kind = 'embed';  m = 'q06q8';  name = 'qwen3-embedding-0.6b'; quant = 'Q8_0'; pool = 'last'; qf = 'qwen3'; s = $small; cand = $true }
  @{ id = 'parity-qwen3-embedding-0.6b-q8_0-cuda-vs-vulkan'; kind = 'parity'; m = 'q06q8'; name = 'qwen3-embedding-0.6b'; quant = 'Q8_0'; pool = 'last'; s = $small }
  @{ id = 'embed-qwen3-embedding-0.6b-f16-cuda';   kind = 'embed';  m = 'q06f16'; name = 'qwen3-embedding-0.6b'; quant = 'f16'; pool = 'last'; qf = 'qwen3'; s = $small }
  @{ id = 'embed-bge-m3-q8_0-cuda';                kind = 'embed';  m = 'bge';    name = 'bge-m3';              quant = 'Q8_0'; pool = 'cls';  qf = 'none';  s = $small }
  @{ id = 'embed-harrier-oss-v1-0.6b-q8_0-cuda';   kind = 'embed';  m = 'har';    name = 'harrier-oss-v1-0.6b'; quant = 'Q8_0'; pool = 'last'; qf = 'harrier'; s = $small }
  @{ id = 'embed-qwen3-embedding-4b-q8_0-cuda';    kind = 'embed';  m = 'q4q8';   name = 'qwen3-embedding-4b';  quant = 'Q8_0'; pool = 'last'; qf = 'qwen3'; s = $big }
  @{ id = 'rerank-qwen3-reranker-0.6b-q8_0-cuda';  kind = 'rerank'; m = 'r06';    name = 'qwen3-reranker-0.6b'; quant = 'Q8_0'; pool = 'rank'; s = $rsmall }
  @{ id = 'rerank-qwen3-reranker-4b-q8_0-cuda';    kind = 'rerank'; m = 'r4';     name = 'qwen3-reranker-4b';   quant = 'Q8_0'; pool = 'rank'; s = $rbig }
  @{ id = 'embed-qwen3-embedding-8b-q8_0-cuda';    kind = 'embed';  m = 'q8q8';   name = 'qwen3-embedding-8b';  quant = 'Q8_0'; pool = 'last'; qf = 'qwen3'; s = $big }
  @{ id = 'rerank-qwen3-reranker-8b-q8_0-cuda';    kind = 'rerank'; m = 'r8';     name = 'qwen3-reranker-8b';   quant = 'Q8_0'; pool = 'rank'; s = $rbig }
  @{ id = 'parity-qwen3-embedding-0.6b-f16-cuda-vs-vulkan'; kind = 'parity'; m = 'q06f16'; name = 'qwen3-embedding-0.6b'; quant = 'f16'; pool = 'last'; s = $small }
) | Where-Object { $_.id -like $Only })

# ---- pre-flight ----
try {
  foreach ($f in @($CudaBin, $VulkanBin, $Bench) + $M.Values) { if (-not (Test-Path $f)) { Die 1 "missing file: $f (run scripts\fetch_assets.py / cargo build --release)" } }
  if (-not (Test-Path (Join-Path $Dataset 'subset.json'))) { Die 1 "dataset subset not found: $Dataset (run scripts\make_subset.py)" }
  Say "plan: $($Plan.Count) runs, dataset $Dataset, build $Build, results $Results"
  foreach ($p in $Plan) { Say ("  {0,-52} {1}" -f $p.id, $(if (Is-Done $p.id) { 'done (will skip)' } else { 'to run' })) }
  if (-not $DryRun) {
  # One-shot free-VRAM check (single query, not a loop): the owner must have released the GPU.
  $mem = (& nvidia-smi --query-gpu=memory.used,memory.total --format=csv,noheader,nounits) -split ',\s*'
  $free = [int]$mem[1] - [int]$mem[0]
  if ($free -lt $MinFreeMiB) { Die 1 "GPU still busy: $($mem[0]) MiB used of $($mem[1]); need >= $MinFreeMiB MiB free. Ask the owner to release it." }
  Say "GPU free: $free MiB"

  # Device names: pick the 5090 explicitly (Vulkan may also list the CPU's integrated GPU).
  function Pick-Device($bin, $label) {
    $txt = (& $bin --list-devices 2>&1 | Out-String)
    Set-Content (Join-Path $OutDir "devices-$label.txt") $txt
    $m = [regex]::Match($txt, '(?m)^\s*([A-Za-z]+\d+):\s*.*5090')
    if (-not $m.Success) { Die 1 "no RTX 5090 in $label --list-devices output:`n$txt" }
    return $m.Groups[1].Value
  }
  $CudaDev = Pick-Device $CudaBin 'cuda'; $VulkanDev = Pick-Device $VulkanBin 'vulkan'
  Say "devices: cuda=$CudaDev vulkan=$VulkanDev"
  } else { $CudaDev = 'CUDA0'; $VulkanDev = 'Vulkan0' }

  $t0 = Get-Date
  foreach ($p in $Plan) {
    if ((Is-Done $p.id) -and (($p.id -ne 'embed-qwen3-embedding-0.6b-q8_0-cuda') -or (Test-Path $Cands))) { Say "skip $($p.id) (result exists)"; continue }
    $log = Join-Path $OutDir "$($p.id).log"
    $a = @(
      $p.kind
      '--server-bin', $CudaBin, '--model', $M[$p.m], '--model-name', $p.name, '--quant', $p.quant,
      '--build', $Build, '--backend', 'cuda', '--device', $CudaDev, '--config-id', $p.id,
      '--results', $Results, '--log-dir', $OutDir, '--dataset', $Dataset, '--pooling', $p.pool,
      '--np', $p.s.np, '--slot-ctx', $p.s.ctx, '--ubatch', $p.s.ub, '--ngl', 99, '--threads', 4,
      '--pid-file', $PidFile, '--server-arg=--no-webui'
    )
    switch ($p.kind) {
      'embed' {
        $a += '--monitor-vram', '--query-format', $p.qf, '--autotune'
        if ($p.cand) { $a += '--save-candidates', $Cands }
      }
      'rerank' { $a += '--monitor-vram', '--candidates', $Cands }
      'parity' { $a += '--server-bin-b', $VulkanBin, '--backend-b', 'vulkan', '--build-b', $Build, '--device-b', $VulkanDev, '--n', 500 }
    }
    if ($p.kind -eq 'rerank' -and -not $DryRun -and -not (Test-Path $Cands)) { Die 1 "first-stage candidates missing ($Cands): run the 0.6B Q8_0 embed config first" }
    if ($DryRun) { Say "DRY $($p.id): $Bench $($a -join ' ')"; continue }
    Say "RUN $($p.id)"
    $t = Get-Date
    $proc = Start-Process -FilePath $Bench -ArgumentList $a -NoNewWindow -PassThru -RedirectStandardError $log -RedirectStandardOutput "$log.out"
    if (-not $proc.WaitForExit($RunTimeoutMin * 60 * 1000)) {
      Stop-Process -Id $proc.Id -Force
      Die 4 "$($p.id) exceeded $RunTimeoutMin min; harness killed. See $log"
    }
    $code = $proc.ExitCode
    Cleanup
    $dur = [int]((Get-Date) - $t).TotalSeconds
    switch ($code) {
      0 { Say "ok   $($p.id) in ${dur}s" }
      3 { Say "WARNING $($p.id): reranker sanity test FAILED (result line written; matrix continues). See $log" }
      default {
        $tail = (Get-Content $log -Tail 12) -join "`n"
        Die 2 "$($p.id) failed (exit $code) after ${dur}s. Stopping. Fix and re-run this script; finished configs are skipped. Logs: $OutDir\$($p.id)*`n$tail"
      }
    }
    if ($p.kind -eq 'parity') { $last = Get-Content $Results -Tail 1; if ($last -match '"status":"parity_failed"') { Say "WARNING $($p.id): parity below threshold (see results.jsonl)" } }
  }
  if ($DryRun) { Say 'dry run: nothing started'; exit 0 }
  Say ("matrix complete in {0:N1} min; results: {1}" -f ((Get-Date) - $t0).TotalMinutes, $Results)

  # Compact summary
  foreach ($l in Get-Content $Results) {
    $o = $l | ConvertFrom-Json
    $mm = $o.metrics
    $sum = switch ($o.kind) {
      'embed' { "ndcg@10 full/512/256 = {0:N3}/{1:N3}/{2:N3}  recall@100 = {3:N3}  {4:N0} tok/s  VRAM peak {5} MiB" -f $mm.macro_mean.full.'ndcg@10', $mm.macro_mean.'512'.'ndcg@10', $mm.macro_mean.'256'.'ndcg@10', $mm.macro_mean.full.'recall@100', $o.throughput.tokens_per_s, $o.vram.peak_mib }
      'rerank' { if ($o.status -eq 'ok') { "ndcg@10 {0:N3} -> {1:N3}  {2:N1} pairs/s  VRAM peak {3} MiB" -f $mm.macro_mean.'first_stage_ndcg@10', $mm.macro_mean.'reranked_ndcg@10', $o.throughput.pairs_per_s, $o.vram.peak_mib } else { 'SANITY FAILED' } }
      'parity' { "min cos {0:N5}  mean {1:N5}  failures {2}" -f $mm.min_cosine, $mm.mean_cosine, $mm.failures }
    }
    Say ("  {0,-52} {1,-14} {2}" -f $o.config_id, $o.status, $sum)
  }
}
finally { Cleanup }
