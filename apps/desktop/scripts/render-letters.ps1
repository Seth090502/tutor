<#
.SYNOPSIS
  Render the Dual N-Back letter clips (C H K L Q R S T) to small mono WAVs with Windows SAPI.

.DESCRIPTION
  Offline and free: uses System.Speech, no network. The app trims silence, caps each clip under
  450 ms and loudness-normalises at startup, so the files are left as SAPI writes them.
  Run once, then commit the WAVs (docs/tutor/NBACK.md, "Timing engineering").

.EXAMPLE
  pwsh apps/desktop/scripts/render-letters.ps1
  pwsh apps/desktop/scripts/render-letters.ps1 -Voice "Microsoft David Desktop"
#>
param(
  [string]$Voice = "Microsoft Zira Desktop",
  [string]$OutDir = (Join-Path $PSScriptRoot "..\src\features\train\audio"),
  [int]$SampleRate = 22050,
  [int]$Rate = 0
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Speech

$letters = "C", "H", "K", "L", "Q", "R", "S", "T"
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
  $SampleRate, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)

New-Item -ItemType Directory -Force $OutDir | Out-Null
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  $installed = $synth.GetInstalledVoices() | Where-Object Enabled | ForEach-Object { $_.VoiceInfo.Name }
  if ($installed -notcontains $Voice) {
    throw "Voice '$Voice' is not installed. Installed: $($installed -join ', ')"
  }
  $synth.SelectVoice($Voice)
  $synth.Rate = $Rate
  foreach ($letter in $letters) {
    $path = Join-Path $OutDir "$($letter.ToLower()).wav"
    $prompt = New-Object System.Speech.Synthesis.PromptBuilder
    # Spell out, so "C" is read as the letter name rather than guessed as a word.
    $prompt.AppendTextWithHint($letter, [System.Speech.Synthesis.SayAs]::SpellOut)
    $synth.SetOutputToWaveFile($path, $format)
    $synth.Speak($prompt)
    $synth.SetOutputToNull()
    "{0} -> {1} ({2:N0} bytes)" -f $letter, $path, (Get-Item $path).Length
  }
}
finally {
  $synth.Dispose()
}
