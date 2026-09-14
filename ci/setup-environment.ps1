param (
    [Parameter(Mandatory = $true)]
    [string]$RepoName
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

# Puts the C and C++ sources the -sys crates compile beside the workspace and
# points each crate's build script at its checkout, which is what
# pull-request.yml does with actions/checkout steps. The repositories are not
# submodules of the workspace, so they have to be fetched separately, and the
# override variables are used rather than the ../../<repo>-cxx sibling layout
# a developer machine has, because the clone here sits under the common-ci
# working directory instead.
#
# Every variable set below is an environment variable rather than a
# PowerShell one, so it survives into build-project.ps1 and
# run-unit-tests.ps1, which common-ci runs in this same process.

$Sources = @(
    @{ Repo = "common-cxx";           Variable = "FIFTYONE_COMMON_CXX_DIR" },
    @{ Repo = "device-detection-cxx"; Variable = "FIFTYONE_DEVICE_DETECTION_CXX_DIR" },
    @{ Repo = "ip-intelligence-cxx";  Variable = "FIFTYONE_IP_INTELLIGENCE_CXX_DIR" }
)

New-Item -ItemType Directory -Force -Path cxx | Out-Null

foreach ($Source in $Sources) {
    $Path = "cxx/$($Source.Repo)"

    Write-Output "::group::Clone $($Source.Repo)"
    # Recursive brings down each repository's own nested sources, being
    # src/common-cxx, src/ip-graph-cxx and the data submodules that carry the
    # on-premise data files and the evidence the tests read.
    git clone --depth 1 --recurse-submodules --shallow-submodules `
        "https://github.com/51Degrees/$($Source.Repo)" $Path
    Write-Output "::endgroup::"

    # The data files are Git LFS content. A clone brings down the pointer
    # files alone, and the native loader reads a pointer as corrupt data, so
    # fetch the real content before anything tries to open a data file.
    Write-Output "::group::Fetch LFS content for $($Source.Repo)"
    git -C $Path lfs install --local
    git -C $Path lfs pull
    git -C $Path submodule foreach --recursive 'git lfs install --local && git lfs pull'
    Write-Output "::endgroup::"

    $Absolute = (Resolve-Path $Path).Path
    Set-Item -Path "env:$($Source.Variable)" -Value $Absolute
    Write-Output "$($Source.Variable)=$Absolute"
}

# Any compiler or documentation warning fails the build, which is the rule
# pull-request.yml sets and which the crates are written to meet.
$env:RUSTFLAGS = "-D warnings"
$env:RUSTDOCFLAGS = "-D warnings"
$env:CARGO_TERM_COLOR = "always"

# The hosted images carry a Rust toolchain, and the format and lint stages
# need two components that are not always part of a minimal profile, so ask
# for them rather than assume them.
Write-Output "::group::Rust toolchain"
rustup component add clippy rustfmt
rustc --version
cargo --version
cmake --version
Write-Output "::endgroup::"
