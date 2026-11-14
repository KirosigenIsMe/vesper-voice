$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location -LiteralPath $root
try {
    $metadataText = cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-gnu
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $ids = @($metadata.resolve.nodes.id)
    $licenseTexts = [Collections.Generic.Dictionary[string,object]]::new()
    $text = [Text.StringBuilder]::new()
    [void]$text.AppendLine('VESPER - third-party Rust software notices')
    [void]$text.AppendLine('Fonts: Noto Sans / Mono / Symbols 2, SIL OFL 1.1: FONT-OFL.txt.')
    [void]$text.AppendLine('VB-CABLE is separate VB-Audio donationware: VB-CABLE-NOTICE.txt and VirtualCable/readme.txt.')
    foreach ($package in ($metadata.packages | Where-Object { $_.name -ne 'vesper' -and $_.id -in $ids } | Sort-Object name,version)) {
        [void]$text.AppendLine("`n========================================================================")
        [void]$text.AppendLine("$($package.name) $($package.version) - $($package.license)")
        [void]$text.AppendLine($package.repository)
        $dir = Split-Path $package.manifest_path -Parent
        $files = @(Get-ChildItem -LiteralPath $dir -File | Where-Object Name -Match '^(LICEN[SC]E|COPYING|NOTICE)([-_.].*)?$')
        if ($package.license_file) {
            $declared = Join-Path $dir $package.license_file
            if ((Test-Path -LiteralPath $declared) -and $declared -notin $files.FullName) { $files += Get-Item -LiteralPath $declared }
        }
        foreach ($file in $files) {
            $content = [IO.File]::ReadAllText($file.FullName)
            $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($content)))
            if (-not $licenseTexts.ContainsKey($hash)) {
                $licenseTexts[$hash] = [pscustomobject]@{ Id = ('L{0:D3}' -f ($licenseTexts.Count + 1)); Text = $content }
            }
            [void]$text.AppendLine("$($file.Name): see $($licenseTexts[$hash].Id) below.")
        }
    }
    [void]$text.AppendLine("`nFULL LICENSE AND NOTICE TEXTS (identical copies stored once)")
    foreach ($license in ($licenseTexts.Values | Sort-Object Id)) {
        [void]$text.AppendLine("`n========================================================================")
        [void]$text.AppendLine($license.Id)
        [void]$text.AppendLine($license.Text)
    }
    [IO.File]::WriteAllText((Join-Path $root 'licenses/THIRD-PARTY-NOTICES.txt'), $text.ToString(), [Text.UTF8Encoding]::new($false))
} finally { Pop-Location }
