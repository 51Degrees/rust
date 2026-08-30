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

    # ---------------------------------------------------------------
    # TEMPORARY DIAGNOSTIC, see 51Degrees/rust issue 24. Remove with the
    # branch that added it.
    #
    # SessionStorageCache_Chrome fails in CI on "the first page must call
    # the json endpoint at least once", while the same suite, the same
    # key and the same branch pass that assertion on a Windows
    # workstation. The client only posts when the served payload carries
    # a JavaScript property with a body to run, so this prints what the
    # cloud returns to this runner for the header sets the browser could
    # be sending. No resource key is printed, because the key lives in
    # the example rather than in these requests.
    Write-Host "===== diagnostic: browser and payload ====="
    if (Get-Command google-chrome -ErrorAction SilentlyContinue) {
        Write-Host "chrome: $(google-chrome --version)"
    }
    if (Get-Command chromedriver -ErrorAction SilentlyContinue) {
        Write-Host "chromedriver: $(chromedriver --version)"
    }

    $chromeUa = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 " +
        "(KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36"
    $headlessUa = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 " +
        "(KHTML, like Gecko) HeadlessChrome/151.0.0.0 Safari/537.36"
    $hints = @(
        'sec-ch-ua: "Chromium";v="151", "Not?A_Brand";v="24", "Google Chrome";v="151"',
        'sec-ch-ua-mobile: ?0',
        'sec-ch-ua-platform: "Linux"')

    $cases = @(
        @{ Name = 'plain Chrome user agent';      Ua = $chromeUa;   Hints = $false },
        @{ Name = 'HeadlessChrome, no hints';     Ua = $headlessUa; Hints = $false },
        @{ Name = 'HeadlessChrome, with hints';   Ua = $headlessUa; Hints = $true  }
    )

    foreach ($case in $cases) {
        $curlArgs = @('-sS', '-H', "User-Agent: $($case.Ua)")
        if ($case.Hints) { foreach ($h in $hints) { $curlArgs += @('-H', $h) } }
        $curlArgs += "http://localhost:$env:PORT/51Degrees.core.js"
        # curl returns one array element per line, so join before
        # measuring or the length is a line count.
        $body = (& curl @curlArgs) -join "`n"

        # A property with a body appears as `"name": "<script>"`. A
        # property with no body is absent or null, and then the client
        # has nothing to run and never posts.
        $runnable = @()
        foreach ($name in @('javascriptgethighentropyvalues',
                            'javascripthardwareprofile',
                            'screenpixelswidthjavascript',
                            'screenpixelsheightjavascript')) {
            if ($body -match ($name + '":\s*"')) { $runnable += $name }
        }
        $listed = if ($body -match '"javascriptProperties"') { 'yes' } else { 'no' }

        Write-Host ("  {0,-28} bytes={1,-7} list={2,-5} runnable={3}" -f `
            $case.Name, $body.Length, $listed, `
            $(if ($runnable.Count) { $runnable -join ',' } else { 'NONE' }))
    }
    # Second half of the diagnostic: drive the real browser through a
    # proxy that keeps what it served, so the include the browser
    # received can be counted rather than inferred from curl.
    Write-Host "  ----- browser leg -----"
    $includeFile = Join-Path (Get-Location) "diag-include.js"
    # The proxy reports and exits after this long, which has to outlast
    # the browser run below.
    $env:DIAG_WAIT_MS = 30000
    $proxy = Start-Process -FilePath "node" `
        -ArgumentList @("ci/diag-browser.js", "$env:PORT", "8099", "$includeFile") `
        -NoNewWindow -PassThru -RedirectStandardOutput "diag-proxy.log" `
        -RedirectStandardError "diag-proxy.err"
    Start-Sleep -Seconds 2

    $chrome = (Get-Command google-chrome -ErrorAction SilentlyContinue).Source
    if ($chrome) {
        $dom = & $chrome --headless=new --disable-gpu --no-sandbox `
            --virtual-time-budget=15000 `
            --user-data-dir="$(Join-Path (Get-Location) 'diag-chrome')" `
            --dump-dom "http://localhost:8099/page1" 2>$null
        $domText = $dom -join "`n"
        if ($domText -match 'data-state="([a-z-]+)"') {
            Write-Host "  page state: $($Matches[1])"
        }
        if ($domText -match '<td id="deviceid">([^<]*)</td>') {
            Write-Host "  device id rendered: '$($Matches[1])'"
        }
    } else {
        Write-Host "  google-chrome not found, browser leg skipped"
    }

    # Let the proxy reach its reporting timeout, then show what it saw.
    $proxy.WaitForExit(40000) | Out-Null
    Get-Content "diag-proxy.log" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host $_ }

    if (Test-Path $includeFile) {
        $served = Get-Content $includeFile -Raw
        $runnable = @()
        foreach ($name in @('javascriptgethighentropyvalues',
                            'javascripthardwareprofile',
                            'screenpixelswidthjavascript',
                            'screenpixelsheightjavascript')) {
            if ($served -match ($name + '":\s*"')) { $runnable += $name }
        }
        Write-Host ("  include as delivered to the browser: bytes={0} runnable={1}" -f `
            $served.Length,
            $(if ($runnable.Count) { $runnable -join ',' } else { 'NONE' }))
    } else {
        Write-Host "  the browser never fetched the include"
    }

    Write-Host "===== end diagnostic ====="

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
