<#
.SYNOPSIS
Copies the 51Degrees C sources the -sys crates compile into each crate's
vendor folder, and records the upstream commits they came from.

.DESCRIPTION
Each -sys build script compiles every .c file in a fixed set of directories of
its checkout, so the vendored copy of a directory is exactly its .c and .h
files. The common-cxx copy for fiftyone-common-sys is the one
device-detection-cxx pins as its submodule, because the Device Detection
sources compile against those headers.

Each vendor folder gets an UPSTREAM.txt naming the repository and commit, and
for a repository with submodules, the submodule commits. CI reads it to check
out the same commits for the data files.

Files are written with LF line endings so the vendored copy is the same on
every platform.

.PARAMETER DeviceDetectionCxx
A device-detection-cxx checkout with its submodules.

.PARAMETER IpIntelligenceCxx
An ip-intelligence-cxx checkout with its submodules.

.PARAMETER Check
Compare instead of copy. Lists every vendored file that differs from the
checkouts, or is missing or extra, and every recorded commit that differs
from the checkout's, then exits 1 if there is any difference.

.EXAMPLE
pwsh ci/vendor-c-sources.ps1 -DeviceDetectionCxx ../device-detection-cxx -IpIntelligenceCxx ../ip-intelligence-cxx
#>
param(
    [Parameter(Mandatory)] [string] $DeviceDetectionCxx,
    [Parameter(Mandatory)] [string] $IpIntelligenceCxx,
    [switch] $Check
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$dd = (Resolve-Path $DeviceDetectionCxx).Path
$ipi = (Resolve-Path $IpIntelligenceCxx).Path

function Get-Commit([string] $Directory) {
    $sha = git -C $Directory rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw "$Directory is not a git checkout" }
    $sha.Trim()
}

# Each entry copies the files matching Patterns directly inside From (no
# subdirectories) to To, relative to the repository root.
$copies = @(
    @{ From = "$dd/src/common-cxx"; To = 'common-sys/vendor/common-cxx'; Patterns = '*.c', '*.h' }
    @{ From = "$dd/src"; To = 'device-detection-sys/vendor/device-detection-cxx/src'; Patterns = '*.c', '*.h' }
    @{ From = "$dd/src/hash"; To = 'device-detection-sys/vendor/device-detection-cxx/src/hash'; Patterns = '*.c', '*.h' }
    @{ From = "$dd/src/common-cxx"; To = 'device-detection-sys/vendor/device-detection-cxx/src/common-cxx'; Patterns = '*.h' }
    @{ From = "$ipi/src"; To = 'ip-intelligence-sys/vendor/ip-intelligence-cxx/src'; Patterns = '*.c', '*.h' }
    @{ From = "$ipi/src/ip-graph-cxx"; To = 'ip-intelligence-sys/vendor/ip-intelligence-cxx/src/ip-graph-cxx'; Patterns = '*.c', '*.h' }
    @{ From = "$ipi/src/common-cxx"; To = 'ip-intelligence-sys/vendor/ip-intelligence-cxx/src/common-cxx'; Patterns = '*.c', '*.h' }
)

$ddCommit = Get-Commit $dd
$ddCommon = Get-Commit "$dd/src/common-cxx"
$ipiCommit = Get-Commit $ipi
$ipiCommon = Get-Commit "$ipi/src/common-cxx"
$ipiGraph = Get-Commit "$ipi/src/ip-graph-cxx"

$records = @{
    'common-sys/vendor/UPSTREAM.txt' = @(
        'repository=51Degrees/common-cxx'
        "commit=$ddCommon"
        "taken-with=51Degrees/device-detection-cxx@$ddCommit"
    )
    'device-detection-sys/vendor/UPSTREAM.txt' = @(
        'repository=51Degrees/device-detection-cxx'
        "commit=$ddCommit"
        "src/common-cxx=$ddCommon"
    )
    'ip-intelligence-sys/vendor/UPSTREAM.txt' = @(
        'repository=51Degrees/ip-intelligence-cxx'
        "commit=$ipiCommit"
        "src/common-cxx=$ipiCommon"
        "src/ip-graph-cxx=$ipiGraph"
    )
}

function Get-Lf([string] $Path) {
    ([System.IO.File]::ReadAllText($Path)) -replace "`r`n", "`n"
}

$differences = [System.Collections.Generic.List[string]]::new()
foreach ($copy in $copies) {
    $to = Join-Path $root $copy.To
    $wanted = @{}
    foreach ($pattern in $copy.Patterns) {
        Get-ChildItem -LiteralPath $copy.From -Filter $pattern -File |
            ForEach-Object { $wanted[$_.Name] = $_.FullName }
    }
    if ($wanted.Count -eq 0) { throw "no sources found in $($copy.From)" }
    $present = @{}
    if (Test-Path -LiteralPath $to) {
        foreach ($pattern in $copy.Patterns) {
            Get-ChildItem -LiteralPath $to -Filter $pattern -File |
                ForEach-Object { $present[$_.Name] = $_.FullName }
        }
    }
    if ($Check) {
        foreach ($name in $wanted.Keys) {
            if (-not $present.ContainsKey($name)) {
                $differences.Add("missing   $($copy.To)/$name")
            } elseif ((Get-Lf $wanted[$name]) -cne (Get-Lf $present[$name])) {
                $differences.Add("differs   $($copy.To)/$name")
            }
        }
        foreach ($name in $present.Keys) {
            if (-not $wanted.ContainsKey($name)) {
                $differences.Add("extra     $($copy.To)/$name")
            }
        }
        continue
    }
    New-Item -ItemType Directory -Force -Path $to | Out-Null
    foreach ($name in $present.Keys) {
        if (-not $wanted.ContainsKey($name)) { Remove-Item -LiteralPath $present[$name] }
    }
    foreach ($name in $wanted.Keys) {
        [System.IO.File]::WriteAllText(
            (Join-Path $to $name),
            (Get-Lf $wanted[$name]),
            [System.Text.UTF8Encoding]::new($false))
    }
}

foreach ($record in $records.GetEnumerator()) {
    $path = Join-Path $root $record.Key
    $text = ($record.Value -join "`n") + "`n"
    if ($Check) {
        if (-not (Test-Path -LiteralPath $path)) {
            $differences.Add("missing   $($record.Key)")
        } elseif ((Get-Lf $path) -cne $text) {
            $differences.Add("differs   $($record.Key) (recorded commits are not the checkouts')")
        }
        continue
    }
    [System.IO.File]::WriteAllText($path, $text, [System.Text.UTF8Encoding]::new($false))
}

if ($Check) {
    if ($differences.Count -gt 0) {
        Write-Output "The vendored C sources differ from the checkouts:"
        $differences | Sort-Object | ForEach-Object { Write-Output "  $_" }
        exit 1
    }
    Write-Output "The vendored C sources match the checkouts."
} else {
    Write-Output "Vendored device-detection-cxx $ddCommit (common-cxx $ddCommon)"
    Write-Output "Vendored ip-intelligence-cxx $ipiCommit (common-cxx $ipiCommon, ip-graph-cxx $ipiGraph)"
}
