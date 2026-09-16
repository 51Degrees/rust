param(
    # Which device detection web example to test, the cloud one or the
    # on-premise one.
    [Parameter(Mandatory = $true)]
    [ValidateSet("cloud", "onpremise")]
    [string]$Mode,
    # A resource key the public cloud accepts. The cloud example uses it to
    # call the cloud. The suite also asks for one on every run, including the
    # on-premise one where nothing uses it.
    [Parameter(Mandatory = $true)]
    [string]$ResourceKey,
    # The Hash data file the on-premise example loads. The contract needs
    # DeviceType and the screen size JavaScript properties, which the Lite
    # file does not carry, so this is the TAC file.
    [string]$DataFile,
    # The port the example listens on. Each mode has its own so the two never
    # meet, and neither is the example's default.
    [int]$Port = $(if ($Mode -eq "cloud") { 8095 } else { 8096 }),
    # Where the shared suite is cloned to.
    [string]$SuiteDir = (Join-Path ([IO.Path]::GetTempPath()) "selenium-api-tests")
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

# Runs the Contract category of the shared selenium-api-tests suite
# (https://github.com/51Degrees/selenium-api-tests) against one of the two
# device detection web examples, built from this checkout. The suite is told
# the example is already running through EXAMPLE_URL, and EXAMPLE_LANG=rust
# gives it the JSON path the page posts to.
#
# Any test that fails, or that the suite skips as inconclusive, fails this
# script. A skip is how the suite reports that it could not check something,
# so a run with skips has not shown that the example meets the contract.
#
# No key or licence value is ever printed. Only variable names are.

$Bin = if ($Mode -eq "cloud") {
    "dd-web-getting-started-cloud"
} else {
    "dd-web-getting-started-onprem"
}
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Examples = Join-Path $Root "examples"
$Results = Join-Path $Root "test-results/browser-contract/$Mode"
$Logs = Join-Path ([IO.Path]::GetTempPath()) "browser-contract-$Mode"
New-Item -ItemType Directory -Force -Path $Results, $Logs | Out-Null

if (-not $ResourceKey) {
    throw "No resource key was given, so the suite cannot run."
}

if ($Mode -eq "onpremise") {
    if (-not $DataFile -or -not (Test-Path -PathType Leaf $DataFile)) {
        throw "The on-premise example needs -DataFile to name an existing " +
            "TAC Hash data file."
    }
    $DataFile = (Resolve-Path $DataFile).Path
}

# Replaces anything shaped like a resource key before a log is printed.
function Hide-Keys([string]$Text) {
    $Text -replace 'AQ[A-Za-z0-9_-]{12,}', '<redacted>'
}

# When the contract fails, shows what the example serves to the three kinds of
# browser the suite drives, so a failure can be traced without a browser. For
# each one it prints the include's size, whether node can parse it, and the
# JavaScript properties and errors in the data the include carries. The
# example has to still be running.
function Show-Includes([string]$Url, [string]$Directory) {
    $browsers = [ordered]@{
        "headless-chrome" = @{
            "User-Agent" = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 " +
                "(KHTML, like Gecko) HeadlessChrome/151.0.0.0 Safari/537.36"
            "sec-ch-ua" = '"Not=A?Brand";v="99", "Google Chrome";v="151", ' +
                '"Chromium";v="151"'
            "sec-ch-ua-mobile" = "?0"
            "sec-ch-ua-platform" = '"Linux"'
        }
        "firefox" = @{
            "User-Agent" = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:140.0) " +
                "Gecko/20100101 Firefox/140.0"
        }
        "iphone" = @{
            "User-Agent" = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) " +
                "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 " +
                "Mobile/15E148 Safari/604.1"
        }
    }
    foreach ($name in $browsers.Keys) {
        Write-Host "::group::Include served to $name"
        try {
            $path = Join-Path $Directory "include-$name.js"
            $response = Invoke-WebRequest -Uri "$Url/51Degrees.core.js" `
                -Headers $browsers[$name] -TimeoutSec 30
            $text = [string]$response.Content
            Set-Content -Path $path -Value $text -NoNewline
            Write-Host "status $($response.StatusCode), $($text.Length) characters"
            $PSNativeCommandUseErrorActionPreference = $false
            $check = node --check $path 2>&1
            Write-Host "node --check exit $LASTEXITCODE"
            if ($LASTEXITCODE -ne 0) {
                Hide-Keys ($check | Out-String) | Out-Host
            }
            $PSNativeCommandUseErrorActionPreference = $true
            # The template starts with 'var json = {...};' on one line.
            $match = [regex]::Match($text, 'var json\s*=\s*(\{.*?\});\s*var parameters')
            if (-not $match.Success) {
                Write-Host "no 'var json' object found, first 600 characters:"
                Hide-Keys $text.Substring(0, [Math]::Min(600, $text.Length)) |
                    Out-Host
                $at = $text.IndexOf("javascriptProperties")
                if ($at -ge 0) {
                    Write-Host "around javascriptProperties:"
                    Hide-Keys $text.Substring($at, [Math]::Min(800, $text.Length - $at)) |
                        Out-Host
                }
                continue
            }
            $json = $match.Groups[1].Value | ConvertFrom-Json -AsHashtable
            Write-Host "javascriptProperties: $($json.javascriptProperties -join ', ')"
            foreach ($property in @($json.javascriptProperties)) {
                $parts = $property -split '\.', 2
                $value = $json[$parts[0]][$parts[1]]
                $size = if ($null -eq $value) { "null" } else {
                    "$(([string]$value).Length) characters"
                }
                Write-Host "  $property = $size"
            }
            if ($json.errors) {
                Write-Host "errors: $(Hide-Keys ($json.errors | ConvertTo-Json -Compress))"
            }
            if ($json.device) {
                foreach ($key in "devicetype", "hardwarename", "browsername",
                    "deviceid", "screenpixelswidth") {
                    Write-Host "  device.$key = $($json.device[$key])"
                }
            }
        } catch {
            Write-Host "could not read the include: $(Hide-Keys $_.ToString())"
        } finally {
            $PSNativeCommandUseErrorActionPreference = $true
            Write-Host "::endgroup::"
        }
    }
}

# Build first, so the wait below only covers the example starting. The release
# profile compiles the native Hash code with optimisation, which keeps the
# TAC file load and each detection fast enough for the suite's timeouts.
# --config source.toml builds against the crates in this checkout rather than
# the published ones.
Write-Host "::group::Build $Bin"
Push-Location $Examples
try {
    cargo build --release --config source.toml -p device-detection-examples --bin $Bin
} finally {
    Pop-Location
}
Write-Host "::endgroup::"

$Exe = Join-Path $Examples "target/release/$Bin"
if ($IsWindows) { $Exe += ".exe" }

$example = $null
$passed = $false
try {
    # The example reads these at start-up. The names start with a digit, so
    # they need the brace form.
    $env:PORT = "$Port"
    if ($Mode -eq "cloud") {
        ${env:51DEGREES_RESOURCE_KEY} = $ResourceKey
        ${env:51DEGREES_CLOUD_ENDPOINT} = "https://cloud.51degrees.com/api/v4/"
    } else {
        ${env:51DEGREES_DD_PATH} = $DataFile
    }

    Write-Host "Starting $Bin on port $Port"
    $example = Start-Process -FilePath $Exe -PassThru -NoNewWindow `
        -RedirectStandardOutput (Join-Path $Logs "stdout.txt") `
        -RedirectStandardError (Join-Path $Logs "stderr.txt")

    # Wait for the page to answer. The cloud example fetches what the key can
    # see as it starts, and the on-premise one loads the whole data file into
    # memory, so allow a few minutes before giving up.
    $url = "http://localhost:$Port"
    $deadline = [DateTime]::UtcNow.AddMinutes(5)
    $ready = $false
    while (-not $ready -and [DateTime]::UtcNow -lt $deadline) {
        if ($example.HasExited) {
            throw "$Bin exited with code $($example.ExitCode) before it " +
                "answered on $url."
        }
        try {
            $response = Invoke-WebRequest -Uri "$url/" -TimeoutSec 10
            $ready = $response.StatusCode -eq 200
        } catch {
            Start-Sleep -Seconds 2
        }
    }
    if (-not $ready) {
        throw "$Bin did not answer on $url within 5 minutes."
    }
    Write-Host "$Bin is answering on $url"

    Write-Host "::group::Clone selenium-api-tests"
    if (-not (Test-Path $SuiteDir)) {
        git clone --depth 1 https://github.com/51Degrees/selenium-api-tests.git $SuiteDir
    }
    git -C $SuiteDir log -1 --format="selenium-api-tests at %H %s"
    Write-Host "::endgroup::"

    # The suite reads CLOUD_ROOT_URL and PAID_RESOURCE_KEY on every run, even
    # though an already running on-premise example makes no use of them.
    $env:CLOUD_ROOT_URL = "https://cloud.51degrees.com/"
    $env:PAID_RESOURCE_KEY = $ResourceKey
    $env:EXAMPLE_URL = $url
    $env:EXAMPLE_LANG = "rust"

    Write-Host "::group::Contract tests against the $Mode example"
    $trx = "contract-$Mode.trx"
    # The exit code is checked after the results file has been reported, so a
    # failing run still names every test that did not pass.
    $PSNativeCommandUseErrorActionPreference = $false
    dotnet test $SuiteDir -c Release --filter TestCategory=Contract `
        --logger "trx;LogFileName=$trx" `
        --logger "console;verbosity=normal" `
        --results-directory $Results
    $testExitCode = $LASTEXITCODE
    $PSNativeCommandUseErrorActionPreference = $true
    Write-Host "::endgroup::"

    # dotnet test passes a run whose tests were skipped as inconclusive, so
    # read the results file and fail on anything that did not pass, naming
    # each test and the reason the suite gave.
    $trxPath = Join-Path $Results $trx
    if (-not (Test-Path -PathType Leaf $trxPath)) {
        throw "dotnet test exited with code $testExitCode and wrote no " +
            "results file."
    }
    [xml]$report = Get-Content -Raw $trxPath
    $ns = @{ t = "http://microsoft.com/schemas/VisualStudio/TeamTest/2010" }
    $all = @(Select-Xml -Xml $report -Namespace $ns -XPath "//t:UnitTestResult" |
        ForEach-Object { $_.Node })
    $notPassed = @($all | Where-Object { $_.outcome -ne "Passed" })
    $summary = "Contract against the rust $Mode example: " +
        "$($all.Count - $notPassed.Count) of $($all.Count) passed"
    Write-Host $summary
    if ($env:GITHUB_STEP_SUMMARY) {
        "### $summary" >> $env:GITHUB_STEP_SUMMARY
    }
    foreach ($result in $notPassed) {
        $message = $result.Output.ErrorInfo.Message
        if (-not $message) {
            $message = ($result.Output.TextMessages.Message -join " ")
        }
        $line = Hide-Keys "$($result.outcome): $($result.testName): $message"
        Write-Host "::error title=Contract ($Mode)::$line"
        if ($env:GITHUB_STEP_SUMMARY) {
            "- $line" >> $env:GITHUB_STEP_SUMMARY
        }
    }
    if ($all.Count -eq 0) {
        throw "The results file lists no tests, so nothing was checked."
    }
    if ($testExitCode -ne 0) {
        throw "dotnet test exited with code $testExitCode."
    }
    if ($notPassed.Count -gt 0) {
        throw "$($notPassed.Count) Contract test(s) did not pass against " +
            "the $Mode example."
    }
    $passed = $true
} finally {
    # Read the includes while the example is still running.
    if (-not $passed -and $example -and -not $example.HasExited -and $url) {
        Show-Includes -Url $url -Directory $Logs
    }
    if ($example -and -not $example.HasExited) {
        Stop-Process -Id $example.Id -Force
        $example.WaitForExit()
    }
    if (-not $passed) {
        foreach ($name in "stdout.txt", "stderr.txt") {
            $path = Join-Path $Logs $name
            if (Test-Path $path) {
                Write-Host ">>> $Bin $name >>>"
                Hide-Keys (Get-Content -Raw $path) | Out-Host
                Write-Host "<<< $Bin $name <<<"
            }
        }
    }
    Remove-Item Env:PORT, Env:51DEGREES_RESOURCE_KEY, Env:51DEGREES_CLOUD_ENDPOINT, `
        Env:51DEGREES_DD_PATH -ErrorAction SilentlyContinue
}
