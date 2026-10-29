# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced only by the explicit native Linux source-context batch.
function Test-WlrRegistry([string]$SourceExecutable,[string]$Library,[string]$Runtime,[string]$Artifacts){
    $matrix=Join-Path $Artifacts registry
    New-Item -ItemType Directory -Path $matrix|Out-Null
    $errors=[Collections.Generic.List[string]]::new()
    foreach($side in @('source','rust')){
        $directory=Join-Path $matrix $side
        $producer=Join-Path $directory producer
        New-Item -ItemType Directory -Path $producer -Force|Out-Null
        $socket=Join-Path $Runtime mfs-p1-protocol
        if([Text.Encoding]::UTF8.GetByteCount($socket) -ge 108){throw 'Registry fixture socket exceeds Linux sun_path capacity'}
        if(Test-Path -LiteralPath $socket){throw 'A previous native fixture still owns the registry socket; no retry or removal'}
        $process=$null;$stdout=$null;$stderr=$null
        try{
            $start=[Diagnostics.ProcessStartInfo]::new($env:MFS_P1_WLR_PROTOCOL_SERVER)
            $start.UseShellExecute=$false;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
            foreach($argument in @('3',$producer,$env:MFS_P1_WLR_PROTOCOL_SERVER_LIBRARY,'--registry')){
                $start.ArgumentList.Add($argument)
            }
            $process=[Diagnostics.Process]::Start($start)
            $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
            $until=[DateTime]::UtcNow.AddSeconds(15)
            while(-not(Test-Path -LiteralPath $socket)){
                if($process.HasExited -or [DateTime]::UtcNow -ge $until){throw "Registry $side fixture did not become ready"}
                Start-Sleep -Milliseconds 50
            }
            if($side -eq 'source'){
                Step original-wlr-registry timeout @('--signal=TERM','--kill-after=5s','90s',$SourceExecutable,$socket,$directory,$Library,'registry')
            }else{
                $env:MFS_P1_WLR_REGISTRY_SOCKET=$socket
                $env:MFS_P1_WLR_REGISTRY_ARTIFACTS=$directory
                $env:MFS_P1_WLR_REGISTRY_SOURCE=Join-Path $matrix source
                Step rust-wlr-registry timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::registry::preserves_last_output_and_existing_snapshots_through_global_removal','--','--exact','--ignored','--test-threads=1','--nocapture')
            }
        }catch{$errors.Add($_.Exception.Message)}
        finally{
            $log=Join-Path $logsRoot "wlr-registry-$side-stop.log";$status='failed'
            try{
                if(-not $process){throw 'Registry fixture process was not created'}
                $premature=$process.HasExited
                if(-not $premature){
                    & /bin/kill -TERM $process.Id
                    if($LASTEXITCODE -ne 0){throw 'Registry fixture SIGTERM failed'}
                    if(-not $process.WaitForExit(10000)){throw 'Registry fixture did not acknowledge graceful stop'}
                }
                $out=$stdout.GetAwaiter().GetResult();$err=$stderr.GetAwaiter().GetResult()
                $out|Set-Content -LiteralPath (Join-Path $directory server-stdout.log) -Encoding utf8
                $err|Set-Content -LiteralPath (Join-Path $directory server-stderr.log) -Encoding utf8
                "premature=$premature exit=$($process.ExitCode)"|Set-Content -LiteralPath $log -Encoding utf8
                $required='registry stopped rounds=20 globals=22 removed=21 outputs=22/22 frames=80/80 copies=80 graceful=1 error=0'
                if($premature -or $process.ExitCode -ne 0 -or -not $out.Contains($required)){throw 'Registry fixture native counters/graceful stop failed'}
                $status='passed'
            }catch{
                $_.Exception.Message|Add-Content -LiteralPath $log -Encoding utf8
                $errors.Add($_.Exception.Message)
            }finally{
                $checks.Add([ordered]@{name="wlr-registry-$side-stop";status=$status;log=(Artifact $log)})
                if($process){$process.Dispose()}
            }
        }
    }
    try{Step wlr-registry-records $ConfigureTool @('upstream-protocol','wlr-registry',$matrix)}
    catch{$errors.Add($_.Exception.Message)}
    if($errors.Count -gt 0){throw ($errors -join "`n")}
}
