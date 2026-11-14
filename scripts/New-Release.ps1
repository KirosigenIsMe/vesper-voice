param(
    [string]$MingwBin = $env:VESPER_MINGW_BIN,
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
$version = [regex]::Match([IO.File]::ReadAllText((Join-Path $root 'Cargo.toml')), '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $version) { throw 'Missing semantic version in Cargo.toml' }
$stage = Join-Path $root "dist/stage/$version"
$release = Join-Path $root "dist/release/$version"
if (Test-Path -LiteralPath $stage) {
    throw "Stage already exists: $stage. Archive it before rebuilding a release."
}
New-Item -ItemType Directory -Path $stage,$release -Force | Out-Null
if (-not $SkipBuild) { & (Join-Path $root 'build.ps1') -MingwBin $MingwBin }
Copy-Item -LiteralPath (Join-Path $root 'dist/bin/Vesper.exe') -Destination (Join-Path $stage 'Vesper.exe')
if ((Get-Item -LiteralPath (Join-Path $stage 'Vesper.exe')).VersionInfo.ProductVersion -ne $version) {
    throw 'EXE version does not match Cargo.toml'
}
$vendor = & (Join-Path $PSScriptRoot 'Get-VbCable.ps1')
Copy-Item -LiteralPath $vendor -Destination (Join-Path $stage 'VirtualCable') -Recurse
Copy-Item -LiteralPath (Join-Path $root 'licenses') -Destination (Join-Path $stage 'licenses') -Recurse
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination (Join-Path $stage 'LICENSE')
# Keep the release package useful offline; the repository has one main README.
Copy-Item -LiteralPath (Join-Path $root 'README.md') -Destination (Join-Path $stage 'README.md')
$portable = Join-Path $release "Vesper-$version-Portable-x64.zip"
Compress-Archive -LiteralPath $stage -DestinationPath $portable -CompressionLevel Optimal -Force
$compiler = & (Join-Path $PSScriptRoot 'Get-InnoCompiler.ps1')
& $compiler "/DAppVersion=$version" "/DStageDir=$stage" "/DVendorDir=$vendor" "/O$release" (Join-Path $root 'installer/vesper.iss')
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
Copy-Item -LiteralPath (Join-Path $stage 'Vesper.exe') -Destination (Join-Path $release 'Vesper.exe') -Force

# A source ZIP includes the matching Rust source and installer/build scripts,
# never toolchains, local recordings, compiled dependencies or vendor binaries.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$sourceZip = Join-Path $release "Vesper-$version-Source.zip"
if (Test-Path -LiteralPath $sourceZip) { Remove-Item -LiteralPath $sourceZip }
$archive = [IO.Compression.ZipFile]::Open($sourceZip, [IO.Compression.ZipArchiveMode]::Create)
try {
    $items = @('README.md','LICENSE','Cargo.toml','Cargo.lock','rust-toolchain.toml','build.rs','build.ps1','.gitignore','.gitattributes','.cargo','.github','src','assets','licenses','installer','scripts','docs','examples')
    foreach ($item in $items) {
        $path = Join-Path $root $item
        $files = if (Test-Path -LiteralPath $path -PathType Container) { Get-ChildItem -LiteralPath $path -File -Recurse } else { @(Get-Item -LiteralPath $path) }
        foreach ($file in $files) {
            $relative = [IO.Path]::GetRelativePath($root, $file.FullName).Replace('\','/')
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $file.FullName, "vesper-voice/$relative", [IO.Compression.CompressionLevel]::Optimal) | Out-Null
        }
    }
} finally { $archive.Dispose() }
$sums = @(Get-ChildItem -LiteralPath $release -File | Where-Object Extension -In '.exe','.zip' | Sort-Object Name | ForEach-Object {
    "{0}  {1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name
})
[IO.File]::WriteAllLines((Join-Path $release 'SHA256SUMS.txt'), $sums, [Text.UTF8Encoding]::new($false))
Write-Host "Release ready: $release"
