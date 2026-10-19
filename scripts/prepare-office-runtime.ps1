param(
    [string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/prerequisites'),
    [string]$InstallerPath = ''
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$payload = Join-Path $Destination 'LibreOffice-26.8.1.msi'
if ($InstallerPath) {
    if ([IO.Path]::GetFullPath($InstallerPath) -ne [IO.Path]::GetFullPath($payload)) {
        Copy-Item -LiteralPath $InstallerPath -Destination $payload -Force
    }
} elseif (-not (Test-Path -LiteralPath $payload)) {
    $uri = 'https://download.documentfoundation.org/libreoffice/stable/26.8.1/win/x86_64/LibreOffice_26.8.1_Win_x86-64.msi'
    Invoke-WebRequest -Uri $uri -OutFile ($payload + '.part') -UseBasicParsing -TimeoutSec 900
    Move-Item -LiteralPath ($payload + '.part') -Destination $payload -Force
}
$signature = Get-AuthenticodeSignature -LiteralPath $payload
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'The Document Foundation') {
    throw 'LibreOffice payload does not have a valid The Document Foundation signature.'
}
$hash = (Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash
[IO.File]::WriteAllText((Join-Path $Destination 'office-runtime-version.iss'), ('#define OfficeRuntimeSha256 "' + $hash + '"'), [Text.UTF8Encoding]::new($false))
Write-Output "LibreOffice payload ready: $payload"
