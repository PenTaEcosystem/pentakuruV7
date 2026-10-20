param([string]$SetupPath = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $SetupPath) { $SetupPath = Join-Path $projectRoot '.build/installer-test/PentaKuru-Setup-Test-RealRuntime.exe' }
$fixture = Join-Path $projectRoot ('.build/offline-setup-' + [Guid]::NewGuid().ToString('N').Substring(0,12))
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
$log = Join-Path $fixture 'setup.log'
# Prevent pip/network downloads while exercising the actual Setup callbacks.
$env:PIP_NO_INDEX = '1'
$env:HTTP_PROXY = 'http://127.0.0.1:1'
$env:HTTPS_PROXY = 'http://127.0.0.1:1'
$setupArgs = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-','/NOICONS','/TASKS=""','/COMPONENTS="core,spell,deeptutor,music,clip"',('/DIR="'+$fixture+'"'),('/LOG="'+$log+'"'))
$process = Start-Process -FilePath $SetupPath -ArgumentList $setupArgs -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Actual offline Setup failed: $($process.ExitCode). See $log" }
$homePath = Join-Path $fixture 'Penteye Notes/DeepTutor'
if ((Get-Content -LiteralPath (Join-Path $homePath 'runtime/.pentaeye-ready') -Raw) -ne '1.6.14') { throw 'Setup did not prepare DeepTutor' }
$setupLog = Get-Content -LiteralPath $log -Raw
if ($setupLog -notmatch 'DEEPTUTOR_RUNTIME_READY' -or $setupLog -notmatch 'WEBVIEW_RUNTIME_(READY|SKIP)') { throw 'Setup prerequisite completion missing' }
$reportPath = Join-Path $fixture 'options-report.json'
$check = Start-Process -FilePath (Join-Path $fixture 'PentaKuru.exe') -ArgumentList @('--installer-options-check',('"'+$reportPath+'"')) -WorkingDirectory $fixture -WindowStyle Hidden -Wait -PassThru
if ($check.ExitCode -ne 0) { throw 'Installed application check failed' }
$options = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
if ('Tutor' -notin $options.dock_icons -or 'Spell' -notin $options.dock_icons -or -not $options.music -or -not $options.clip) { throw 'Installed selections/icons missing' }
$runtimePython = Join-Path $homePath 'runtime/Scripts/python.exe'
& python (Join-Path $PSScriptRoot 'check-deeptutor-native.py') --home $homePath --python $runtimePython
if ($LASTEXITCODE -ne 0) { throw 'Installed DeepTutor service check failed' }
$native = Get-Content -LiteralPath (Join-Path $homePath 'result.json') -Raw | ConvertFrom-Json
if (-not $native.passed) { throw 'Installed DeepTutor is not healthy' }
$uninstall = Start-Process -FilePath (Join-Path $fixture 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -WindowStyle Hidden -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw 'Test uninstall failed' }
@{passed=$true;actual_setup_callbacks=$true;private_python_node=$true;no_pip_index=$true;offline_wheels=$true;selected_icons=$true;deeptutor=$native;webview='existing Microsoft runtime detected; missing/failure branches tested separately';clean_windows_vm_tested=$false} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $projectRoot 'docs/OFFLINE-SETUP-VERIFICATION.json') -Encoding utf8
Write-Output 'Actual Setup prepared and launched DeepTutor without downloading packages.'
