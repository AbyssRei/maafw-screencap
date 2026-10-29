# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced only by the explicit native Wlr SourceReference batch.
function Prepare-WlrReference([string]$FixedUpstream) {
    $refInputs=Join-Path $inputRoot source-reference
    New-Item -ItemType Directory -Path $refInputs | Out-Null
    Step source-system-sdk dpkg-query @('-W','-f=${binary:Package}=${Version}\n','libc6','libc6-dev','linux-libc-dev','libstdc++6','libgcc-s1')
    . (Join-Path $PSScriptRoot prepare-opencv-reference.ps1)
    $opencv=Prepare-OpenCvReference $refInputs ([ordered]@{
        backend='wlr';wayland_version=$wayland.version;wayland_library=$library
        scope='Fixed original Wlr/WaylandClient/MemfdBuffer screenshot context; private namespace/logging and external random-name provider.'
    })
    $build=Join-Path $opencv.Native (Split-Path -Leaf $RunRoot)
    Step wlr-reference-configure cmake @('-S',(Join-Path $productRoot tests/upstream-parity/linux-wlr),'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',"-DCMAKE_C_COMPILER:FILEPATH=$($env:CC)","-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$($opencv.Package)","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_WAYLAND_VERSION=$($wayland.version)")
    Step wlr-reference-build cmake @('--build',$build,'--parallel','4')
    foreach($item in @(@('CMakeCache.txt','reference-CMakeCache.txt'),@('compile_commands.json','reference-compile-commands.json'),@('overlay/Base/UnitBase.h','source-screenshot-base.h'),@('source-context-scope.txt','source-context-scope.txt'))){
        $saved=Join-Path $refInputs $item[1]
        Copy-Item -LiteralPath (Join-Path $build $item[0]) -Destination $saved;Save $saved
    }
    $identities=@(Get-ChildItem -LiteralPath (Join-Path $FixedUpstream source/MaaLinuxControlUnit) -File -Recurse|Where-Object{
        $_.Name -in @('WlrScreencap.cpp','WlrScreencap.h','WaylandClient.cpp','WaylandClient.h','WaylandHelper.cpp','WaylandHelper.h','MemfdBuffer.cpp','MemfdBuffer.h','UnitBase.h','wlr-screencopy.h','wlr-screencopy-priv.c','wlr-virtual-pointer.h','wlr-virtual-pointer-priv.c','virtual-keyboard.h','virtual-keyboard-priv.c')
    }|ForEach-Object{[ordered]@{path=[IO.Path]::GetRelativePath($FixedUpstream,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
    $identityFile=Join-Path $refInputs original-source-identities.json
    Json $identityFile $identities;Save $identityFile
    $executable=Join-Path $build mfs_wlr_source_context
    $saved=Join-Path $refInputs mfs_wlr_source_context
    Copy-Item -LiteralPath $executable -Destination $saved;Save $saved
    Step wlr-reference-linkage ldd @($executable)
    $adapter=Join-Path $build libmfs_wlr_opencv.so
    $saved=Join-Path $refInputs libmfs_wlr_opencv.so
    Copy-Item -LiteralPath $adapter -Destination $saved;Save $saved
    Step wlr-opencv-linkage ldd @($adapter)
    $env:MFS_P1_WLR_OPENCV_ADAPTER=$adapter
    return $executable
}
