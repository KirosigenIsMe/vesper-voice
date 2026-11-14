$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
$version = [regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
$release = Join-Path $root "dist/release/$version"
foreach ($line in Get-Content -LiteralPath (Join-Path $release 'SHA256SUMS.txt')) {
    if ($line -notmatch '^([0-9a-f]{64})  ([A-Za-z0-9_.-]+)$') { throw 'Invalid checksum manifest' }
    if ((Get-FileHash -LiteralPath (Join-Path $release $Matches[2]) -Algorithm SHA256).Hash -ne $Matches[1]) {
        throw "Checksum mismatch: $($Matches[2])"
    }
}
Add-Type -AssemblyName System.IO.Compression.FileSystem
function EntryHash($Entry) {
    if (-not $Entry) { throw 'Required ZIP entry missing' }
    $stream = $Entry.Open()
    $sha = [Security.Cryptography.SHA256]::Create()
    try { [Convert]::ToHexString($sha.ComputeHash($stream)) } finally { $stream.Dispose(); $sha.Dispose() }
}
$portable = [IO.Compression.ZipFile]::OpenRead((Join-Path $release "Vesper-$version-Portable-x64.zip"))
$source = [IO.Compression.ZipFile]::OpenRead((Join-Path $release "Vesper-$version-Source.zip"))
$vendor = [IO.Compression.ZipFile]::OpenRead((Join-Path $root '.tools/vendor/VBCABLE_Driver_Pack46.zip'))
try {
    $exe = @($portable.Entries | Where-Object FullName -Match '(^|/)Vesper\.exe$')
    if ($exe.Count -ne 1) { throw 'Expected exactly one portable executable' }
    if ((EntryHash $exe[0]) -ne (Get-FileHash -LiteralPath (Join-Path $release 'Vesper.exe')).Hash) { throw 'Portable EXE differs' }
    $prefix = $exe[0].FullName.Substring(0, $exe[0].FullName.Length - 'Vesper.exe'.Length)
    foreach ($entry in $vendor.Entries | Where-Object Name) {
        $copy = $portable.GetEntry($prefix + 'VirtualCable/' + $entry.FullName.Replace('\','/'))
        if ((EntryHash $entry) -ne (EntryHash $copy)) { throw "Vendor package modified: $($entry.FullName)" }
    }
    foreach ($name in @('README.md','LICENSE','Cargo.toml','Cargo.lock','scripts/Publish-GitHub.ps1','installer/vesper.iss','licenses/THIRD-PARTY-NOTICES.txt')) {
        if ((EntryHash ($source.GetEntry("vesper-voice/$name"))) -ne (Get-FileHash -LiteralPath (Join-Path $root $name)).Hash) { throw "Stale source ZIP: $name" }
    }
    if ($source.Entries.FullName -match '/(target|dist|\.tools|performance)/|/\.env($|\.)|\.wav$') { throw 'Local-only files found in source ZIP' }
} finally { $portable.Dispose(); $source.Dispose(); $vendor.Dispose() }
Write-Host 'Release checksums, matching source, portable EXE and original VB-CABLE files verified.'
