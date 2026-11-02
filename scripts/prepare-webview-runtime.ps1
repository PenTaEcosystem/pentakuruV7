param([string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/prerequisites'))
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$payload = Join-Path $Destination 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe'
function Test-StandalonePayload([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    # The ~2 MB Edge Update bootstrapper is signed too, but needs Internet.
    $file = Get-Item -LiteralPath $Path
    return $file.Length -ge 10MB
}
if (-not (Test-StandalonePayload $payload)) {
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/?LinkId=2124701' -OutFile ($payload + '.part') -TimeoutSec 900
    if (-not (Test-StandalonePayload ($payload + '.part'))) {
        throw 'Expected the full x64 WebView2 offline installer, not the online bootstrapper.'
    }
    $downloadSignature = Get-AuthenticodeSignature -LiteralPath ($payload + '.part')
    if ($downloadSignature.Status -ne 'Valid' -or $downloadSignature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
        throw 'Downloaded WebView2 installer does not have a valid Microsoft signature.'
    }
    Move-Item -LiteralPath ($payload + '.part') -Destination $payload -Force
}
$signature = Get-AuthenticodeSignature -LiteralPath $payload
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
    throw 'WebView2 payload does not have a valid Microsoft signature.'
}
$hash = (Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash
[IO.File]::WriteAllText((Join-Path $Destination 'webview-runtime-version.iss'), ('#define WebViewRuntimeSha256 "' + $hash + '"'), [Text.UTF8Encoding]::new($false))
Write-Output "Verified Microsoft WebView2: $payload"
