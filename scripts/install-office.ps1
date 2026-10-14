param([Parameter(Mandatory=$true)][string]$Destination, [switch]$ForcePrivate)
$env:PSModulePath = $PSHOME + '\Modules;' + $env:ProgramFiles + '\WindowsPowerShell\Modules'
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$officeBase = [IO.Path]::GetFullPath($Destination)
function Test-Office([string]$Program) {
    return (Test-Path -LiteralPath (Join-Path $Program 'python.exe')) -and (Test-Path -LiteralPath (Join-Path $Program 'soffice.exe'))
}
$officeProgram = Join-Path $officeBase 'runtime/LibreOffice/program'
$officeFlatProgram = Join-Path $officeBase 'runtime/program'
if ((Test-Office $officeProgram) -or (Test-Office $officeFlatProgram)) { Write-Output 'LibreOffice riêng đã sẵn sàng.'; exit 0 }
if (-not $ForcePrivate) {
    foreach ($root in @($env:ProgramW6432, ${env:ProgramFiles(x86)})) {
        if ($root -and (Test-Office (Join-Path $root 'LibreOffice/program'))) {
            Write-Output 'Dùng LibreOffice đã có trên máy.'; exit 0
        }
    }
}
New-Item -ItemType Directory -Path $officeBase -Force | Out-Null
$officeLog = Join-Path $officeBase 'install.log'
$officeMsiLog = Join-Path $officeBase 'msiexec.log'
try {
    $officeMsi = Join-Path $officeBase 'LibreOffice-26.8.1.msi'
    $officeUri = 'https://download.documentfoundation.org/libreoffice/stable/26.8.1/win/x86_64/LibreOffice_26.8.1_Win_x86-64.msi'
    function Test-SignedInstaller {
        if (-not (Test-Path -LiteralPath $officeMsi)) { return $false }
        $signature = Get-AuthenticodeSignature -LiteralPath $officeMsi
        return $signature.Status -eq 'Valid' -and $signature.SignerCertificate.Subject -match 'The Document Foundation'
    }
    if (-not (Test-SignedInstaller)) {
        $download = $officeMsi + '.download'
        $downloaded = $false
        for ($attempt = 1; $attempt -le 3; $attempt++) {
            try {
                Invoke-WebRequest -Uri $officeUri -OutFile $download -UseBasicParsing -TimeoutSec 900
                Move-Item -LiteralPath $download -Destination $officeMsi -Force
                $downloaded = $true
                break
            } catch {
                if ($attempt -eq 3) { throw }
                Start-Sleep -Seconds 2
            }
        }
        if (-not $downloaded -or -not (Test-SignedInstaller)) { throw 'Chữ ký LibreOffice không hợp lệ; chưa cài đặt.' }
    }
    $officeRuntime = Join-Path $officeBase 'runtime/LibreOffice'
    New-Item -ItemType Directory -Path $officeRuntime -Force | Out-Null
    $arguments = @('/a', ('"' + $officeMsi + '"'), '/qn', '/norestart', ('TARGETDIR="' + $officeRuntime + '"'), '/L*v', ('"' + $officeMsiLog + '"'))
    $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -in @(1603,1925)) {
        # Ask Windows for elevation; cancellation or policy refusal remains an error.
        $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $arguments -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    }
    if ($process.ExitCode -ne 0) { throw ('Giải nén LibreOffice thất bại: ' + $process.ExitCode) }
    if (-not ((Test-Office $officeProgram) -or (Test-Office $officeFlatProgram))) { throw 'Bộ cài chưa tạo đủ soffice và Python/UNO.' }
    'LibreOffice đã sẵn sàng.' | Set-Content -LiteralPath $officeLog -Encoding UTF8
    Write-Output 'Đã cài LibreOffice riêng cho PentaKuru.'
} catch {
    $message = $_.Exception.Message + "`r`nLog: " + $officeMsiLog
    $message | Set-Content -LiteralPath $officeLog -Encoding UTF8
    Write-Error $message
    exit 1
}
