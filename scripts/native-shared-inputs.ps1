# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later

# Select only the explicitly observed producer in this job. Do not scan older
# batches or infer dependency readiness from the outcome of a capture test.
function Read-SharedNativeArtifact([string]$ReferenceRoot, $Artifact) {
    if ($Artifact.path -notmatch '^runs/' -or $Artifact.path.Contains([char]0) -or
        $Artifact.path.Contains('\') -or $Artifact.path.Contains(':') -or
        @($Artifact.path.Split('/') | Where-Object { $_ -in @('', '.', '..') }).Count -or
        $Artifact.sha256 -cnotmatch '^[a-f0-9]{64}$') { throw 'Invalid shared native input artifact path/hash' }
    $path = [IO.Path]::GetFullPath($ReferenceRoot)
    foreach ($part in $Artifact.path.Split('/')) {
        $path = Join-Path $path $part
        $entry = Get-Item -LiteralPath $path -Force -ErrorAction Stop
        if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Shared native input path traverses a link' }
    }
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $Artifact.sha256) {
        throw 'Shared native input bytes differ from their registered identity'
    }
    return $path
}

function Get-SharedNativeSeed([string]$ReferenceRoot, $Producer) {
    $manifestPath = Read-SharedNativeArtifact $ReferenceRoot $Producer
    $run = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($run.layout_version -ne 1 -or $run.phase -ne 'P1' -or $run.phase_status -ne 'incomplete' -or
        $run.run_id -cnotmatch '^p1-print-window-windows-\d+$' -or $Producer.path -cne "runs/$($run.run_id)/manifest.json") {
        throw 'Shared native inputs require the exact PrintWindow producer from this job'
    }
    $names = @(
        'native-configuration.json', 'OpenCV-CMakeCache.txt', 'opencv-downloads.tar',
        'opencv-release.json', 'OpenCV-resolution.json', 'OpenCV-source.tar',
        'maautils-release-policy.json', 'MaaUtils-resolution.json', 'MaaUtils-source.tar',
        'ZLIB-release.json', 'ZLIB-resolution.json', 'ZLIB-source.tar',
        'PNG-public-release.html', 'PNG-release.json', 'PNG-resolution.json', 'PNG-source.tar', 'boost-release.json'
    )
    $resolved = @{}
    foreach ($name in $names) {
        $items = @($run.inputs | Where-Object { $_.path.EndsWith("/inputs/$name", [StringComparison]::Ordinal) })
        if ($items.Count -ne 1) {
            Write-Host "Shared native seed unavailable: $name is missing or ambiguous in $($run.run_id); independent components will prepare online"
            return $null
        }
        $resolved[$name] = Read-SharedNativeArtifact $ReferenceRoot $items[0]
    }
    $boost = Get-Content -LiteralPath $resolved['boost-release.json'] -Raw | ConvertFrom-Json
    $archives = @($boost.assets | Where-Object name -Match '-cmake.tar.xz$')
    if ($archives.Count -ne 1) { throw 'Shared native Boost release has no unique CMake source archive' }
    $cmake = @($run.inputs | Where-Object { $_.path.EndsWith('/inputs/cmake-preinstalled.json', [StringComparison]::Ordinal) })
    if ($cmake.Count -gt 1) { throw 'Shared preinstalled CMake identity is ambiguous' }
    if ($cmake.Count -eq 1) {
        [void](Read-SharedNativeArtifact $ReferenceRoot $cmake[0])
    } else {
        $metadata = @($run.inputs | Where-Object { $_.path.EndsWith('/inputs/cmake-release.json', [StringComparison]::Ordinal) })
        if ($metadata.Count -ne 1) { Write-Host 'Shared native seed unavailable: CMake preparation is incomplete'; return $null }
        $cmakeRelease = Get-Content -LiteralPath (Read-SharedNativeArtifact $ReferenceRoot $metadata[0]) -Raw | ConvertFrom-Json
        $cmakeArchives = @($cmakeRelease.assets | Where-Object name -Match 'windows-x86_64.zip$')
        if ($cmakeArchives.Count -ne 1) { throw 'Shared native CMake release has no unique Windows x64 archive' }
        $archives += $cmakeArchives[0]
    }
    foreach ($archive in $archives) {
        if ($archive.name -notmatch '^[A-Za-z0-9_.+-]+$') { throw 'Invalid shared native archive filename' }
        $items = @($run.inputs | Where-Object { $_.path.EndsWith('/inputs/'+$archive.name, [StringComparison]::Ordinal) })
        if ($items.Count -ne 1) { Write-Host "Shared native seed unavailable: archive $($archive.name) is missing or ambiguous"; return $null }
        [void](Read-SharedNativeArtifact $ReferenceRoot $items[0])
        if ($archive.digest -cne ('sha256:'+$items[0].sha256)) { throw 'Shared native archive differs from its recorded release asset' }
    }
    $parents = @($run.inputs | Where-Object { $_.path -cmatch '^runs/p0-windows-\d+/manifest.json$' })
    if ($parents.Count -ne 1) { Write-Host 'Shared native seed unavailable: Rust dependency batch is missing or ambiguous'; return $null }
    $rust = Get-Content -LiteralPath (Read-SharedNativeArtifact $ReferenceRoot $parents[0]) -Raw | ConvertFrom-Json
    if ($rust.layout_version -ne 1 -or -not @($rust.checks).Count -or @($rust.checks | Where-Object status -NE passed).Count) {
        Write-Host 'Shared native seed unavailable: its Rust preparation contains failures'
        return $null
    }
    foreach ($name in @('Cargo.lock', 'toolchain.txt', 'dependency-target.txt')) {
        $items = @($rust.inputs | Where-Object { $_.path.EndsWith("/inputs/$name", [StringComparison]::Ordinal) })
        if ($items.Count -ne 1) { Write-Host "Shared native seed unavailable: Rust $name is missing or ambiguous"; return $null }
        [void](Read-SharedNativeArtifact $ReferenceRoot $items[0])
    }
    if (-not @($rust.inputs | Where-Object { $_.path.Contains('/inputs/vendor/') }).Count) {
        Write-Host 'Shared native seed unavailable: archived Rust sources are missing'
        return $null
    }
    Write-Host "Shared native inputs explicitly selected: $($Producer.path); capture check outcomes remain unchanged"
    return $Producer.path
}

function Publish-SharedNativeSeed([string]$ReferenceRoot, [string[]]$Before, [string]$EnvironmentFile) {
    $index = Get-Content -LiteralPath (Join-Path $ReferenceRoot manifest.json) -Raw | ConvertFrom-Json
    $created = @($index.runs | Where-Object { $_.path -notin $Before -and $_.path -cmatch '^runs/p1-print-window-windows-\d+/manifest.json$' })
    if ($created.Count -gt 1) { throw 'Multiple PrintWindow producers appeared during one native command' }
    if ($created.Count -eq 0) { Write-Host 'No shared PrintWindow preparation was produced; later independent components will prepare online'; return }
    $selected = Get-SharedNativeSeed $ReferenceRoot $created[0]
    if ($selected) { "MFS_WINDOWS_SHARED_NATIVE_INPUTS=$selected" | Add-Content -LiteralPath $EnvironmentFile -Encoding utf8 }
}
