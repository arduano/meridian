Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "common.ps1")

$repoRoot = Get-RepoRoot
$paths = Get-PublishPaths -RepoRoot $repoRoot
$installerPath = $paths.InstallerPath

if (-not (Test-Path $installerPath)) {
    throw "Missing installer artifact: $installerPath"
}

$installDir = Join-Path $env:LOCALAPPDATA "Programs\Meridian"
if (Test-Path $installDir) {
    Remove-Item -Recurse -Force $installDir
}

& $installerPath /S | Out-Host

$installedCli = Join-Path $installDir "meridian.exe"
if (-not (Test-Path $installedCli)) {
    throw "Installer did not produce $installedCli"
}

& $installedCli --help | Out-Host

$installedUi = Join-Path $installDir "meridian-ui.exe"
if (-not (Test-Path $installedUi)) {
    throw "Installer did not produce $installedUi"
}

& $installedUi --help | Out-Host
