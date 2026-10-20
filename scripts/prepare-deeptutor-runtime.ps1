param([string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/prerequisites/DeepTutor'))
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$bootstrapHome = Join-Path $projectRoot '.build/deeptutor-bundle-builder'
& (Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe') -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'install-deeptutor.ps1') -Destination $bootstrapHome -ForcePrivate
if ($LASTEXITCODE -ne 0) { throw 'DeepTutor runtime preparation failed.' }
$runtimePaths = Get-Content -LiteralPath (Join-Path $bootstrapHome 'runtime-paths.json') -Raw | ConvertFrom-Json
$wheels = Join-Path $Destination 'wheels'
New-Item -ItemType Directory -Path $wheels -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $bootstrapHome 'downloads/python-3.13.12.zip'),(Join-Path $bootstrapHome 'downloads/node-v24.21.0-win-x64.zip') -Destination $Destination -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'assets/deeptutor-constraints.txt') -Destination (Join-Path $Destination 'constraints.txt') -Force
# Fail the build if a package needs a compiler on the recipient's PC.
& $runtimePaths.python -I -m pip download --only-binary=:all: --dest $wheels --constraint (Join-Path $Destination 'constraints.txt') 'deeptutor==1.6.14'
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare the complete Windows wheel bundle.' }
$files = @{}
Get-ChildItem -LiteralPath $Destination -Recurse -File | Where-Object { $_.Name -ne 'bundle-manifest.json' } | ForEach-Object {
    $relative = $_.FullName.Substring([IO.Path]::GetFullPath($Destination).TrimEnd('\').Length + 1).Replace('\','/')
    $files[$relative] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
}
$manifest = @{version='1.6.14';python='3.13.12';node='24.21.0';files=$files} | ConvertTo-Json -Depth 4
[IO.File]::WriteAllText((Join-Path $Destination 'bundle-manifest.json'), $manifest, [Text.UTF8Encoding]::new($false))
Write-Output "DeepTutor offline payload ready: $Destination"
