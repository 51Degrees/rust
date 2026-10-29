param (
    [Parameter(Mandatory = $true)]
    [string]$RepoName
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

# Readies the cxx submodule, which holds the C sources the -sys crates compile
# and, in its own data submodules, the on-premise data files and the evidence
# the tests read. common-ci clones the repository with its submodules, so the
# update below normally finds nothing to do, and it is there so the script
# also works on a clone made without them.
#
# Every variable set below is an environment variable rather than a
# PowerShell one, so it survives into build-project.ps1 and
# run-unit-tests.ps1, which common-ci runs in this same process.

Write-Output "::group::Check out the cxx submodule"
git -C $RepoName submodule update --init --recursive
Write-Output "::endgroup::"

# The data files are Git LFS content. A clone brings down the pointer files
# alone, and the native loader reads a pointer as corrupt data, so fetch the
# real content before anything tries to open a data file.
Write-Output "::group::Fetch LFS content for the cxx submodule"
git -C "$RepoName/cxx" submodule foreach --recursive 'git lfs install --local && git lfs pull'
Write-Output "::endgroup::"

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
