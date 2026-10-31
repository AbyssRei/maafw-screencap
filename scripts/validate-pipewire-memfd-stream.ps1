# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit child of the SourceReference batch, with its owned zero-offset producer,
# exact private route, native libraries, Cargo resolution and owned daemon.
function Invoke-PipeWireMemFdStream([string]$Root) {
    Json (Join-Path $root scope.json) ([ordered]@{
        width=28;height=24;source_stride=96;cpu_stride=84
        source='Separately allocated 4096-byte MemFd per buffer; offset zero, grow/shrink sealed, ordinary MAPPABLE transport. Producer pwrite completes before native queue.'
        intervention='Only the two isolated consumers preload the MemFd probe. Each real dequeued MemFd data pointer is temporarily hidden and restored before native queue; FD, chunk, layout, producer, MAP_BUFFERS, source code and native cleanup remain intact.'
        limits='Not a spontaneously NULL stream, DMA/GPU or Portal. Source copies/maps are unknown; Rust owns and counts maps. Source BGR and Rust RGB serials are independently verified.'
    })
    $program=Capture-TestProgram rust-pipewire-memfd-stream-build
    $saved=Join-Path $nativeArtifacts mfs-p1-pipewire-memfd-stream-test
    Copy-Item -LiteralPath $program -Destination $saved
    Save $saved
    $env:MFS_P1_PIPEWIRE_MEMFD_STREAM_ARTIFACTS=Join-Path $root rust
    try {
        try { Step original-pipewire-memfd-stream timeout @('--signal=TERM','--kill-after=5s','180s','env',"LD_PRELOAD=$env:MFS_P1_PIPEWIRE_MEMFD_PROBE_LIBRARY",$sourceExecutable,'--memfd-null',$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $root source)) }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-memfd-stream timeout @('--signal=TERM','--kill-after=5s','180s','env',"LD_PRELOAD=$env:MFS_P1_PIPEWIRE_MEMFD_PROBE_LIBRARY",$saved,'backends::linux::pipewire::native::tests::memfd_stream::maps_null_data_in_real_memfd_callbacks_and_restores_before_return','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-memfd-stream-records $ConfigureTool @('upstream-trace','pipewire-memfd-stream',$root) }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
    } finally {
        Remove-Item Env:MFS_P1_PIPEWIRE_MEMFD_STREAM_ARTIFACTS -ErrorAction SilentlyContinue
    }
}
