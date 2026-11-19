$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    Push-Location 'vendor/Sally'
    try {
        if (-not (Test-Path -LiteralPath 'node_modules')) {
            npm.cmd ci
            if ($LASTEXITCODE -ne 0) { throw 'Sally dependency installation failed' }
        }
        npm.cmd run build
        if ($LASTEXITCODE -ne 0) { throw 'Sally frontend build failed' }
    } finally { Pop-Location }

    Write-Host '=== Building PentaKuru Full (Windows x64) ==='
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Full Rust build failed' }
    python scripts/package-sally.py --name PentaKuru-Full --edition full --binary target/release/pentakuru.exe --source-name PentaKuru-v0.2.1
    if ($LASTEXITCODE -ne 0) { throw 'Full packaging failed' }

    Write-Host '=== Building PentaKuru Lite (Windows x64, no LibreOffice) ==='
    cargo build --release --locked --no-default-features --features lite
    if ($LASTEXITCODE -ne 0) { throw 'Lite Rust build failed' }
    python scripts/package-sally.py --name PentaKuru-Lite --edition lite --binary target/release/pentakuru.exe --skip-source
    if ($LASTEXITCODE -ne 0) { throw 'Lite packaging failed' }
} finally { Pop-Location }
