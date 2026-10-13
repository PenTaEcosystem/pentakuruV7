param([Parameter(Mandatory=$true)][string]$Destination,[switch]$ForcePrivate)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$tutorRoot = [IO.Path]::GetFullPath($Destination)
$engineRoot = Join-Path $tutorRoot 'engines'
$cacheRoot = Join-Path $tutorRoot 'downloads'
New-Item -ItemType Directory -Path $engineRoot,$cacheRoot -Force | Out-Null
function Stage([string]$Text) { [IO.File]::WriteAllText((Join-Path $tutorRoot 'install-progress.txt'),$Text,[Text.UTF8Encoding]::new($false)) }
function Fetch([string]$Uri,[string]$Name,[string]$Hash) {
    $path = Join-Path $cacheRoot $Name
    if (-not (Test-Path -LiteralPath $path) -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $Hash) {
        Invoke-WebRequest -Uri $Uri -OutFile ($path+'.part') -UseBasicParsing -TimeoutSec 180
        if ((Get-FileHash -LiteralPath ($path+'.part') -Algorithm SHA256).Hash -ne $Hash) { throw 'Downloaded runtime checksum mismatch' }
        Move-Item -LiteralPath ($path+'.part') -Destination $path -Force
    }
    return $path
}
function Verify-Exe([string]$Path,[string]$Publisher) {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch $Publisher) { throw 'Runtime publisher signature is invalid' }
}
function Remove-EngineStage([string]$Path) {
    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not $resolved.StartsWith($engineRoot.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($resolved) -notin @('python-stage','node-stage')) { throw 'Invalid engine staging path' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
try {
    Stage '1/5 · Kiểm tra Python và Node.js'
    $pythonExe = $null
    $nodeDir = $null
    if (-not $ForcePrivate) {
        $candidate = Get-Command python.exe -ErrorAction SilentlyContinue
        if ($candidate -and $candidate.Source -notmatch '\\WindowsApps\\') {
            $version = & $candidate.Source -c 'import sys; print("%d.%d" % sys.version_info[:2])' 2>$null
            if ($LASTEXITCODE -eq 0 -and $version -match '^3\.(11|12|13|14)$') { $pythonExe = $candidate.Source }
        }
        $candidate = Get-Command node.exe -ErrorAction SilentlyContinue
        if ($candidate) {
            $version = & $candidate.Source --version 2>$null
            if ($LASTEXITCODE -eq 0 -and $version -match '^v(\d+)\.' -and [int]$Matches[1] -ge 20) { $nodeDir = Split-Path -Parent $candidate.Source }
        }
    }
    if (-not $pythonExe) {
        Stage '2/5 · Chuẩn bị Python riêng cho DeepTutor'
        $pythonDir = Join-Path $engineRoot 'python-3.13.12'
        $pythonExe = Join-Path $pythonDir 'tools\python.exe'
        if (-not (Test-Path -LiteralPath (Join-Path $pythonDir '.pentaeye-ready'))) {
            $archive = Fetch 'https://www.nuget.org/api/v2/package/python/3.13.12' 'python-3.13.12.zip' '389C0228AFBB3530F6CD079F5CEC44139B1441DD51C85EC704F7A63F1C9F3802'
            $stage = Join-Path $engineRoot 'python-stage'
            Remove-EngineStage $stage
            Expand-Archive -LiteralPath $archive -DestinationPath $stage
            Verify-Exe (Join-Path $stage 'tools\python.exe') 'Python Software Foundation'
            # Copy after a complete, verified extraction; retry can repair partial files.
            New-Item -ItemType Directory -Path $pythonDir -Force | Out-Null
            Copy-Item -LiteralPath (Join-Path $stage 'tools') -Destination $pythonDir -Recurse -Force
            [IO.File]::WriteAllText((Join-Path $pythonDir '.pentaeye-ready'),'3.13.12')
            Remove-EngineStage $stage
        }
        Verify-Exe $pythonExe 'Python Software Foundation'
    }
    if (-not $nodeDir) {
        Stage '3/5 · Chuẩn bị Node.js riêng cho DeepTutor'
        $nodeDir = Join-Path $engineRoot 'node-v24.21.0-win-x64'
        if (-not (Test-Path -LiteralPath (Join-Path $nodeDir '.pentaeye-ready'))) {
            $archive = Fetch 'https://nodejs.org/dist/v24.21.0/node-v24.21.0-win-x64.zip' 'node-v24.21.0-win-x64.zip' '158F7685B44DE51F6C0DF1D153526CBCD3E1BC739A8DFC607721CEF75DE9E541'
            $stage = Join-Path $engineRoot 'node-stage'
            Remove-EngineStage $stage
            Expand-Archive -LiteralPath $archive -DestinationPath $stage
            $extracted = Join-Path $stage 'node-v24.21.0-win-x64'
            Verify-Exe (Join-Path $extracted 'node.exe') 'OpenJS Foundation'
            New-Item -ItemType Directory -Path $nodeDir -Force | Out-Null
            Copy-Item -Path (Join-Path $extracted '*') -Destination $nodeDir -Recurse -Force
            [IO.File]::WriteAllText((Join-Path $nodeDir '.pentaeye-ready'),'24.21.0')
            Remove-EngineStage $stage
        }
        Verify-Exe (Join-Path $nodeDir 'node.exe') 'OpenJS Foundation'
    }
    $paths = @{python=$pythonExe;node_dir=$nodeDir} | ConvertTo-Json -Compress
    [IO.File]::WriteAllText((Join-Path $tutorRoot 'runtime-paths.json'),$paths,[Text.UTF8Encoding]::new($false))
    Stage '3/5 · Python và Node.js đã sẵn sàng'
} catch {
    [IO.File]::WriteAllText((Join-Path $tutorRoot 'bootstrap-error.txt'),$_.Exception.Message,[Text.UTF8Encoding]::new($false))
    Stage 'Không chuẩn bị được runtime. Kiểm tra Internet/proxy hoặc chính sách mạng rồi thử lại.'
    exit 1
}
