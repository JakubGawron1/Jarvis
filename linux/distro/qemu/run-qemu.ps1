# Boot Jarvis Linux (Arch + Plasma) in QEMU on Windows.
# Build first:  powershell -File linux\distro\build.ps1
$ErrorActionPreference = "Stop"
$Here = $PSScriptRoot
$Disk = Join-Path $Here "jarvis-linux.qcow2"
$WslDisk = "\\wsl.localhost\Ubuntu\home\jakub\jarvis-distro\jarvis-linux.qcow2"
$Ovmf = "C:\Program Files\qemu\share\edk2-x86_64-code.fd"
$Qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"

if (-not (Test-Path $Disk)) {
    if (Test-Path $WslDisk) {
        $Disk = $WslDisk
        Write-Host "Using WSL image: $Disk"
    } else {
        throw "No jarvis-linux.qcow2. Run linux\distro\build.ps1 first."
    }
}
if (-not (Test-Path $Qemu)) { throw "QEMU not found at $Qemu" }
if (-not (Test-Path $Ovmf)) { throw "UEFI firmware missing: $Ovmf" }

Write-Host "Booting $Disk. Login jarvis / jarvis"
& $Qemu `
    -machine q35,accel=whpx:tcg `
    -m 4096 -smp 4 `
    -drive "if=pflash,format=raw,readonly=on,file=$Ovmf" `
    -drive "file=$Disk,if=virtio,format=qcow2" `
    -device virtio-vga `
    -display gtk `
    -netdev "user,id=n0,hostfwd=tcp::7420-:7420" `
    -device virtio-net-pci,netdev=n0 `
    -device qemu-xhci -device usb-tablet `
    -name "Jarvis Linux"
