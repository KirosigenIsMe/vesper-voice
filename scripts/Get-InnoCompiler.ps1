$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$tools = Join-Path $root '.tools'
$downloads = Join-Path $tools 'downloads'
New-Item -ItemType Directory -Path $downloads -Force | Out-Null
function Get-PinnedFile([string]$Url, [string]$Path, [string]$Sha256) {
    if (-not (Test-Path -LiteralPath $Path)) { Invoke-WebRequest -Uri $Url -OutFile $Path }
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Sha256) {
        throw "Checksum mismatch: $Path"
    }
}
$archive = Join-Path $downloads 'innosetup-6.7.4.exe'
Get-PinnedFile 'https://github.com/jrsoftware/issrc/releases/download/is-6_7_4/innosetup-6.7.4.exe' $archive 'A549929319F9120716677C685535CA4671E4B937B5FDC87E2585F32D8EB30E8C'
$signature = Get-AuthenticodeSignature -LiteralPath $archive
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Pyrsys B.V.') {
    throw 'Inno Setup publisher signature could not be verified'
}
# Extract the signed compiler package as data; do not install tooling globally.
$extractorZip = Join-Path $downloads 'innounp.zip'
Get-PinnedFile 'https://raw.githubusercontent.com/jrathlev/InnoUnpacker-Windows-GUI/75ded58da3a81c029db4f73ab803b9b4154400e9/innounp-2/bin/innounp-2.zip' $extractorZip 'F8E45C9676BEBC7D7570108AB572673B541FCF7A7D3D24A04319AC6B5315B4E8'
$extractorDir = Join-Path $tools 'innounp'
Expand-Archive -LiteralPath $extractorZip -DestinationPath $extractorDir -Force
$output = Join-Path $tools 'inno-extracted'
$compiler = Join-Path $output '{app}/ISCC.exe'
if (-not (Test-Path -LiteralPath $compiler)) {
    & (Join-Path $extractorDir 'innounp.exe') -x -b -q "-d$output" $archive | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Could not extract Inno Setup compiler' }
}
if (-not (Test-Path -LiteralPath $compiler)) { throw 'ISCC.exe missing from compiler package' }
$compiler
