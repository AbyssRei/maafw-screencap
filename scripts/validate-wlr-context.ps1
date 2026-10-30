# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced only by the explicit native Linux source-context batch.
function Test-WlrProtocolContext([string]$SourceExecutable,[string]$Library,[string]$Runtime,[string]$Artifacts,[ValidateSet('registry','failures')][string]$Mode){
    $matrix=Join-Path $Artifacts $Mode
    New-Item -ItemType Directory -Path $matrix|Out-Null
    $errors=[Collections.Generic.List[string]]::new()
    if($Mode -eq 'failures') {
        $control=Join-Path $matrix disconnect-control
        New-Item -ItemType Directory -Path $control|Out-Null
        try { Step native-wlr-deferred-disconnect-control timeout @('--signal=TERM','--kill-after=5s','30s',$env:MFS_P1_WLR_PROTOCOL_SERVER,'3',$control,$env:MFS_P1_WLR_PROTOCOL_SERVER_LIBRARY,'--disconnect-control') }
        catch { $errors.Add($_.Exception.Message) }
    }
    foreach($side in @('source','rust')){
        $directory=Join-Path $matrix $side
        $producer=Join-Path $directory producer
        New-Item -ItemType Directory -Path $producer -Force|Out-Null
        $socket=Join-Path $Runtime mfs-p1-protocol
        if([Text.Encoding]::UTF8.GetByteCount($socket) -ge 108){throw 'Context fixture socket exceeds Linux sun_path capacity'}
        if(Test-Path -LiteralPath $socket){throw 'A previous native fixture still owns the context socket; no retry or removal'}
        $process=$null;$stdout=$null;$stderr=$null
        try{
            $start=[Diagnostics.ProcessStartInfo]::new($env:MFS_P1_WLR_PROTOCOL_SERVER)
            $start.UseShellExecute=$false;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
            foreach($argument in @('3',$producer,$env:MFS_P1_WLR_PROTOCOL_SERVER_LIBRARY,"--$Mode")){
                $start.ArgumentList.Add($argument)
            }
            $process=[Diagnostics.Process]::Start($start)
            $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
            $until=[DateTime]::UtcNow.AddSeconds(15)
            while(-not(Test-Path -LiteralPath $socket)){
                if($process.HasExited -or [DateTime]::UtcNow -ge $until){throw "$Mode $side fixture did not become ready"}
                Start-Sleep -Milliseconds 50
            }
            if($side -eq 'source'){
                Step "original-wlr-$Mode" timeout @('--signal=TERM','--kill-after=5s','90s',$SourceExecutable,$socket,$directory,$Library,$Mode)
            }else{
                $key=$Mode.ToUpperInvariant()
                [Environment]::SetEnvironmentVariable("MFS_P1_WLR_$($key)_SOCKET",$socket)
                [Environment]::SetEnvironmentVariable("MFS_P1_WLR_$($key)_ARTIFACTS",$directory)
                [Environment]::SetEnvironmentVariable("MFS_P1_WLR_$($key)_SOURCE",(Join-Path $matrix source))
                $test=if($Mode -eq 'registry'){
                    'registry::preserves_last_output_and_existing_snapshots_through_global_removal'
                }else{
                    'failures::preserves_failed_outputs_and_buffer_reuse_before_native_disconnect'
                }
                Step "rust-wlr-$Mode" timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr',"backends::linux::wlr::tests::$test",'--','--exact','--ignored','--test-threads=1','--nocapture')
            }
        }catch{$errors.Add($_.Exception.Message)}
        finally{
            $log=Join-Path $logsRoot "wlr-$Mode-$side-stop.log";$status='failed'
            try{
                if(-not $process){throw 'Context fixture process was not created'}
                $premature=$process.HasExited
                if(-not $premature){
                    & /bin/kill -TERM $process.Id
                    if($LASTEXITCODE -ne 0){throw 'Context fixture SIGTERM failed'}
                    if(-not $process.WaitForExit(10000)){throw 'Context fixture did not acknowledge graceful stop'}
                }
                $out=$stdout.GetAwaiter().GetResult();$err=$stderr.GetAwaiter().GetResult()
                $out|Set-Content -LiteralPath (Join-Path $directory server-stdout.log) -Encoding utf8
                $err|Set-Content -LiteralPath (Join-Path $directory server-stderr.log) -Encoding utf8
                "premature=$premature exit=$($process.ExitCode)"|Set-Content -LiteralPath $log -Encoding utf8
                $required=if($Mode -eq 'registry'){
                    'registry stopped rounds=20 globals=22 removed=21 outputs=22/22 frames=80/80 copies=80 graceful=1 error=0'
                }else{
                    'failures stopped clients=20/20 frames=80/80 copies=60 failed=40 disconnected=20 graceful=1 error=0'
                }
                $deferred= $Mode -ne 'failures' -or ($out -split '\r?\n') -ccontains 'deferred disconnect scheduled=20 executed=20 cancelled=0 pending=0'
                if($premature -or $process.ExitCode -ne 0 -or -not $out.Contains($required) -or -not $deferred){throw 'Context fixture native counters/graceful stop failed'}
                $status='passed'
            }catch{
                $_.Exception.Message|Add-Content -LiteralPath $log -Encoding utf8
                $errors.Add($_.Exception.Message)
            }finally{
                $checks.Add([ordered]@{name="wlr-$Mode-$side-stop";status=$status;log=(Artifact $log)})
                if($process){$process.Dispose()}
            }
        }
    }
    try{Step "wlr-$Mode-records" $ConfigureTool @('upstream-protocol',"wlr-$Mode",$matrix)}
    catch{$errors.Add($_.Exception.Message)}
    if($errors.Count -gt 0){throw ($errors -join "`n")}
}
