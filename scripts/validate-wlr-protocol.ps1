# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced by the explicit native Linux source-context orchestration only.
function Test-WlrProtocolMatrix([string]$SourceExecutable,[string]$Library,[string]$Runtime,[string]$Artifacts){
    $matrix=Join-Path $Artifacts protocol
    New-Item -ItemType Directory -Path $matrix|Out-Null
    $errors=[Collections.Generic.List[string]]::new()
    foreach($version in 1..3){
        $directory=Join-Path $matrix "v$version"
        $producer=Join-Path $directory producer
        New-Item -ItemType Directory -Path $producer -Force|Out-Null
        $socket=Join-Path $Runtime mfs-p1-protocol
        if([Text.Encoding]::UTF8.GetByteCount($socket) -ge 108){throw 'Protocol fixture socket exceeds Linux sun_path capacity'}
        if(Test-Path -LiteralPath $socket){throw 'A prior protocol fixture still owns the socket; no retry or removal'}
        $process=$null;$stdout=$null;$stderr=$null
        try{
            $start=[Diagnostics.ProcessStartInfo]::new($env:MFS_P1_WLR_PROTOCOL_SERVER)
            $start.UseShellExecute=$false;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
            $start.ArgumentList.Add([string]$version);$start.ArgumentList.Add($producer)
            $start.ArgumentList.Add($env:MFS_P1_WLR_PROTOCOL_SERVER_LIBRARY)
            $process=[Diagnostics.Process]::Start($start)
            $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
            $until=[DateTime]::UtcNow.AddSeconds(15)
            while(-not(Test-Path -LiteralPath $socket)){
                if($process.HasExited -or [DateTime]::UtcNow -ge $until){throw "Protocol v$version fixture did not become ready"}
                Start-Sleep -Milliseconds 50
            }
            try{Step "original-wlr-protocol-v$version" timeout @('--signal=TERM','--kill-after=5s','90s',$SourceExecutable,$socket,$directory,$Library,"protocol-$version")}
            catch{$errors.Add($_.Exception.Message)}
            $env:MFS_P1_WLR_PROTOCOL_SOCKET=$socket
            $env:MFS_P1_WLR_PROTOCOL_VERSION=[string]$version
            $env:MFS_P1_WLR_PROTOCOL_ARTIFACTS=$directory
            try{Step "rust-wlr-protocol-v$version" timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::protocol::matches_native_protocol_formats_padding_and_ignored_yinvert','--','--exact','--ignored','--test-threads=1','--nocapture')}
            catch{$errors.Add($_.Exception.Message)}
        }catch{$errors.Add($_.Exception.Message)}
        finally{
            $log=Join-Path $logsRoot "wlr-protocol-v$version-stop.log";$status='failed'
            try{
                if(-not $process){throw 'Protocol fixture process was not created'}
                $premature=$process.HasExited
                if(-not $premature){
                    & /bin/kill -TERM $process.Id
                    if($LASTEXITCODE -ne 0){throw 'Protocol fixture SIGTERM failed'}
                    if(-not $process.WaitForExit(10000)){throw 'Protocol fixture did not acknowledge graceful stop'}
                }
                $out=$stdout.GetAwaiter().GetResult();$err=$stderr.GetAwaiter().GetResult()
                $out|Set-Content -LiteralPath (Join-Path $directory server-stdout.log) -Encoding utf8
                $err|Set-Content -LiteralPath (Join-Path $directory server-stderr.log) -Encoding utf8
                "premature=$premature exit=$($process.ExitCode)"|Set-Content -LiteralPath $log -Encoding utf8
                $required="protocol stopped version=$version managers=40/40 frames=960/960 copies=960 graceful=1 error=0"
                if($premature -or $process.ExitCode -ne 0 -or -not $out.Contains($required)){throw 'Protocol fixture native counters/graceful stop failed'}
                $status='passed'
            }catch{
                $_.Exception.Message|Add-Content -LiteralPath $log -Encoding utf8
                $errors.Add($_.Exception.Message)
            }finally{
                $checks.Add([ordered]@{name="wlr-protocol-v$version-stop";status=$status;log=(Artifact $log)})
                if($process){$process.Dispose()}
            }
        }
    }
    try{Step wlr-protocol-records $ConfigureTool @('upstream-protocol','wlr',$matrix)}
    catch{$errors.Add($_.Exception.Message)}
    if($errors.Count -gt 0){throw ($errors -join "`n")}
}
