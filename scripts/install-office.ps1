param([Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference = 'Stop'
$officeBase = [IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $officeBase -Force | Out-Null
$officeMsi = Join-Path $officeBase 'LibreOffice-26.8.1.msi'
$officeUri = 'https://download.documentfoundation.org/libreoffice/stable/26.8.1/win/x86_64/LibreOffice_26.8.1_Win_x86-64.msi'
Invoke-WebRequest -Uri $officeUri -OutFile $officeMsi -UseBasicParsing
$officeSignature = Get-AuthenticodeSignature -LiteralPath $officeMsi
if ($officeSignature.Status -ne 'Valid' -or $officeSignature.SignerCertificate.Subject -notmatch 'The Document Foundation') { throw 'Chữ ký bộ cài LibreOffice không hợp lệ; chưa cài đặt.' }
$officeRuntime = Join-Path $officeBase 'runtime'
New-Item -ItemType Directory -Path $officeRuntime -Force | Out-Null
$officeInstall = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/a', ('"' + $officeMsi + '"'), '/qn', ('TARGETDIR="' + $officeRuntime + '"')) -WindowStyle Hidden -Wait -PassThru
if ($officeInstall.ExitCode -ne 0) { throw ('Giải nén LibreOffice thất bại: ' + $officeInstall.ExitCode) }
$officePython = Join-Path $officeRuntime 'LibreOffice/program/python.exe'
if (-not (Test-Path -LiteralPath $officePython)) { throw 'Bộ cài chưa tạo được runtime Python/UNO cần thiết.' }
Write-Output 'Đã cài LibreOffice riêng cho PentaKuru.'
