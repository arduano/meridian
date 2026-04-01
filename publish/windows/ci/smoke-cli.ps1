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
