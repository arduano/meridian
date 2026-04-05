Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "common.ps1")

$repoRoot = Get-RepoRoot
$paths = Get-PublishPaths -RepoRoot $repoRoot
$cliPath = Join-Path $paths.StageDir "meridian.exe"

if (-not (Test-Path $cliPath)) {
    throw "Missing staged CLI binary: $cliPath"
}

& $cliPath --help | Out-Host

$geometryPath = Join-Path $env:TEMP "meridian-geometry-smoke.json"
if (Test-Path $geometryPath) {
    Remove-Item -Force $geometryPath
}

& $cliPath debug-piano-trail-classic-geometry --width 64 --height 64 | Set-Content -NoNewline $geometryPath

if (-not (Test-Path $geometryPath)) {
    throw "Geometry smoke output was not created: $geometryPath"
}

if ((Get-Item $geometryPath).Length -le 0) {
    throw "Geometry smoke output was empty: $geometryPath"
}
