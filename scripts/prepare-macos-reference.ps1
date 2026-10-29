# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit original-source/Rust acceptance, dot-sourced after formal Xcode
# selection. Step/Artifact/Json and evidence ownership belong to the caller.
function Prepare-MacosReference([string]$FixedUpstream) {
    $refInputs=Join-Path $inputRoot source-reference
    New-Item -ItemType Directory -Path $refInputs|Out-Null
    $env:CC=(& xcrun --find clang|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $env:CC)){throw 'Selected Xcode C compiler missing'}
    $env:CXX=(& xcrun --find clang++|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $env:CXX)){throw 'Selected Xcode C++ compiler missing'}
    $pythonPackages=[ordered]@{}
    foreach($name in @('pip','ninja','cmake')){
        $path=Join-Path $refInputs "$name-release.json"
        Step "source-$name-latest" curl @('--fail','--silent','--show-error','--location','--max-time','60','-o',$path,"https://pypi.org/pypi/$name/json")
        Save $path;$pythonPackages[$name]=(Get-Content -LiteralPath $path -Raw|ConvertFrom-Json).info.version
    }
    $venv=Join-Path $DepsRoot ('python/macos-reference-'+(($pythonPackages.Values|Sort-Object)-join '-'))
    Step source-python-environment python3 @('-m','venv',$venv)
    $python=Join-Path $venv bin/python
    $report=Join-Path $refInputs python-install.json
    Step source-python-tools $python @('-m','pip','install','--upgrade','--report',$report,'pip','ninja','cmake');Save $report
    $installed=@(& $python -m pip list --format=json|ConvertFrom-Json)
    if($LASTEXITCODE -ne 0){throw 'Native source build-tool query failed'}
    foreach($name in $pythonPackages.Keys){
        if(($installed|Where-Object name -EQ $name).version -ne $pythonPackages[$name]){throw "$name differs from latest formal package release"}
    }
    $wheels=Join-Path $refInputs python-wheels
    New-Item -ItemType Directory -Path $wheels|Out-Null
    Step source-python-offline-inputs $python @('-m','pip','download','--only-binary=:all:','--dest',$wheels,'pip','ninja','cmake')
    Get-ChildItem -LiteralPath $wheels -File|ForEach-Object{Save $_.FullName}
    $env:PATH=(Join-Path $venv bin)+':'+$env:PATH
    . (Join-Path $PSScriptRoot prepare-opencv-reference.ps1)
    $opencv=Prepare-OpenCvReference $refInputs ([ordered]@{
        backend='macos-original-screen-capture-kit';target='aarch64-apple-darwin'
        developer_dir=$env:DEVELOPER_DIR;sdkroot=$env:SDKROOT
        scope='Unchanged fixed Objective-C++ source plus private initial Rust migration; original callback/5s wait; full P1 acceptance remains incomplete.'
    })
    $build=Join-Path $opencv.Native (Split-Path -Leaf $RunRoot)
    Step macos-reference-configure cmake @('-S',(Join-Path $productRoot tests/upstream-parity/macos-screencap),'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON','-DCMAKE_OSX_ARCHITECTURES=arm64','-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0',"-DCMAKE_OSX_SYSROOT:PATH=$($env:SDKROOT)","-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DCMAKE_OBJCXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$($opencv.Package)","-DMFS_UPSTREAM_ROOT=$FixedUpstream")
    Step macos-reference-build cmake @('--build',$build,'--parallel','4')
    foreach($item in @(@('CMakeCache.txt','reference-CMakeCache.txt'),@('compile_commands.json','reference-compile-commands.json'),@('overlay/Base/UnitBase.h','source-screenshot-base.h'),@('source-context-scope.txt','source-context-scope.txt'))){
        $saved=Join-Path $refInputs $item[1]
        Copy-Item -LiteralPath (Join-Path $build $item[0]) -Destination $saved;Save $saved
    }
    $identities=@(@('Base/UnitBase.h','Screencap/ScreenCaptureKitScreencap.h','Screencap/ScreenCaptureKitScreencap.mm')|ForEach-Object{
        $path=Join-Path $FixedUpstream "source/MaaMacOSControlUnit/$_"
        [ordered]@{path=[IO.Path]::GetRelativePath($FixedUpstream,$path).Replace('\','/');sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
    $identityFile=Join-Path $refInputs original-source-identities.json
    Json $identityFile $identities;Save $identityFile
    $executable=Join-Path $build mfs_macos_source_context
    $saved=Join-Path $refInputs mfs_macos_source_context
    Copy-Item -LiteralPath $executable -Destination $saved;Save $saved
    Step macos-reference-linkage otool @('-L',$executable)
    $adapter=Join-Path $build libmfs_macos_opencv.dylib
    $saved=Join-Path $refInputs libmfs_macos_opencv.dylib
    Copy-Item -LiteralPath $adapter -Destination $saved;Save $saved
    Step macos-opencv-linkage otool @('-L',$adapter)
    $env:MFS_P1_MACOS_OPENCV_ADAPTER=$adapter
    return $executable
}

function Run-RustMacosProbe([string]$Name,[string]$Test,[string]$Executable,[string]$Artifacts) {
    $directory=Join-Path $Artifacts $Name
    $scene=Join-Path $directory scene
    New-Item -ItemType Directory -Path $directory,$scene|Out-Null
    $server=$null;$serverOut=$null;$serverError=$null
    try {
        $info=[Diagnostics.ProcessStartInfo]::new($Executable)
        $info.ArgumentList.Add('scene');$info.ArgumentList.Add($scene)
        $info.UseShellExecute=$false;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
        $server=[Diagnostics.Process]::Start($info)
        $serverOut=$server.StandardOutput.ReadToEndAsync();$serverError=$server.StandardError.ReadToEndAsync()
        $ready=Join-Path $scene ready.json
        $until=[DateTime]::UtcNow.AddSeconds(20)
        while(-not(Test-Path -LiteralPath $ready)){
            if($server.HasExited -or [DateTime]::UtcNow -ge $until){throw 'Owned AppKit scene did not publish a ready identity'}
            Start-Sleep -Milliseconds 25
        }
        $identity=Get-Content -LiteralPath $ready -Raw|ConvertFrom-Json
        if($identity.owner_pid -ne $server.Id -or $identity.width -ne 47 -or $identity.height -ne 31 -or $identity.colour -ne 'green' -or $identity.window_id -le 0){throw 'Unexpected owned native scene identity'}
        $env:MFS_P1_MACOS_WINDOW_ID=[string]$identity.window_id
        $env:MFS_P1_MACOS_SCENE_STOP=Join-Path $scene stop
        $env:MFS_P1_MACOS_SCENE_STOPPED=Join-Path $scene stopped.txt
        $env:MFS_P1_MACOS_RUST_ARTIFACTS=$directory
        $env:MFS_P1_MACOS_MAIN_THREAD_HOST='1'
        Step "$Name-original-oracle" $Executable @('external-window',$directory,[string]$identity.window_id)
        $env:MFS_P1_MACOS_EXTERNAL_ORACLE=Join-Path $directory external-window.bgr
        Step $Name cargo @('test','--release','--locked','--lib','--features','prototype-macos,macos-screen-capture-kit',$Test,'--','--exact','--ignored','--test-threads=1','--nocapture')
    }catch{
        $log=Join-Path $logsRoot "$Name-probe-error.log"
        $_.Exception.ToString()|Set-Content -LiteralPath $log
        $classification=if(($server -and $server.HasExited -and $server.ExitCode -eq 77) -or ($checks.Count -gt 0 -and $checks[$checks.Count-1].status -eq 'missing_environment')){'missing_environment'}else{'failed'}
        $checks.Add([ordered]@{name="$Name-probe";status=$classification;log=(Artifact $log)})
        throw
    }finally{
        if($server){
            $log=Join-Path $logsRoot "$Name-scene-stop.log";$status='failed'
            "owned_pid=$($server.Id)"|Set-Content -LiteralPath $log
            try {
                $unexpected=$server.HasExited -and -not(Test-Path -LiteralPath (Join-Path $scene stop))
                if(-not $server.HasExited){[IO.File]::WriteAllText((Join-Path $scene stop),'stop')}
                $forced=-not $server.WaitForExit(10000)
                if($forced){$server.Kill();$server.WaitForExit()}
                $stdout=$serverOut.GetAwaiter().GetResult();$stderr=$serverError.GetAwaiter().GetResult()
                $stdout|Set-Content (Join-Path $logsRoot "$Name-scene-stdout.log")
                $stderr|Set-Content (Join-Path $logsRoot "$Name-scene-stderr.log")
                Save (Join-Path $logsRoot "$Name-scene-stdout.log");Save (Join-Path $logsRoot "$Name-scene-stderr.log")
                "owned_pid=$($server.Id)`nexit=$($server.ExitCode)`nunexpected=$unexpected`nforced=$forced"|Set-Content $log
                if(-not $unexpected -and -not $forced -and $server.ExitCode -eq 0 -and $stdout -match 'owned macOS scene stop=1 main_thread=1 passed'){$status='passed'}
                elseif($server.ExitCode -eq 77){$status='missing_environment'}
            }finally{
                $checks.Add([ordered]@{name="$Name-scene-stop";status=$status;log=(Artifact $log)})
                $server.Dispose()
            }
        }
    }
}
