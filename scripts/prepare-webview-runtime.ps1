param([string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/prerequisites'))
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$payload = Join-Path $Destination 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe'
if (-not (Test-Path -LiteralPath $payload)) {
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/?LinkId=2124703' -OutFile ($payload + '.part') -TimeoutSec 900
    Move-Item -LiteralPath ($payload + '.part') -Destination $payload -Force
}
$signature = Get-AuthenticodeSignature -LiteralPath $payload
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
    throw 'WebView2 payload does not have a valid Microsoft signature.'
}
$hash = (Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash
[IO.File]::WriteAllText((Join-Path $Destination 'webview-runtime-version.iss'), ('#define WebViewRuntimeSha256 "' + $hash + '"'), [Text.UTF8Encoding]::new($false))
Write-Output "Verified Microsoft WebView2: $payload"
