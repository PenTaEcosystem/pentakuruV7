param([string]$CompilerPath = '')
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
if (-not $CompilerPath) {
    $candidates = @((Join-Path $projectDir '.build\installer-tools\inno\ISCC.exe'), (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'), (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'))
    $CompilerPath = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $CompilerPath) { throw 'Install Inno Setup 6.7.3+ or provide -CompilerPath.' }
Push-Location $projectDir
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    & $CompilerPath (Join-Path $PSScriptRoot 'pentakuru-setup.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
    Get-FileHash -LiteralPath (Join-Path $projectDir 'dist\PentaKuru-Setup.exe') -Algorithm SHA256
} finally { Pop-Location }
