param([string]$Compiler='ISCC.exe',[switch]$TestBuild,[string]$MinimumRuntimeVersion='14.44.35211.0')
$ErrorActionPreference='Stop'
& (Join-Path $PSScriptRoot 'prepare-vc-runtime.ps1') -MinimumVersion $MinimumRuntimeVersion
& (Join-Path $PSScriptRoot 'prepare-office-runtime.ps1')
& (Join-Path $PSScriptRoot 'prepare-webview-runtime.ps1')
& (Join-Path $PSScriptRoot 'prepare-deeptutor-runtime.ps1')
$compilerArgs = @()
if ($TestBuild) { $compilerArgs += '/DPentaKuruTest' }
$compilerArgs += (Join-Path $PSScriptRoot 'pentakuru-setup.iss')
& $Compiler @compilerArgs
if ($LASTEXITCODE -ne 0) { throw 'Setup compilation failed.' }
