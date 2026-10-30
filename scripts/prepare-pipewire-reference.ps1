# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced by prepare-pipewire.ps1. Step/Save/Json and verified native
# environment belong to that batch; this file does not run on import.
function Prepare-PipeWireReference([string]$FixedUpstream) {
    $refInputs=Join-Path $inputRoot source-reference
    New-Item -ItemType Directory -Path $refInputs | Out-Null
    Step source-system-sdk dpkg-query @('-W','-f=${binary:Package}=${Version}\n','libc6','libc6-dev','linux-libc-dev','libstdc++6','libgcc-s1')
    # Only native DRM declarations are needed, but resolve/record the latest
    # formal source rather than substituting an older installed header.
    $drmTags=[Collections.Generic.List[object]]::new()
    $page=1
    do {
        $tagPath=Join-Path $refInputs "drm-tags-$page.json"
        Step "source-drm-tags-$page" curl @('--fail','--silent','--show-error','--location','--connect-timeout','15','--max-time','60','-o',$tagPath,"https://gitlab.freedesktop.org/api/v4/projects/mesa%2Fdrm/repository/tags?per_page=100&page=$page")
        Save $tagPath
        $items=@(Get-Content -LiteralPath $tagPath -Raw|ConvertFrom-Json)
        foreach($item in $items){$drmTags.Add($item)}
        $page++
    }while($items.Count -eq 100)
    $selected=$drmTags|Where-Object name -Match '^libdrm-(\d+\.\d+\.\d+)$'|Sort-Object{[version]([regex]::Match($_.name,'^libdrm-(\d+\.\d+\.\d+)$').Groups[1].Value)} -Descending|Select-Object -First 1
    if(-not $selected){throw 'No recognized formal libdrm tag'}
    $drmSource=Join-Path $DepsRoot "native-sources/libdrm-$($selected.commit.id)"
    if(-not(Test-Path -LiteralPath $drmSource)){
        Step source-drm-clone git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$selected.name,'https://gitlab.freedesktop.org/mesa/drm.git',$drmSource)
    }
    $actual=(& git -C $drmSource rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $actual -ne $selected.commit.id){throw 'DRM header source differs from the formal tag'}
    $dirty=& git -C $drmSource status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw 'DRM source cache was modified'}
    $drmArchive=Join-Path $refInputs libdrm-source.tar
    Step source-drm-archive git @('-C',$drmSource,'archive','--format=tar','--output',$drmArchive,$actual)
    Save $drmArchive
    . (Join-Path $PSScriptRoot prepare-opencv-reference.ps1)
    $opencv=Prepare-OpenCvReference $refInputs ([ordered]@{
        backend='pipewire';drm_tag=$selected.name;drm_commit=$selected.commit.id
        pipewire_version=$env:MFS_P0_PIPEWIRE_VERSION;pipewire_library=$env:MFS_P1_PIPEWIRE_LIBRARY
        scope='Original-source and private OpenCV acceptance only; no product dependency.'
    })
    $refBuild=Join-Path $opencv.Native (Split-Path -Leaf $RunRoot)
    $cvPackage=$opencv.Package
    . (Join-Path $PSScriptRoot prepare-meojson-reference.ps1)
    $meojson=Prepare-MeoJsonReference $refInputs
    Step pipewire-reference-configure cmake @('-S',(Join-Path $productRoot tests/upstream-parity/linux-pipewire),'-B',$refBuild,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',"-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$cvPackage","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_DRM_SOURCE=$drmSource","-DMFS_PIPEWIRE_VERSION=$($env:MFS_P0_PIPEWIRE_VERSION)","-DMFS_MEOJSON_INCLUDE=$($meojson.Include)")
    Step pipewire-reference-build cmake @('--build',$refBuild,'--parallel','4')
    Observe-LinuxConfigReference $refBuild $refInputs $FixedUpstream
    $manager=Join-Path $refBuild linux-manager/mfs_linux_manager_model
    $saved=Join-Path $refInputs mfs_linux_manager_model
    Copy-Item -LiteralPath $manager -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-source.inc
    Copy-Item -LiteralPath (Join-Path $refBuild linux-manager/linux-manager-source.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-features.inc
    Copy-Item -LiteralPath (Join-Path $refBuild linux-manager/linux-manager-features.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-info.inc
    Copy-Item -LiteralPath (Join-Path $refBuild linux-manager/linux-manager-info.inc) -Destination $saved;Save $saved
    $saved=Join-Path $refInputs linux-manager-config.inc
    Copy-Item -LiteralPath (Join-Path $refBuild linux-manager/linux-manager-config.inc) -Destination $saved;Save $saved
    Step linux-manager-model-linkage ldd @($manager)
    $env:MFS_P1_LINUX_MANAGER_MODEL=$manager
    foreach($item in @(@('CMakeCache.txt','reference-CMakeCache.txt'),@('compile_commands.json','reference-compile-commands.json'),@('overlay/Base/UnitBase.h','source-screenshot-base.h'),@('source-context-scope.txt','source-context-scope.txt'))){
        $saved=Join-Path $refInputs $item[1]
        Copy-Item -LiteralPath (Join-Path $refBuild $item[0]) -Destination $saved
        Save $saved
    }
    $executable=Join-Path $refBuild mfs_pipewire_source_context
    $saved=Join-Path $refInputs mfs_pipewire_source_context
    Copy-Item -LiteralPath $executable -Destination $saved
    Save $saved
    Step pipewire-reference-linkage ldd @($executable)
    $kwin=Join-Path $refBuild mfs_pipewire_kwin_native_context
    $saved=Join-Path $refInputs mfs_pipewire_kwin_native_context
    Copy-Item -LiteralPath $kwin -Destination $saved;Save $saved
    Step kwin-native-source-linkage ldd @($kwin)
    $env:MFS_P1_KWIN_NATIVE_SOURCE=$kwin
    $kwinDeclarations=Join-Path $refInputs kwin-native-abi-source
    foreach($relative in @('source/MaaLinuxControlUnit/Compat/KWinControlUnitCompat.cpp',
        'source/MaaLinuxControlUnit/Manager/LinuxControlUnitMgr.h','source/MaaLinuxControlUnit/Base/UnitBase.h',
        'source/MaaLinuxControlUnit/Base/Config.h','include/MaaControlUnit/ControlUnitAPI.h',
        'include/MaaControlUnit/KWinControlUnitAPI.h','include/MaaControlUnit/MaaControlUnitDef.h',
        'include/MaaToolkit/Portal/MaaToolkitPortal.h')) {
        $saved=Join-Path $kwinDeclarations $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
        Copy-Item -LiteralPath (Join-Path $FixedUpstream $relative) -Destination $saved;Save $saved
    }
    $adapter=Join-Path $refBuild libmfs_pipewire_opencv.so
    $saved=Join-Path $refInputs libmfs_pipewire_opencv.so
    Copy-Item -LiteralPath $adapter -Destination $saved
    Save $saved
    Step pipewire-opencv-adapter-linkage ldd @($adapter)
    $env:MFS_P1_PIPEWIRE_OPENCV_ADAPTER=$adapter
    $fault=Join-Path $refBuild libmfs_pipewire_fd_fault.so
    if($fault -match '[:\s]'){throw 'The explicit FD fault fixture path cannot be encoded in LD_PRELOAD'}
    $saved=Join-Path $refInputs libmfs_pipewire_fd_fault.so
    Copy-Item -LiteralPath $fault -Destination $saved
    Save $saved
    Step pipewire-fd-fault-linkage ldd @($fault)
    $env:MFS_P1_PIPEWIRE_FD_FAULT_LIBRARY=$fault
    $dma=Join-Path $refBuild libmfs_pipewire_dma_fixture.so
    $saved=Join-Path $refInputs libmfs_pipewire_dma_fixture.so
    Copy-Item -LiteralPath $dma -Destination $saved
    Save $saved
    Step pipewire-dma-fixture-linkage ldd @($dma)
    $env:MFS_P1_PIPEWIRE_DMA_FIXTURE_LIBRARY=$dma
    # libdrm's resolved source has no root COPYING. Preserve its actual project
    # license declaration/full notice plus all canonical header notices. The
    # complete source tar above remains an independent reconstructable input.
    foreach($item in @(@((Join-Path $drmSource meson.build),'libdrm-meson.build'),@((Join-Path $drmSource include/drm/README),'libdrm-headers-README.txt'))){
        $saved=Join-Path $refInputs $item[1]
        Copy-Item -LiteralPath $item[0] -Destination $saved
        Save $saved
    }
    $headerNotices=Join-Path $refInputs libdrm-header-notices.tar
    Step source-drm-header-notices tar @('-cf',$headerNotices,'-C',(Join-Path $drmSource include/drm),'.')
    Save $headerNotices
    $copying=Join-Path $drmSource COPYING
    $hasCopying=Test-Path -LiteralPath $copying -PathType Leaf
    if($hasCopying){
        $saved=Join-Path $refInputs libdrm-COPYING
        Copy-Item -LiteralPath $copying -Destination $saved
        Save $saved
    }
    $licenseScope=Join-Path $refInputs libdrm-license-scope.txt
    "Root COPYING present=$hasCopying. Preserve actual meson.build project license declaration/notice, complete canonical header notices including drm_fourcc.h/drm.h, header README and full source. Only native declarations are consumed; this diagnostic does not link or distribute libdrm runtime. Exact license identities remain in those archived sources; do not infer a blanket license from a missing filename."|Set-Content -LiteralPath $licenseScope -Encoding utf8
    Save $licenseScope
    return $executable
}
