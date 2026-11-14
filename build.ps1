param(
    [string]$MingwBin = $env:VESPER_MINGW_BIN,
    [string]$OutputDirectory = 'dist/bin'
)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo/bin'
if (Test-Path -LiteralPath $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }
$env:RUSTUP_TOOLCHAIN = '1.100.0-x86_64-pc-windows-gnu'
if (-not ((rustup toolchain list) -match '^1\.100\.0-x86_64-pc-windows-gnu')) {
    rustup toolchain install $env:RUSTUP_TOOLCHAIN --profile minimal --component rustfmt,clippy
    if ($LASTEXITCODE -ne 0) { throw 'Could not install the pinned Rust GNU toolchain' }
}
if ($MingwBin) {
    $compilerDir = (Resolve-Path -LiteralPath $MingwBin).Path
    $env:PATH = "$compilerDir;$env:PATH"
} elseif (Test-Path -LiteralPath 'C:/msys64/mingw64/bin/gcc.exe') {
    $env:PATH = "C:/msys64/mingw64/bin;$env:PATH"
}
foreach ($tool in @('cargo', 'gcc', 'windres', 'ar')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Missing $tool. Install Rust GNU and MinGW-w64; see README.md."
    }
}
cargo build --release --locked --target x86_64-pc-windows-gnu
if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed' }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
Copy-Item -LiteralPath 'target/x86_64-pc-windows-gnu/release/vesper.exe' -Destination (Join-Path $OutputDirectory 'Vesper.exe') -Force
Write-Host "Built: $OutputDirectory/Vesper.exe"
