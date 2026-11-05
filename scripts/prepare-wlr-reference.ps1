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
        scope='Fixed original Wlr/WaylandClient/MemfdBuffer screenshot context; private namespace/logging; actual MaaUtils/Boost UUID provider.'
    })
    $build=Join-Path $opencv.Native (Split-Path -Leaf $RunRoot)
    . (Join-Path $PSScriptRoot prepare-meojson-reference.ps1)
    $meojson=Prepare-MeoJsonReference $refInputs
    . (Join-Path $PSScriptRoot prepare-maautils-reference.ps1)
    $maautils=Prepare-MaaUtilsReference $refInputs -StepPrefix wlr-maautils
    . (Join-Path $PSScriptRoot prepare-boost-reference.ps1)
    $boost=Prepare-BoostReference $refInputs -StepPrefix wlr-boost
    $uuidInputs=Join-Path $refInputs original-uuid
    New-Item -ItemType Directory -Path $uuidInputs|Out-Null
    foreach($relative in @('source/Uuid/Uuid.cpp','include/MaaUtils/Uuid.h','include/MaaUtils/Conf.h','include/MaaUtils/Port.h')) {
        $saved=Join-Path $uuidInputs $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
        Copy-Item -LiteralPath (Join-Path $maautils.Source $relative) -Destination $saved;Save $saved
    }
    Step wlr-protocol-scanner wayland-scanner @('--version')
    Step wlr-reference-configure cmake @('-S',(Join-Path $productRoot tests/upstream-parity/linux-wlr),'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',"-DCMAKE_C_COMPILER:FILEPATH=$($env:CC)","-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$($opencv.Package)","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_WAYLAND_VERSION=$($wayland.version)","-DMFS_WLROOTS_PROTOCOL_ROOT=$(Join-Path $wlr.source protocol)","-DMFS_MEOJSON_INCLUDE=$($meojson.Include)","-DMFS_MAAUTILS_ROOT=$($maautils.Source)","-DMFS_BOOST_ROOT=$($boost.Source)")
    Step wlr-reference-build cmake @('--build',$build,'--parallel','4')
    $uuidProgram=Join-Path $build mfs_wlr_uuid_corpus
    $saved=Join-Path $uuidInputs mfs_wlr_uuid_corpus
    Copy-Item -LiteralPath $uuidProgram -Destination $saved;Save $saved
    $uuidCorpus=Join-Path $uuidInputs draws.txt
    try { Step original-wlr-uuid-corpus $uuidProgram @($uuidCorpus) }
    finally { if(Test-Path -LiteralPath $uuidCorpus){Save $uuidCorpus} }
    $vectorsProgram=Join-Path $build mfs_wlr_uuid_vectors
    $saved=Join-Path $uuidInputs mfs_wlr_uuid_vectors
    Copy-Item -LiteralPath $vectorsProgram -Destination $saved;Save $saved
    $vectors=Join-Path $uuidInputs vectors.tsv
    try { Step original-wlr-uuid-vectors $vectorsProgram @($vectors) }
    finally { if(Test-Path -LiteralPath $vectors){Save $vectors} }
    $env:MFS_P1_WLR_UUID_VECTORS=$vectors
    # This GNU standard-library observer is separate from the capture build.
    # Preserve its failure and continue the independent native capture probes.
    try { Step wlr-uuid-entropy-build cmake @('--build',$build,'--target','mfs_wlr_uuid_entropy','--parallel','4') }
    catch { Write-Host $_.Exception.Message }
    $entropyProgram=Join-Path $build mfs_wlr_uuid_entropy
    $entropy=Join-Path $uuidInputs default-entropy
    New-Item -ItemType Directory -Path $entropy|Out-Null
    if(Test-Path -LiteralPath $entropyProgram) {
        $saved=Join-Path $uuidInputs mfs_wlr_uuid_entropy
        Copy-Item -LiteralPath $entropyProgram -Destination $saved;Save $saved
        foreach($mode in @('real','controlled','retry')) {
            $records=Join-Path $entropy "$mode.tsv"
            try { Step ('original-wlr-uuid-entropy-'+$mode) $entropyProgram @($mode,$records) }
            catch { Write-Host $_.Exception.Message }
            finally {
                foreach($path in @($records,($records+'.failures'))) {
                    if(Test-Path -LiteralPath $path){Save $path}
                }
            }
        }
    }
    $env:MFS_P1_WLR_UUID_ENTROPY=$entropy
    $saved=Join-Path $refInputs linux-manager-source.inc
    Copy-Item -LiteralPath (Join-Path $build linux-manager-source.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-features.inc
    Copy-Item -LiteralPath (Join-Path $build linux-manager-features.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-info.inc
    Copy-Item -LiteralPath (Join-Path $build linux-manager-info.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-config.inc
    Copy-Item -LiteralPath (Join-Path $build linux-manager-config.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs native-context-scope.txt
    Copy-Item -LiteralPath (Join-Path $build native-context-scope.txt) -Destination $saved;Save $saved
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
    $native=Join-Path $build mfs_wlr_native_context
    $saved=Join-Path $refInputs mfs_wlr_native_context
    Copy-Item -LiteralPath $native -Destination $saved;Save $saved
    Step wlr-native-source-linkage ldd @($native)
    $env:MFS_P1_WLR_NATIVE_SOURCE=$native
    $whole=Join-Path $build mfs_wlr_whole_native_context
    $wholeInputs=Join-Path $refInputs whole-native-wlr
    New-Item -ItemType Directory -Path $wholeInputs|Out-Null
    foreach($item in @(@($whole,'mfs_wlr_whole_native_context'),
        @((Join-Path $build whole-native-scope.txt),'scope.txt'),
        @((Join-Path $build compile_commands.json),'compile-commands.json'))) {
        $saved=Join-Path $wholeInputs $item[1]
        Copy-Item -LiteralPath $item[0] -Destination $saved;Save $saved
    }
    foreach($relative in @('source/MaaLinuxControlUnit/API/LinuxControlUnitAPI.cpp',
        'source/MaaLinuxControlUnit/Manager/LinuxControlUnitMgr.cpp','source/MaaLinuxControlUnit/Manager/LinuxControlUnitMgr.h',
        'source/MaaLinuxControlUnit/Input/NoneInput.cpp','source/MaaLinuxControlUnit/Input/NoneInput.h',
        'source/MaaLinuxControlUnit/Base/Config.h','include/MaaControlUnit/LinuxControlUnitAPI.h',
        'include/MaaControlUnit/ControlUnitAPI.h','include/MaaControlUnit/MaaControlUnitDef.h',
        'include/MaaFramework/MaaDef.h')) {
        $saved=Join-Path $wholeInputs $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
        Copy-Item -LiteralPath (Join-Path $FixedUpstream $relative) -Destination $saved;Save $saved
    }
    foreach($identity in $identities) {
        $saved=Join-Path $wholeInputs $identity.path
        New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
        Copy-Item -LiteralPath (Join-Path $FixedUpstream $identity.path) -Destination $saved;Save $saved
    }
    Step whole-native-wlr-linkage ldd @($whole)
    $env:MFS_P1_WLR_WHOLE_SOURCE=$whole
    $declarations=Join-Path $refInputs wlr-native-abi-source
    foreach($relative in @('source/MaaLinuxControlUnit/Compat/WlRootsControlUnitCompat.cpp',
        'source/MaaLinuxControlUnit/Manager/LinuxControlUnitMgr.h','source/MaaLinuxControlUnit/Base/UnitBase.h',
        'source/MaaLinuxControlUnit/Base/Config.h','source/MaaLinuxControlUnit/Wayland/WaylandClient.h',
        'source/MaaLinuxControlUnit/Input/WlrInput.h','source/MaaLinuxControlUnit/Input/WlrInput.cpp',
        'include/MaaControlUnit/ControlUnitAPI.h','include/MaaControlUnit/WlRootsControlUnitAPI.h',
        'include/MaaControlUnit/MaaControlUnitDef.h')) {
        $saved=Join-Path $declarations $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
        Copy-Item -LiteralPath (Join-Path $FixedUpstream $relative) -Destination $saved;Save $saved
    }
    $adapter=Join-Path $build libmfs_wlr_opencv.so
    $saved=Join-Path $refInputs libmfs_wlr_opencv.so
    Copy-Item -LiteralPath $adapter -Destination $saved;Save $saved
    Step wlr-opencv-linkage ldd @($adapter)
    $protocolServer=Join-Path $build mfs_wlr_protocol_server
    $saved=Join-Path $refInputs mfs_wlr_protocol_server
    Copy-Item -LiteralPath $protocolServer -Destination $saved;Save $saved
    Step wlr-protocol-server-linkage ldd @($protocolServer)
    $serverLibrary=(& readlink -f (Join-Path $prefix lib/libwayland-server.so)|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $serverLibrary)){throw 'Prepared native Wayland server library missing'}
    $saved=Join-Path $refInputs native-libwayland-server.so
    Copy-Item -LiteralPath $serverLibrary -Destination $saved;Save $saved
    $env:MFS_P1_WLR_PROTOCOL_SERVER_LIBRARY=$serverLibrary
    foreach($pair in @(@('screencopy','wlr-screencopy-unstable-v1'),@('pointer','wlr-virtual-pointer-unstable-v1'),@('keyboard','virtual-keyboard-unstable-v1'))){
        foreach($suffix in @('h','c')){
            $saved=Join-Path $refInputs "$($pair[0])-server.$suffix"
            Copy-Item -LiteralPath (Join-Path $build "$($pair[0])-server.$suffix") -Destination $saved;Save $saved
        }
        $saved=Join-Path $refInputs "$($pair[1]).xml"
        Copy-Item -LiteralPath (Join-Path $wlr.source "protocol/$($pair[1]).xml") -Destination $saved;Save $saved
    }
    $env:MFS_P1_WLR_PROTOCOL_SERVER=$protocolServer
    $env:MFS_P1_WLR_OPENCV_ADAPTER=$adapter
    return $executable
}
