param([string]$SetupPath = '', [switch]$IncludeOffice)
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
$components = 'core,spell,notes,music,clip'
if ($IncludeOffice) { $components += ',office\writer,office\impress,office\calc' }
$setupArgs = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-','/NOICONS','/TASKS=""',('/COMPONENTS="'+$components+'"'),('/DIR="'+$fixture+'"'),('/LOG="'+$log+'"'))
$process = Start-Process -FilePath $SetupPath -ArgumentList $setupArgs -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Actual offline Setup failed: $($process.ExitCode). See $log" }
$setupLog = Get-Content -LiteralPath $log -Raw
if ($setupLog -notmatch 'WEBVIEW_RUNTIME_(READY|SKIP)') { throw 'Setup prerequisite completion missing' }
if (Test-Path -LiteralPath (Join-Path $fixture 'prerequisites/DeepTutor')) { throw 'Removed tutor payload was installed' }
if ($IncludeOffice -and $setupLog -notmatch 'OFFICE_RUNTIME_READY') { throw 'Office preparation did not complete' }
$reportPath = Join-Path $fixture 'options-report.json'
$check = Start-Process -FilePath (Join-Path $fixture 'PentaKuru.exe') -ArgumentList @('--installer-options-check',('"'+$reportPath+'"')) -WorkingDirectory $fixture -WindowStyle Hidden -Wait -PassThru
if ($check.ExitCode -ne 0) { throw 'Installed application check failed' }
$options = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
if ('Tutor' -in $options.dock_icons -or 'Spell' -notin $options.dock_icons -or -not $options.music -or -not $options.clip) { throw 'Installed selections/icons missing' }
if ($IncludeOffice -and (-not $options.writer -or -not $options.impress -or -not $options.calc)) { throw 'Installed Office selections missing' }
$uninstall = Start-Process -FilePath (Join-Path $fixture 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -WindowStyle Hidden -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw 'Test uninstall failed' }
@{passed=$true;actual_setup_callbacks=$true;selected_icons=$true;office_selected=[bool]$IncludeOffice;removed_tutor_payload=$true;webview='existing Microsoft runtime detected';clean_windows_vm_tested=$false} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $projectRoot 'docs/OFFLINE-SETUP-VERIFICATION.json') -Encoding utf8
Write-Output 'Actual Setup completed without the removed tutor runtime.'
