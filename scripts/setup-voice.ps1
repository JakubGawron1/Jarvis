# Download Piper TTS + PL/EN voices into vendor/piper (Windows).
# Usage: powershell -File scripts\setup-voice.ps1
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Dest = Join-Path $Root "vendor\piper"
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

$zip = Join-Path $env:TEMP "piper-windows-amd64.zip"
$piperUrl = "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip"
Write-Host "Piper → $Dest"
Invoke-WebRequest -Uri $piperUrl -OutFile $zip -UseBasicParsing
Expand-Archive -Path $zip -DestinationPath $Dest -Force
Remove-Item $zip -Force

$voices = Join-Path $Dest "voices"
New-Item -ItemType Directory -Force -Path $voices | Out-Null
$hf = "https://huggingface.co/rhasspy/piper-voices/resolve/main"
$files = @(
    @{ Rel = "pl/pl_PL/darkman/medium/pl_PL-darkman-medium.onnx"; Name = "pl_PL-darkman-medium.onnx" },
    @{ Rel = "pl/pl_PL/darkman/medium/pl_PL-darkman-medium.onnx.json"; Name = "pl_PL-darkman-medium.onnx.json" },
    @{ Rel = "en/en_GB/alan/medium/en_GB-alan-medium.onnx"; Name = "en_GB-alan-medium.onnx" },
    @{ Rel = "en/en_GB/alan/medium/en_GB-alan-medium.onnx.json"; Name = "en_GB-alan-medium.onnx.json" }
)
foreach ($f in $files) {
    $out = Join-Path $voices $f.Name
    Write-Host "voice $($f.Name)"
    Invoke-WebRequest -Uri "$hf/$($f.Rel)" -OutFile $out -UseBasicParsing
}

$exe = Get-ChildItem -Path $Dest -Recurse -Filter piper.exe | Select-Object -First 1
if (-not $exe) { throw "piper.exe missing after extract" }
Write-Host "OK  $($exe.FullName)"
Write-Host "Set in .env (optional — daemon auto-discovers vendor/piper):"
Write-Host "  PIPER_BIN=$($exe.FullName)"
Write-Host "  PIPER_VOICE_PL=$voices\pl_PL-darkman-medium.onnx"
Write-Host "  PIPER_VOICE_EN=$voices\en_GB-alan-medium.onnx"
Write-Host "Restart jarvisd. Without Piper, Windows SAPI still speaks."
