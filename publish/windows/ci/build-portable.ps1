Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "common.ps1")

$repoRoot = Get-RepoRoot
$paths = Initialize-StageDir -RepoRoot $repoRoot

if (Test-Path $paths.ZipPath) {
    Remove-Item -Force $paths.ZipPath
}

Compress-Archive -Path (Join-Path $paths.StageDir "*") -DestinationPath $paths.ZipPath
Write-Host "Created portable artifact: $($paths.ZipPath)"
