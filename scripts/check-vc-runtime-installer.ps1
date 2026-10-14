param([string]$SetupPath,[string]$ReportPath)
$ErrorActionPreference='Stop'
$projectDir=Split-Path -Parent $PSScriptRoot
if (!$SetupPath) { $SetupPath=Join-Path $projectDir '.build/installer-test/PentaKuru-Setup-Test.exe' }
if (!$ReportPath) { $ReportPath=Join-Path $projectDir 'docs/VC-RUNTIME-VERIFICATION.json' }
$base=Join-Path $projectDir ('.build/vc-runtime-check-' + [Guid]::NewGuid().ToString('N').Substring(0,12))
New-Item -ItemType Directory -Path $base -Force | Out-Null
$cases=@('present','newer','missing','old','repair','newer-race','declined','uac-cancel','failure','reboot')
$results=@{}
foreach ($case in $cases) {
    $fixture=Join-Path $base $case
    $log=Join-Path $base ($case+'.log')
    $setupArgs=@('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/RESTARTEXITCODE=3010','/SP-','/NOICONS','/TASKS=""','/COMPONENTS="core"',('/DIR="'+$fixture+'"'),('/LOG="'+$log+'"'),('/RUNTIMETEST='+$case))
    $process=Start-Process -FilePath $SetupPath -ArgumentList $setupArgs -WindowStyle Hidden -Wait -PassThru
    $text=Get-Content -LiteralPath $log -Raw
    $blocked=$case -in @('declined','uac-cancel','failure')
    if ($blocked) {
        if ($process.ExitCode -eq 0 -or (Test-Path -LiteralPath (Join-Path $fixture 'PentaKuru.exe'))) { throw "Runtime $case did not block app installation" }
    } else {
        $expected=if($case -eq 'reboot'){3010}else{0}
        if ($process.ExitCode -ne $expected -or !(Test-Path -LiteralPath (Join-Path $fixture 'PentaKuru.exe'))) { throw "Runtime $case installation failed: $($process.ExitCode)" }
        if ($case -in @('present','newer') -and $text -notmatch 'VC_RUNTIME_SKIP') { throw "Runtime $case did not skip dependency installer" }
        if ($case -in @('missing','old','newer-race') -and $text -notmatch 'VC_RUNTIME_START: /install') { throw "Runtime $case did not install dependency" }
        if ($case -eq 'repair' -and $text -notmatch 'VC_RUNTIME_START: /repair') { throw 'Corrupt runtime did not choose repair' }
        if ($case -eq 'reboot' -and $text -notmatch 'VC_RUNTIME_LAUNCH_READY=0') { throw 'Reboot case allowed app launch' }
        if ($case -ne 'reboot' -and $text -notmatch 'VC_RUNTIME_LAUNCH_READY=1') { throw "Runtime $case did not enable app launch" }
        $uninstall=Start-Process -FilePath (Join-Path $fixture 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -WindowStyle Hidden -Wait -PassThru
        if($uninstall.ExitCode -ne 0){throw 'Fixture uninstall failed'}
    }
    $results[$case]=@{passed=$true;exit_code=$process.ExitCode;blocked_before_app_install=$blocked}
    Write-Output "Runtime $case passed"
}
@{passed=$true;runtime_outcomes='simulated in test build; Microsoft installer was not executed';shared_runtime_not_modified=$true;cases=$results} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $ReportPath -Encoding utf8
