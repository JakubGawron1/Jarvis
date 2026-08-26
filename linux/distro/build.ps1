# Build Jarvis Linux on Windows via WSL2 Ubuntu (as root) + mkosi, then QEMU.
# Usage:  powershell -File linux\distro\build.ps1
param(
    [switch]$Boot
)

$ErrorActionPreference = "Stop"
$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

Write-Host "Jarvis Linux - build on Windows (WSL2 Ubuntu + mkosi + QEMU)"
wsl -d Ubuntu -u root -e true
if ($LASTEXITCODE -ne 0) {
    throw "Need WSL2 Ubuntu. Run: wsl --install -d Ubuntu"
}

$wslRepo = (wsl -d Ubuntu -u root -e wslpath -a $Repo).Trim()
$builder = "$wslRepo/linux/distro/build-in-wsl.sh"
Write-Host "Repo in WSL: $wslRepo"

$launch = @"
set -e
sed -i 's/\r`$//' '$builder'
chmod +x '$builder'
export HOME=/root USER=root LOGNAME=root
export PATH=/root/.local/bin:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
exec bash '$builder' '$wslRepo'
"@

$launch = $launch -replace "`r", ""
wsl -d Ubuntu -u root --cd /root -e bash -c $launch
if ($LASTEXITCODE -ne 0) {
    throw "WSL build failed (exit $LASTEXITCODE)"
}

if ($Boot) {
    & (Join-Path $PSScriptRoot "qemu\run-qemu.ps1")
}
