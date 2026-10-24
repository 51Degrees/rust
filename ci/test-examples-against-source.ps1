<#
.SYNOPSIS
Builds and tests the examples against this tree's crates, and fails if any
example still takes one of those crates from crates.io.

.DESCRIPTION
The examples depend on the published crates, and examples/source.toml patches
each one to its path in this tree. Cargo uses a patch only when its version is
the one it would pick, so this first writes a version above every published
release into the crates, then moves each patched crate in the examples' lock to
the copy in this tree, then checks the lock and runs the tests.

The manifests and the lock are put back as they were when the script ends, so a
run leaves the working tree unchanged.

.PARAMETER Version
The version to give the crates. The publish workflow passes the version it is
releasing. Without it, the next patch above the highest release tag is used,
such as 4.5.9 after 4.5.8.

.PARAMETER Root
The workspace root. Defaults to the repository this script sits in.
#>
param (
    [string]$Version,
    [string]$Root = (Join-Path $PSScriptRoot "..")
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

if (-not $Version) {
    # Release tags are plain versions. Tags with a suffix are pre-releases,
    # which a requirement such as 4.5.2 never selects, so they are left out.
    $released = git -C $Root tag --list |
        Where-Object { $_ -match '^\d+\.\d+\.\d+$' } |
        ForEach-Object { [version]$_ } |
        Sort-Object -Descending |
        Select-Object -First 1
    if (-not $released) {
        throw "No release tag found. The checkout needs the tags (fetch-depth: 0)."
    }
    $Version = "$($released.Major).$($released.Minor).$($released.Build + 1)"
}
Write-Output "Testing the examples against this tree's crates at $Version."

$examples = Join-Path $Root "examples"
$patched = Get-Content -Path (Join-Path $examples "source.toml") |
    Where-Object { $_ -match '^([A-Za-z0-9_-]+)\s*=\s*\{\s*path\s*=' } |
    ForEach-Object { $Matches[1] }
if ($patched.Count -eq 0) { throw "No patches found in examples/source.toml" }

# Every file the run changes, kept so it can be put back afterwards.
$changed = @(Get-ChildItem -Path $Root -Filter Cargo.toml -Recurse -File |
    Where-Object { $_.FullName -notmatch '[\\/](target|examples|owid-rust)[\\/]' }) +
    @(Get-Item -Path (Join-Path $examples "Cargo.lock"))
$originals = @{}
foreach ($file in $changed) {
    $originals[$file.FullName] = [System.IO.File]::ReadAllBytes($file.FullName)
}

Push-Location $examples
try {
    & (Join-Path $PSScriptRoot "set-crate-versions.ps1") -Version $Version -Root $Root

    # The committed lock names the published versions, and Cargo keeps a locked
    # version until that package is updated. Each crate is updated on its own,
    # because Cargo refuses the whole update if one named crate is in no
    # example's dependencies.
    $PSNativeCommandUseErrorActionPreference = $false
    foreach ($crate in $patched) {
        $output = cargo update --config source.toml -p $crate 2>&1 | ForEach-Object { "$_" }
        if ($LASTEXITCODE -ne 0) {
            if ($output -match 'did not match any packages') {
                Write-Output "  $crate is not used by any example."
                continue
            }
            $output | Write-Output
            throw "cargo update failed for $crate"
        }
    }
    $PSNativeCommandUseErrorActionPreference = $true

    # A patched crate that still resolves from crates.io carries a registry
    # source in the lock, where the copy in this tree carries none.
    $fromRegistry = (Get-Content -Raw -Path "Cargo.lock") -split '\[\[package\]\]' |
        ForEach-Object {
            $name = [regex]::Match($_, '(?m)^name = "([^"]+)"').Groups[1].Value
            if ($patched -contains $name -and $_ -match '(?m)^source = "registry') {
                $version = [regex]::Match($_, '(?m)^version = "([^"]+)"').Groups[1].Value
                "$name $version"
            }
        }
    if ($fromRegistry) {
        throw "These crates still come from crates.io, not this tree:`n$($fromRegistry -join "`n")"
    }

    cargo test --config source.toml
} finally {
    Pop-Location
    foreach ($path in $originals.Keys) {
        [System.IO.File]::WriteAllBytes($path, $originals[$path])
    }
}
