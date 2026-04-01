Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "common.ps1")

$repoRoot = Get-RepoRoot
$paths = Initialize-StageDir -RepoRoot $repoRoot

$makensis = Get-Command makensis -ErrorAction SilentlyContinue
if (-not $makensis) {
    throw "makensis not found in PATH"
}

& $makensis.Source `
    "/DSTAGE_DIR=$($paths.StageDir)" `
    "/DOUT_FILE=$($paths.InstallerPath)" `
    (Join-Path $repoRoot "publish\windows\nsis\installer.nsi")

Write-Host "Created installer artifact: $($paths.InstallerPath)"
