# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit child of prepare-pipewire.ps1's SourceReference batch. All native
# versions, compiler, Cargo resolution and owned daemon come from that batch.
function Invoke-PipeWireDmaStream {
    $dmaStreamRoot=Join-Path $referenceArtifacts dma-stream
    New-Item -ItemType Directory -Path $dmaStreamRoot | Out-Null
    # Compile independently of device availability. Missing kernel access must
    # not conceal whether the producer itself builds with selected real headers.
    $program=Join-Path $native mfs-p1-pipewire-dma-source
    $dmaLibraryDirectory=Split-Path -Parent $env:MFS_P1_PIPEWIRE_DMA_FIXTURE_LIBRARY
    Step native-dma-source-build $env:CC @('-std=c11','-D_GNU_SOURCE','-O2','-Wall','-Wextra','-Werror',"-I$(Join-Path $prefix include/pipewire-0.3)","-I$(Join-Path $prefix include/spa-0.2)","-I$(Join-Path $productRoot tests/upstream-parity/linux-pipewire)",(Join-Path $productRoot tests/fixtures/pipewire-dma-source.c),$env:MFS_P1_PIPEWIRE_DMA_FIXTURE_LIBRARY,"-Wl,-rpath,$dmaLibraryDirectory","-L$(Join-Path $prefix lib)",'-lpipewire-0.3','-pthread','-fuse-ld=lld','-o',$program)
    $savedProgram=Join-Path $nativeArtifacts mfs-p1-pipewire-dma-source
    Copy-Item -LiteralPath $program -Destination $savedProgram
    Save $savedProgram
    $exporterFile=Join-Path $dmaDirectory exporter.txt
    $exporter=if(Test-Path -LiteralPath $exporterFile){(Get-Content -LiteralPath $exporterFile -Raw).Trim()}else{'0'}
    if($exporter -notmatch '^[12]$'){
        $missing=Join-Path $logsRoot pipewire-dma-stream-environment.log
        'missing_environment: no verified kernel DMA exporter; no incoming DMA stream ran'|Set-Content -LiteralPath $missing -Encoding utf8
        $checks.Add([ordered]@{name='pipewire-dma-stream-environment';status='missing_environment';log=(Artifact $missing)})
        throw 'Incoming DMA stream needs a verified real kernel exporter'
    }
    $dmaProducer=$null;$dmaProducerOut=$null;$dmaProducerError=$null
    $dmaRouter=$null;$dmaRouterOut=$null;$dmaRouterError=$null
    $oldNode=$env:MFS_P0_PIPEWIRE_NODE_ID
    try {
        # Native socket teardown precedes the daemon's registry notification.
        # Establish producer removal before publishing a new named fixture;
        # this waits for environment setup only, never retries a capture.
        $registry=Join-Path $dmaStreamRoot private-registry-before-start.json
        $until=[DateTime]::UtcNow.AddSeconds(10)
        do {
            & timeout --signal=TERM --kill-after=5s 10s pw-dump | Set-Content -LiteralPath $registry -Encoding utf8
            if($LASTEXITCODE -ne 0){throw 'DMA preflight registry query failed'}
            $oldSources=@(Get-Content -LiteralPath $registry -Raw|ConvertFrom-Json|Where-Object{ $_.type -eq 'PipeWire:Interface:Node' -and $_.info.props.'node.name' -eq 'mfs-p0-fixture' })
            if($oldSources.Count){Start-Sleep -Milliseconds 20}
        }while($oldSources.Count -and [DateTime]::UtcNow -lt $until)
        if($oldSources.Count){throw 'Ordinary fixture still registered after its process shutdown'}
        $start=[Diagnostics.ProcessStartInfo]::new($program)
        $start.UseShellExecute=$false
        $start.RedirectStandardOutput=$true
        $start.RedirectStandardError=$true
        $start.ArgumentList.Add($exporter)
        $dmaProducer=[Diagnostics.Process]::Start($start)
        $dmaProducerOut=$dmaProducer.StandardOutput.ReadToEndAsync()
        $dmaProducerError=$dmaProducer.StandardError.ReadToEndAsync()
        $registry=Join-Path $dmaStreamRoot private-registry.json
        $until=[DateTime]::UtcNow.AddSeconds(15)
        do {
            if($dmaProducer.HasExited -or $server.HasExited){throw 'DMA producer/owned daemon exited during registration'}
            & timeout --signal=TERM --kill-after=5s 10s pw-dump | Set-Content -LiteralPath $registry -Encoding utf8
            if($LASTEXITCODE -ne 0){throw 'DMA private registry query failed'}
            $nodes=@(Get-Content -LiteralPath $registry -Raw|ConvertFrom-Json|Where-Object{ $_.type -eq 'PipeWire:Interface:Node' -and $_.info.props.'node.name' -eq 'mfs-p0-fixture' })
            if($nodes.Count -eq 0){Start-Sleep -Milliseconds 50}
        }while($nodes.Count -eq 0 -and [DateTime]::UtcNow -lt $until)
        if($nodes.Count -ne 1){throw 'Expected exactly the separately owned DMA producer'}
        $env:MFS_P0_PIPEWIRE_NODE_ID=[string]$nodes[0].id
        $ready=Join-Path $dmaStreamRoot route-ready.txt
        $start=[Diagnostics.ProcessStartInfo]::new($routeBinary)
        $start.UseShellExecute=$false
        $start.RedirectStandardOutput=$true
        $start.RedirectStandardError=$true
        $start.ArgumentList.Add($env:MFS_P0_PIPEWIRE_NODE_ID)
        $start.ArgumentList.Add($ready)
        $dmaRouter=[Diagnostics.Process]::Start($start)
        $dmaRouterOut=$dmaRouter.StandardOutput.ReadToEndAsync()
        $dmaRouterError=$dmaRouter.StandardError.ReadToEndAsync()
        $until=[DateTime]::UtcNow.AddSeconds(10)
        while(-not(Test-Path -LiteralPath $ready)){
            if($dmaRouter.HasExited -or [DateTime]::UtcNow -ge $until){throw 'Owned DMA route supervisor did not become ready'}
            Start-Sleep -Milliseconds 20
        }
        Json (Join-Path $dmaStreamRoot geometry.json) ([ordered]@{width=28;height=24;source_stride=96;cpu_stride=84;exporter=[int]$exporter;source_format='RGB';source_output='BGR';rust_output='RGB';scope='Real CPU-written kernel DMA buffers without MAPPABLE; no GPU or Portal. Source/Rust capture different serials and each must match known pixels; source callback pointer itself is not instrumented.'})
        try { Step original-pipewire-dma-stream timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $dmaStreamRoot source)) }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
        $env:MFS_P1_PIPEWIRE_DMA_STREAM_ARTIFACTS=Join-Path $dmaStreamRoot rust
        try { Step rust-pipewire-dma-stream timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::captures_null_data_dma_stream_with_paired_sync_and_retained_cpu_samples','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $script:failed=$true; Write-Host $_.Exception.Message }
    }finally{
        StopOwned dma-route-supervisor $dmaRouter $dmaRouterOut $dmaRouterError
        StopOwned dma-source $dmaProducer $dmaProducerOut $dmaProducerError
        $env:MFS_P0_PIPEWIRE_NODE_ID=$oldNode
        Remove-Item Env:MFS_P1_PIPEWIRE_DMA_STREAM_ARTIFACTS -ErrorAction SilentlyContinue
    }
}
