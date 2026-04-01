Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-RepoRoot {
    return (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
}

function Get-WorkspaceVersion {
    param(
        [string]$RepoRoot
    )

    $cargoToml = Join-Path $RepoRoot "Cargo.toml"
    $content = Get-Content -Raw $cargoToml
    $match = [regex]::Match($content, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw "Could not determine workspace version from $cargoToml"
    }

    return $match.Groups[1].Value
}

function Get-PublishPaths {
    param(
        [string]$RepoRoot
    )

    $version = Get-WorkspaceVersion -RepoRoot $RepoRoot
    $artifactsRoot = Join-Path $RepoRoot "publish\windows\artifacts"
    $distRoot = Join-Path $artifactsRoot "dist"
    $stageRoot = Join-Path $artifactsRoot "stage"
    $prefix = "meridian-$version-windows-x86_64"

    return @{
        ArtifactsRoot = $artifactsRoot
        DistRoot = $distRoot
        StageRoot = $stageRoot
        Version = $version
        Prefix = $prefix
        StageDir = (Join-Path $stageRoot $prefix)
        ZipPath = (Join-Path $distRoot "$prefix.zip")
        InstallerPath = (Join-Path $distRoot "meridian-installer-$version-windows-x86_64.exe")
    }
}

function Initialize-StageDir {
    param(
        [string]$RepoRoot
    )

    $paths = Get-PublishPaths -RepoRoot $RepoRoot
    $targetRelease = Join-Path $RepoRoot "target\release"

    New-Item -ItemType Directory -Force -Path $paths.ArtifactsRoot, $paths.DistRoot, $paths.StageRoot | Out-Null
    if (Test-Path $paths.StageDir) {
        Remove-Item -Recurse -Force $paths.StageDir
    }
    New-Item -ItemType Directory -Force -Path $paths.StageDir | Out-Null

    $cliSource = Join-Path $targetRelease "meridian-cli.exe"
    if (-not (Test-Path $cliSource)) {
        throw "Missing CLI binary: $cliSource"
    }

    Copy-Item $cliSource (Join-Path $paths.StageDir "meridian.exe")

    $uiSource = Join-Path $targetRelease "meridian-ui.exe"
    $uiBuilt = Test-Path $uiSource
    if ($uiBuilt) {
        Copy-Item $uiSource (Join-Path $paths.StageDir "meridian-ui.exe")
    }

    $repoReadme = Join-Path $RepoRoot "README.md"
    if (Test-Path $repoReadme) {
        Copy-Item $repoReadme (Join-Path $paths.StageDir "README.md")
    }

    $icon = Join-Path $RepoRoot "assets\icons\meridian.ico"
    if (Test-Path $icon) {
        Copy-Item $icon (Join-Path $paths.StageDir "meridian.ico")
    }

    $knownIssues = @(
        "Current Windows prototype notes:",
        "- meridian.exe was built natively on a GitHub Actions Windows runner.",
        "- meridian-ui.exe is included only if the native Windows build succeeded."
    )

    if (-not $uiBuilt) {
        $knownIssues += "- meridian-ui.exe was not produced by this workflow run."
    }

    Set-Content -Path (Join-Path $paths.StageDir "KNOWN_ISSUES.txt") -Value $knownIssues

    $paths["UiBuilt"] = $uiBuilt
    return $paths
}
