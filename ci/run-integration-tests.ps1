param(
    # A resource key the public cloud accepts. When empty the Selenium
    # contract is skipped with a warning, so offline and fork runs stay green.
    [string]$TestResourceKey
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

# Browser contract check via the shared selenium-api-tests suite, following
# the same pattern as the other 51Degrees SDK repos (see for example
# device-detection-dotnet/ci/run-integration-tests.ps1): launch the cloud
# GettingStarted web example against the LOCAL source tree, shallow-clone the
# suite as a sibling directory (it is deliberately not a submodule), and run
# its Contract category against the example with headless Chrome.

if (-not $TestResourceKey) {
    Write-Host "::warning title=No Resource Key::No resource key; skipping the Selenium contract."
    exit 0
}

try {
    $example = $null

    # Build ahead of the run so the readiness wait only covers server start-up.
    # --config source.toml patches the fiftyone-* dependencies to this checkout,
    # so the example exercises the code under test rather than crates.io.
    Push-Location examples
    cargo build --config source.toml -p device-detection-examples --bin dd-web-getting-started-cloud

    # Start the example, pointed at the live cloud.
    $env:PORT = 8095
    ${env:51DEGREES_RESOURCE_KEY} = $TestResourceKey
    ${env:51DEGREES_CLOUD_ENDPOINT} = "https://cloud.51degrees.com/api/v4/"
    $example = cargo run --config source.toml -p device-detection-examples --bin dd-web-getting-started-cloud 2>&1 &
    Pop-Location

    # Get the shared contract tests.
    if (-not (Test-Path selenium-api-tests)) {
        git clone --depth 1 https://github.com/51Degrees/selenium-api-tests.git
    }

    # Wait for the example to come up. It resolves its cloud discovery as it
    # builds, so a bad key or unreachable cloud surfaces here.
    curl -sS -o $(if ($IsWindows) { 'NUL' } else { '/dev/null' }) --retry 10 --retry-connrefused "http://localhost:$env:PORT"

    $env:CLOUD_ROOT_URL = "https://cloud.51degrees.com/"
    $env:PAID_RESOURCE_KEY = $TestResourceKey
    $env:EXAMPLE_URL = "http://localhost:$env:PORT"
    $env:EXAMPLE_LANG = 'rust'
    dotnet test selenium-api-tests -c Release --filter TestCategory=Contract
} catch {
    if ($example) { Write-Host '>>> example app output >>>'; Receive-Job $example | Out-Host; Write-Host '<<< app output <<<' }
    throw
} finally {
    if ($example) { Remove-Job -Force $example }
    Remove-Item Env:PORT -ErrorAction SilentlyContinue
}
