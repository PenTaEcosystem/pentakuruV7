param([string]$SetupPath = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $SetupPath) { $SetupPath = Join-Path $projectRoot '.build/installer-test/PentaKuru-Setup-Test.exe' }
$base = Join-Path $projectRoot ('.build/webview-check-' + [Guid]::NewGuid().ToString('N').Substring(0,12))
New-Item -ItemType Directory -Path $base -Force | Out-Null
$results = @{}
foreach ($case in @('present','missing','failure')) {
    $fixture = Join-Path $base $case
    $log = Join-Path $base ($case + '.log')
    $setupArgs = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-','/NOICONS','/TASKS=""','/COMPONENTS="core"',('/DIR="'+$fixture+'"'),('/LOG="'+$log+'"'),('/WEBVIEWTEST='+$case))
    $process = Start-Process -FilePath $SetupPath -ArgumentList $setupArgs -WindowStyle Hidden -Wait -PassThru
    $text = Get-Content -LiteralPath $log -Raw
    if ($case -eq 'failure') {
        if ($process.ExitCode -eq 0 -or (Test-Path -LiteralPath (Join-Path $fixture 'PentaKuru.exe'))) { throw 'WebView2 failure did not block app installation' }
    } else {
        if ($process.ExitCode -ne 0) { throw "WebView2 $case failed: $($process.ExitCode)" }
        $expected = if ($case -eq 'present') { 'WEBVIEW_RUNTIME_SKIP' } else { 'WEBVIEW_RUNTIME_READY' }
        if ($text -notmatch $expected) { throw "WebView2 $case did not take the expected branch" }
        $uninstall = Start-Process -FilePath (Join-Path $fixture 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -WindowStyle Hidden -Wait -PassThru
        if ($uninstall.ExitCode -ne 0) { throw 'Test uninstall failed' }
    }
    $results[$case] = @{passed=$true;exit_code=$process.ExitCode}
}
@{passed=$true;outcomes='simulated in a test build; shared WebView2 was not changed';cases=$results} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $projectRoot 'docs/WEBVIEW-RUNTIME-VERIFICATION.json') -Encoding utf8
Write-Output 'WebView2 prerequisite checks passed.'
