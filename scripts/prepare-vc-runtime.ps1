param([string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/prerequisites'), [switch]$Refresh, [string]$MinimumVersion='14.44.35211.0')
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$runtimePath = Join-Path $Destination 'vc_redist.x64.exe'
if ($Refresh -or -not (Test-Path -LiteralPath $runtimePath)) {
    Invoke-WebRequest -Uri 'https://aka.ms/vc14/vc_redist.x64.exe' -OutFile ($runtimePath + '.part') -TimeoutSec 180
    Move-Item -LiteralPath ($runtimePath + '.part') -Destination $runtimePath -Force
}
$signature = Get-AuthenticodeSignature -LiteralPath $runtimePath
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
    throw 'Visual C++ payload does not have a valid Microsoft signature.'
}
$versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($runtimePath)
$version = '{0}.{1}.{2}.{3}' -f $versionInfo.FileMajorPart,$versionInfo.FileMinorPart,$versionInfo.FileBuildPart,$versionInfo.FilePrivatePart
if ($versionInfo.FileMajorPart -ne 14) { throw 'Unexpected Visual C++ runtime version.' }
if ([version]$MinimumVersion -gt [version]$version -or ([version]$MinimumVersion).Major -ne 14) { throw 'Invalid runtime minimum version.' }
$hash = (Get-FileHash -LiteralPath $runtimePath -Algorithm SHA256).Hash.ToLowerInvariant()
@("#define VcRuntimeMinimumVersion `"$MinimumVersion`"", "#define VcRuntimePayloadVersion `"$version`"", "#define VcRuntimeSHA256 `"$hash`"") | Set-Content -LiteralPath (Join-Path $Destination 'vc-runtime-version.iss') -Encoding utf8
@{source='https://aka.ms/vc14/vc_redist.x64.exe';version=$version;minimum_version=$MinimumVersion;sha256=$hash;signature='Valid';publisher=$signature.SignerCertificate.Subject} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Destination 'vc-runtime-manifest.json') -Encoding utf8
Write-Output "Verified Microsoft Visual C++ x64 $version"
