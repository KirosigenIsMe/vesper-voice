$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$vendor = Join-Path $root '.tools/vendor'
New-Item -ItemType Directory -Path $vendor -Force | Out-Null
$zip = Join-Path $vendor 'VBCABLE_Driver_Pack46.zip'
if (-not (Test-Path -LiteralPath $zip)) {
    Invoke-WebRequest -Uri 'https://download.vb-audio.com/Download_CABLE/VBCABLE_Driver_Pack46.zip' -OutFile $zip
}
if ((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash -ne 'BC44E8974191A5A2427D6C979E2DAB08DBB09925F3F435DFA2F0BA3E45B6EB90') {
    throw 'VB-CABLE package checksum mismatch. Review vendor package before updating the pin.'
}
$folder = Join-Path $vendor 'VBCABLE_Driver_Pack46'
Expand-Archive -LiteralPath $zip -DestinationPath $folder -Force
$signature = Get-AuthenticodeSignature -LiteralPath (Join-Path $folder 'VBCABLE_Setup_x64.exe')
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'BUREL VINCENT') {
    throw 'VB-CABLE publisher signature could not be verified'
}
$folder
