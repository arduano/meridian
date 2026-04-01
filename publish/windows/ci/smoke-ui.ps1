Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "common.ps1")

$repoRoot = Get-RepoRoot
$paths = Get-PublishPaths -RepoRoot $repoRoot
$uiPath = Join-Path $paths.StageDir "meridian-ui.exe"

if (-not (Test-Path $uiPath)) {
    Write-Host "Skipping UI smoke test because meridian-ui.exe was not built."
    exit 0
}

$process = Start-Process -FilePath $uiPath -PassThru
Start-Sleep -Seconds 5

if ($process.HasExited) {
    if ($process.ExitCode -ne 0) {
        throw "meridian-ui.exe exited early with code $($process.ExitCode)"
    }
} else {
    Stop-Process -Id $process.Id -Force
}

Write-Host "UI smoke test completed."
