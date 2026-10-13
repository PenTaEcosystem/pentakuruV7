$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
$fixtureDir = Join-Path $projectDir ('.build\installer-check-' + [Guid]::NewGuid().ToString('N').Substring(0,12))
$setupPath = Join-Path $projectDir '.build\installer-test\PentaKuru-Setup-Test.exe'
New-Item -ItemType Directory -Path $fixtureDir -Force | Out-Null

function Invoke-FixtureSetup([string]$Components) {
    $setupArgs = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', '/NOICONS', '/TASKS=""', ('/DIR="' + $fixtureDir + '"'), ('/COMPONENTS="' + $Components + '"'), ('/LOG="' + (Join-Path $fixtureDir 'setup.log') + '"'))
    $process = Start-Process -FilePath $setupPath -ArgumentList $setupArgs -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw ('Setup failed: ' + $process.ExitCode) }
    $reportPath = Join-Path $fixtureDir 'options-report.json'
    $check = Start-Process -FilePath (Join-Path $fixtureDir 'PentaKuru.exe') -ArgumentList @('--installer-options-check', ('"' + $reportPath + '"')) -WorkingDirectory $fixtureDir -WindowStyle Hidden -Wait -PassThru
    if ($check.ExitCode -ne 0) { throw 'Installed app check failed' }
    return (Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json)
}

$custom = Invoke-FixtureSetup 'core,search,office\writer,music,clip'
if (-not $custom.applied -or -not $custom.writer -or -not $custom.music -or -not $custom.clip -or $custom.impress -or $custom.calc -or $custom.deeptutor -or $custom.utilities.enable_sally -or $custom.utilities.enable_notes -or $custom.utilities.enable_ui_tars) { throw 'Custom choices were not applied accurately' }
$sourceHash = (Get-FileHash -LiteralPath (Join-Path $projectDir 'target\release\pentakuru.exe')).Hash
if ((Get-FileHash -LiteralPath (Join-Path $fixtureDir 'PentaKuru.exe')).Hash -ne $sourceHash) { throw 'Installed executable differs from release' }
if (Test-Path -LiteralPath (Join-Path $fixtureDir 'Penteye Notes')) { throw 'Runtime was installed before the application was opened' }
$firstPlan = Get-Content -LiteralPath (Join-Path $fixtureDir 'installation-options.json') -Raw | ConvertFrom-Json
# Keep an actual user-data fixture across upgrade and uninstall.
$notesDir = Join-Path $fixtureDir 'Penteye Notes'
New-Item -ItemType Directory -Path $notesDir -Force | Out-Null
Set-Content -LiteralPath (Join-Path $notesDir 'keep.txt') -Value 'fixture user data'
$minimal = Invoke-FixtureSetup 'core'
if (-not $minimal.applied -or $minimal.utilities.enable_office -or $minimal.music -or $minimal.clip -or $minimal.deeptutor -or $minimal.utilities.enable_notes -or $minimal.utilities.enable_sally) { throw 'Minimal choices were not applied accurately' }
$secondPlan = Get-Content -LiteralPath (Join-Path $fixtureDir 'installation-options.json') -Raw | ConvertFrom-Json
if ($firstPlan.installation_id -eq $secondPlan.installation_id) { throw 'Upgrade did not create a new selection revision' }
if (-not (Test-Path -LiteralPath (Join-Path $notesDir 'keep.txt'))) { throw 'Upgrade removed user data' }
$all = Invoke-FixtureSetup 'core,notes,search,spell,sally,office\writer,office\impress,office\calc,music,clip,deeptutor,uitars'
if (-not $all.applied -or -not $all.writer -or -not $all.impress -or -not $all.calc -or -not $all.music -or -not $all.clip -or -not $all.deeptutor -or -not $all.utilities.enable_ui_tars -or -not $all.utilities.enable_notes -or -not $all.utilities.enable_search -or -not $all.utilities.enable_spell -or -not $all.utilities.enable_sally) { throw 'All-component choices were not applied accurately' }
$uninstall = Start-Process -FilePath (Join-Path $fixtureDir 'unins000.exe') -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -WindowStyle Hidden -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw 'Uninstall failed' }
if (Test-Path -LiteralPath (Join-Path $fixtureDir 'PentaKuru.exe')) { throw 'Uninstall left the application executable' }
if (-not (Test-Path -LiteralPath (Join-Path $notesDir 'keep.txt'))) { throw 'Uninstall removed user data' }
@{passed=$true;custom_choices=$true;minimal_choices=$true;all_choices=$true;installed_exe_matches_release=$true;no_premature_runtime=$true;upgrade_preserves_data=$true;uninstall_preserves_data=$true} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $projectDir 'docs\INSTALLER-VERIFICATION.json') -Encoding utf8
Write-Output 'Installer selection, upgrade and uninstall checks passed.'
