# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit original-source acceptance only, dot-sourced after formal Xcode
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
        scope='Unchanged fixed Objective-C++ source; original callback/5s wait; no Rust backend or full P1 acceptance.'
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
    return $executable
}
