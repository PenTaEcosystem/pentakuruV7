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
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Combined Rust build failed' }
    python scripts/package-sally.py
    if ($LASTEXITCODE -ne 0) { throw 'Packaging failed' }
} finally { Pop-Location }
