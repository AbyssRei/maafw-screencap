# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$ArchivePath,
    [Parameter(Mandatory)][string]$RestoreRoot
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.Formats.Tar
$source = (Resolve-Path -LiteralPath $ReferenceRoot).ProviderPath
$archive = [IO.Path]::GetFullPath($ArchivePath)
$restored = [IO.Path]::GetFullPath($RestoreRoot)
$comparison = if ($IsWindows) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }
$prefix = $source.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
foreach ($output in @($archive, $restored)) {
    if ($output.Equals($source, $comparison) -or $output.StartsWith($prefix, $comparison)) {
        throw 'Evidence archive and restored copy must be outside the source reference tree'
    }
    if (Test-Path -LiteralPath $output) { throw "Refusing to overwrite evidence output: $output" }
}
New-Item -ItemType Directory -Path (Split-Path -Parent $archive) -Force | Out-Null
Write-Host "Evidence writer: $([System.Formats.Tar.TarWriter].Assembly.FullName)"
$stream = [IO.File]::Open($archive, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
$writer = [System.Formats.Tar.TarWriter]::new($stream, [System.Formats.Tar.TarEntryFormat]::Pax, $true)
$count = 0
try {
    # Explicit enumeration is essential: shortcut/native traversal omitted
    # hidden Git contents on tested Windows environments. Do not follow links.
    foreach ($entry in Get-ChildItem -LiteralPath $source -Force -Recurse) {
        $relative = [IO.Path]::GetRelativePath($source, $entry.FullName).Replace('\', '/')
        $writer.WriteEntry($entry.FullName, $relative)
        $count++
    }
}
finally {
    $writer.Dispose()
    $stream.Dispose()
}
Write-Host "Archived $count entries, including hidden and empty directories"
New-Item -ItemType Directory -Path $restored | Out-Null
[System.Formats.Tar.TarFile]::ExtractToDirectory($archive, $restored, $false)
# The verifier checks the restored Git objects/worktree and every registered
# input/log hash. A readable tar alone is not sufficient evidence.
& cargo xtask reference verify --root $restored
if ($LASTEXITCODE -ne 0) { throw 'Restored evidence verification failed' }
Write-Host "Evidence SHA-256: $((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant())"
