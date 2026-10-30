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
    . (Join-Path $PSScriptRoot prepare-meojson-reference.ps1)
    $meojson=Prepare-MeoJsonReference $refInputs
    Step macos-reference-configure cmake @('-S',(Join-Path $productRoot tests/upstream-parity/macos-screencap),'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON','-DCMAKE_OSX_ARCHITECTURES=arm64','-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0',"-DCMAKE_OSX_SYSROOT:PATH=$($env:SDKROOT)","-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DCMAKE_OBJCXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$($opencv.Package)","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_MEOJSON_INCLUDE=$($meojson.Include)")
    Step macos-reference-build cmake @('--build',$build,'--parallel','4')
    . (Join-Path $PSScriptRoot prepare-manager-metadata.ps1)
    Save-ManagerMetadataReference (Join-Path $build metadata) $refInputs (Join-Path $RunRoot macos-arm64/reference-u/artifacts/manager-metadata)
    foreach($item in @(@('CMakeCache.txt','reference-CMakeCache.txt'),@('compile_commands.json','reference-compile-commands.json'),@('overlay/Base/UnitBase.h','source-screenshot-base.h'),@('macos-manager-source.inc','macos-manager-source.inc'),@('source-context-scope.txt','source-context-scope.txt'))){
        $saved=Join-Path $refInputs $item[1]
        Copy-Item -LiteralPath (Join-Path $build $item[0]) -Destination $saved;Save $saved
    }
    $identities=@(@('Base/UnitBase.h','Screencap/ScreenCaptureKitScreencap.h','Screencap/ScreenCaptureKitScreencap.mm','Manager/MacOSControlUnitMgr.cpp','Manager/MacOSControlUnitMgr.h')|ForEach-Object{
        $path=Join-Path $FixedUpstream "source/MaaMacOSControlUnit/$_"
        [ordered]@{path=[IO.Path]::GetRelativePath($FixedUpstream,$path).Replace('\','/');sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
    $identityFile=Join-Path $refInputs original-source-identities.json
    Json $identityFile $identities;Save $identityFile
    $executable=Join-Path $build mfs_macos_source_context
    $saved=Join-Path $refInputs mfs_macos_source_context
    Copy-Item -LiteralPath $executable -Destination $saved;Save $saved
    Step macos-reference-linkage otool @('-L',$executable)
    # A byte-identical scene host at another pathname isolates host identity
    # from the observer's code/IO changes. Keep the original consumer intact.
    $script:MacosDistinctSceneExecutable=Join-Path $build mfs_macos_scene_identity_control
    [IO.File]::Copy($executable,$script:MacosDistinctSceneExecutable,$false)
    $mode=[IO.File]::GetUnixFileMode($executable)
    [IO.File]::SetUnixFileMode($script:MacosDistinctSceneExecutable,$mode)
    $sceneMode=[IO.File]::GetUnixFileMode($script:MacosDistinctSceneExecutable)
    $consumerHash=(Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
    $sceneHash=(Get-FileHash -LiteralPath $script:MacosDistinctSceneExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
    if($consumerHash -ne $sceneHash -or $mode -ne $sceneMode -or -not($mode -band [IO.UnixFileMode]::UserExecute)){
        throw 'Distinct scene executable must retain the original bytes and executable mode'
    }
    $script:MacosSceneIdentity=[ordered]@{
        layout_version=1;consumer_path=$executable;scene_path=$script:MacosDistinctSceneExecutable
        consumer_sha256=$consumerHash;scene_sha256=$sceneHash
        consumer_unix_mode=[int]$mode;scene_unix_mode=[int]$sceneMode
    }
    $sceneIdentity=Join-Path $refInputs scene-executable-identity.json
    Json $sceneIdentity $script:MacosSceneIdentity;Save $sceneIdentity
    $saved=Join-Path $refInputs mfs_macos_scene_identity_control
    Copy-Item -LiteralPath $script:MacosDistinctSceneExecutable -Destination $saved;Save $saved
    $script:MacosObservedExecutable=Join-Path $build mfs_macos_observed_context
    $saved=Join-Path $refInputs mfs_macos_observed_context
    Copy-Item -LiteralPath $script:MacosObservedExecutable -Destination $saved;Save $saved
    Step macos-dispatch-observer-linkage otool @('-L',$script:MacosObservedExecutable)
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
    $hostPremise=$Name -eq 'rust-macos-display-host-premise'
    $distinctHost=$Name -eq 'rust-macos-display-distinct-host'
    $displayPatch=$Name -in @('rust-macos-display-patch','rust-macos-display-host-premise','rust-macos-display-distinct-host')
    $originalFailure=$null
    try {
        $sceneExecutable=if($distinctHost){$script:MacosDistinctSceneExecutable}else{$Executable}
        $info=[Diagnostics.ProcessStartInfo]::new($sceneExecutable)
        $info.ArgumentList.Add(($displayPatch ? 'display-scene' : 'scene'));$info.ArgumentList.Add($scene)
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
        if($displayPatch -and $identity.display_patch -ne 1){throw 'Missing explicit first-display patch premise'}
        if($distinctHost){
            $binaryIdentity=[ordered]@{}
            foreach($key in $script:MacosSceneIdentity.Keys){$binaryIdentity[$key]=$script:MacosSceneIdentity[$key]}
            foreach($side in @('consumer','scene')){
                $path=$binaryIdentity[$side+'_path']
                if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryIdentity[$side+'_sha256'] -or
                    [int][IO.File]::GetUnixFileMode($path) -ne $binaryIdentity[$side+'_unix_mode']){throw 'Scene/consumer executable identity changed before capture'}
            }
            if($binaryIdentity.consumer_path -ne $Executable -or $binaryIdentity.scene_path -ne $info.FileName){throw 'Unexpected executable selection in distinct-host control'}
            $binaries=Join-Path $directory scene-executables
            New-Item -ItemType Directory -Path $binaries|Out-Null
            foreach($side in @('consumer','scene')){
                $saved=Join-Path $binaries $side
                [IO.File]::Copy($binaryIdentity[$side+'_path'],$saved,$false)
                if((Get-FileHash -LiteralPath $saved -Algorithm SHA256).Hash.ToLowerInvariant() -ne $binaryIdentity[$side+'_sha256']){throw 'Archived scene/consumer executable bytes changed'}
                Save $saved
            }
            $binaryIdentity.scene_pid=$server.Id
            $file=Join-Path $directory scene-executable-identity.json
            Json $file $binaryIdentity;Save $file
        }
        $env:MFS_P1_MACOS_WINDOW_ID=[string]$identity.window_id
        $env:MFS_P1_MACOS_SCENE_STOP=Join-Path $scene stop
        $env:MFS_P1_MACOS_SCENE_STOPPED=Join-Path $scene stopped.txt
        $env:MFS_P1_MACOS_RUST_ARTIFACTS=$directory
        $env:MFS_P1_MACOS_MAIN_THREAD_HOST='1'
        if($hostPremise){$env:MFS_P1_MACOS_HOST_GEOMETRY_PREMISE='1'}
        else{Remove-Item Env:MFS_P1_MACOS_HOST_GEOMETRY_PREMISE -ErrorAction SilentlyContinue}
        if($displayPatch){$env:MFS_P1_MACOS_DISPLAY_GEOMETRY=Join-Path $scene display-geometry.tsv}
        $oracleMode=if($hostPremise){'display-patch-host-premise'}elseif($displayPatch){'display-patch'}elseif($Name -eq 'rust-macos-manager'){'manager-external-window'}else{'external-window'}
        $oracleArguments=@($oracleMode,$directory,[string]$identity.window_id)
        if($hostPremise){$oracleArguments+=$env:MFS_P1_MACOS_DISPLAY_GEOMETRY}
        try {Step "$Name-original-oracle" $Executable $oracleArguments}
        catch {
            if(-not $displayPatch){throw}
            # Preserve the failed original check. An independent known-patch
            # Rust observation cannot accept an absent original oracle.
            $originalFailure=$_
        }
        if(-not $displayPatch){$env:MFS_P1_MACOS_EXTERNAL_ORACLE=Join-Path $directory external-window.bgr}
        $oracleFiles=@(Get-ChildItem -LiteralPath $directory -File|Where-Object{
            if($hostPremise){$_.Name -match '^display-(call|held)-[0-2]\.source\.bgr$' -or $_.Name -in @('source-expected-geometry.tsv','source-geometry-provenance.txt')}
            elseif($displayPatch){$_.Name -match '^display-(call|held)-[0-2]\.source\.bgr$' -or $_.Name -eq 'source-display-geometry.tsv'}
            else{$_.Name -eq 'external-window.bgr' -or $_.Name -match '^manager-cycle-[0-9]+-call-[0-9]+\.bgr$'}
        })
        $expectedOracles=if($hostPremise){8}elseif($displayPatch){7}elseif($Name -eq 'rust-macos-manager'){61}else{1}
        if($displayPatch){Json (Join-Path $directory source-oracle-status.json) ([ordered]@{
            original_oracle_passed=($null -eq $originalFailure)
            expected_source_files=$expectedOracles;recorded_source_files=$oracleFiles.Count
        })}
        if($oracleFiles.Count -ne $expectedOracles -and -not $originalFailure){throw 'Incomplete original oracle files before Rust execution'}
        $oracleRecords=@($oracleFiles|ForEach-Object{Artifact $_.FullName})
        $oracleRecord=Join-Path $directory source-oracle-identities.json
        Json $oracleRecord $oracleRecords;Save $oracleRecord
        foreach($file in $oracleFiles){Save $file.FullName}
        if($displayPatch -and -not $distinctHost){
            # A distinct executable/child directory instruments the real
            # semaphore calls. Its timing/logging is diagnostic only; it never
            # supplies or replaces the unchanged oracle's status or images.
            $observed=Join-Path $directory dispatch-observer
            New-Item -ItemType Directory -Path $observed|Out-Null
            $observedArguments=@($oracleArguments);$observedArguments[1]=$observed
            try {Step "$Name-dispatch-observer" $script:MacosObservedExecutable $observedArguments}
            catch {Write-Host $_.Exception.Message}
        }
        Step $Name cargo @('test','--release','--locked','--lib','--features','prototype-macos,macos-manager',$Test,'--','--exact','--ignored','--test-threads=1','--nocapture')
        foreach($record in $oracleRecords){
            $path=Join-Path $ReferenceRoot $record.path
            if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $record.sha256){throw 'Rust probe changed an archived original oracle'}
        }
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
    if($displayPatch){
        $recordKind=if($distinctHost){'macos-display-distinct-host'}elseif($hostPremise){'macos-display-host-premise'}else{'macos-display-patch'}
        try {Step "$recordKind-records" $ConfigureTool @('upstream-trace',$recordKind,$directory)}
        catch {if(-not $originalFailure){throw}}
        if($originalFailure){throw $originalFailure}
    }
}
