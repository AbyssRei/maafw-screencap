# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Runs under the same private route, owned daemon and exact batch dependencies
# as MemFd/DMA. Each format receives a fresh producer and isolated consumers.
function Invoke-PipeWireFormatStream([string]$Root,[string]$Format) {
    $env:MFS_P1_PIPEWIRE_STREAM_FORMAT=$Format
    $env:MFS_P1_PIPEWIRE_FORMAT_STREAM_ARTIFACTS=$Root
    try {
        try { Step "original-pipewire-format-$Format" timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,'--format-stream',$Format,$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $Root source)) }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
        try { Step "rust-pipewire-format-$Format" timeout @('--signal=TERM','--kill-after=5s','180s',$formatTest,'backends::linux::pipewire::native::tests::opencv::format_stream::captures_negotiated_format_rows_and_retains_raw_and_bgr_after_shutdown','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
    } finally {
        Remove-Item Env:MFS_P1_PIPEWIRE_STREAM_FORMAT -ErrorAction SilentlyContinue
        Remove-Item Env:MFS_P1_PIPEWIRE_FORMAT_STREAM_ARTIFACTS -ErrorAction SilentlyContinue
    }
}
