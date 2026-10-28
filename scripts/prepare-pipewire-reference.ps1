# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced by prepare-pipewire.ps1. Step/Save/Json and verified native
# environment belong to that batch; this file does not run on import.
function Prepare-PipeWireReference([string]$FixedUpstream) {
    $refInputs=Join-Path $inputRoot source-reference
    New-Item -ItemType Directory -Path $refInputs | Out-Null
    Step source-system-sdk dpkg-query @('-W','-f=${binary:Package}=${Version}\n','libc6','libc6-dev','linux-libc-dev','libstdc++6','libgcc-s1')
    $releaseFile=Join-Path $refInputs opencv-release.json
    & gh api repos/opencv/opencv/releases/latest > $releaseFile
    if($LASTEXITCODE -ne 0){throw 'Official OpenCV release lookup failed; cache is not a fallback'}
    Save $releaseFile
    $cvRelease=Get-Content -LiteralPath $releaseFile -Raw|ConvertFrom-Json
    if($cvRelease.draft -or $cvRelease.prerelease -or $cvRelease.tag_name -notmatch '^\d+\.\d+\.\d+$'){throw 'OpenCV release is not a recognized formal version'}
    $remote=@(& git ls-remote --tags https://github.com/opencv/opencv.git "refs/tags/$($cvRelease.tag_name)" "refs/tags/$($cvRelease.tag_name)^{}")
    if($LASTEXITCODE -ne 0 -or $remote.Count -eq 0){throw 'OpenCV formal tag identity missing'}
    $tagFile=Join-Path $refInputs opencv-tag.txt
    $remote|Set-Content -LiteralPath $tagFile -Encoding utf8
    Save $tagFile
    $peeled=@($remote|Where-Object{$_ -match '\^\{\}$'})
    $cvCommit=(($peeled.Count -gt 0 ? $peeled[0] : $remote[0]) -split '\s+')[0]
    $cvSource=Join-Path $DepsRoot "native-sources/OpenCV-$cvCommit"
    if(-not(Test-Path -LiteralPath $cvSource)){
        Step source-opencv-clone git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$cvRelease.tag_name,'https://github.com/opencv/opencv.git',$cvSource)
    }
    $actual=(& git -C $cvSource rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $actual -ne $cvCommit){throw 'OpenCV source does not match the resolved formal tag'}
    $dirty=& git -C $cvSource status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw 'OpenCV source cache was modified'}
    $cvArchive=Join-Path $refInputs OpenCV-source.tar
    Step source-opencv-archive git @('-C',$cvSource,'archive','--format=tar','--output',$cvArchive,$cvCommit)
    Save $cvArchive

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
    $downloadRoot=Join-Path $DepsRoot downloads/opencv
    New-Item -ItemType Directory -Path $downloadRoot -Force|Out-Null
    $cvOptions=@('-DBUILD_LIST=core,imgproc','-DBUILD_SHARED_LIBS=OFF','-DBUILD_TESTS=OFF','-DBUILD_PERF_TESTS=OFF','-DBUILD_EXAMPLES=OFF','-DBUILD_opencv_apps=OFF','-DWITH_PROTOBUF=OFF','-DWITH_FLATBUFFERS=OFF','-DWITH_IPP=ON','-DBUILD_IPP_IW=ON')
    # OpenCV 5 registers optional codec export targets during global discovery,
    # even when imgcodecs is excluded; their archives are then never built.
    # This diagnostic writes raw bytes and uses only core/imgproc. Disable only
    # those unused image-I/O dependencies, retaining CPU/IPP/HAL optimization.
    $unusedCodecs=@('AVIF','JPEG','JPEGXL','WEBP','OPENJPEG','JASPER','OPENEXR','PNG','SPNG','TIFF','GDAL','GDCM')
    $cvOptions+=@($unusedCodecs|ForEach-Object{"-DWITH_$($_)=OFF"})
    $configuration=Join-Path $refInputs native-configuration.json
    Json $configuration ([ordered]@{
        opencv_release=$cvRelease.tag_name;opencv_commit=$cvCommit;drm_tag=$selected.name;drm_commit=$selected.commit.id
        pipewire_version=$env:MFS_P0_PIPEWIRE_VERSION;pipewire_library=$env:MFS_P1_PIPEWIRE_LIBRARY;cc=$env:CC;cxx=$env:CXX
        compiler=(& $env:CXX --version|Out-String);cmake=(& cmake --version|Out-String)
        options=$cvOptions;scope='Native original PipeWire screenshot class with exact extracted screenshot base, private namespace/logging overlays; no Rust native parity or full framework acceptance.'
        codec_policy='Only core/imgproc operations and raw pixel artifacts are used. Disable unused image-I/O discovery to avoid exported-but-unbuilt codec archives; do not disable CPU/IPP/HAL optimization.'
    })
    Save $configuration
    $identity=(Get-FileHash -LiteralPath $configuration).Hash.ToLowerInvariant()
    $refNative=Join-Path $DepsRoot "native/pipewire-reference-$identity"
    $cvBuild=Join-Path $refNative opencv-build
    $cvInstall=Join-Path $refNative install
    Step source-opencv-configure cmake (@('-S',$cvSource,'-B',$cvBuild,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release',"-DCMAKE_C_COMPILER:FILEPATH=$($env:CC)","-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DCMAKE_INSTALL_PREFIX=$cvInstall",'-DCMAKE_INSTALL_LIBDIR=lib',"-DOPENCV_DOWNLOAD_PATH=$downloadRoot")+$cvOptions)
    $cache=Join-Path $refInputs OpenCV-CMakeCache.txt
    Copy-Item -LiteralPath (Join-Path $cvBuild CMakeCache.txt) -Destination $cache
    Save $cache
    $cvConfig=Join-Path $refInputs opencv-cvconfig.h
    Copy-Item -LiteralPath (Join-Path $cvBuild cvconfig.h) -Destination $cvConfig
    Save $cvConfig
    if((Get-Content -LiteralPath $cvConfig -Raw) -notmatch '(?m)^#define HAVE_IPP\s*$' -or
       (Get-Content -LiteralPath $cache -Raw) -notmatch '(?m)^CV_DISABLE_OPTIMIZATION:BOOL=OFF\s*$' -or
       (Get-Content -LiteralPath $cache -Raw) -notmatch '(?m)^CPU_DISPATCH:STRING=\S+'){
        throw 'Reference OpenCV did not retain required IPP/CPU optimizations'
    }
    try { Step source-opencv-build-install cmake @('--build',$cvBuild,'--target','install','--parallel','4') }
    finally {
        $downloadArchive=Join-Path $refInputs opencv-downloads.tar
        Step source-opencv-download-archive tar @('-cf',$downloadArchive,'-C',$downloadRoot,'.')
        Save $downloadArchive
    }
    $refBuild=Join-Path $refNative (Split-Path -Leaf $RunRoot)
    $cvConfigs=@(Get-ChildItem -LiteralPath $cvInstall -File -Filter OpenCVConfig.cmake -Recurse)
    if($cvConfigs.Count -ne 1){throw 'Reference OpenCV install has no unique package configuration'}
    $cvPackage=Split-Path -Parent $cvConfigs[0].FullName
    Step pipewire-reference-configure cmake @('-S',(Join-Path $productRoot tests/upstream-parity/linux-pipewire),'-B',$refBuild,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',"-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$cvPackage","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_DRM_SOURCE=$drmSource","-DMFS_PIPEWIRE_VERSION=$($env:MFS_P0_PIPEWIRE_VERSION)")
    Step pipewire-reference-build cmake @('--build',$refBuild,'--parallel','4')
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
    # libdrm's resolved source has no root COPYING. Preserve its actual project
    # license declaration/full notice plus all canonical header notices. The
    # complete source tar above remains an independent reconstructable input.
    foreach($item in @(@((Join-Path $cvSource LICENSE),'opencv-LICENSE.txt'),@((Join-Path $drmSource meson.build),'libdrm-meson.build'),@((Join-Path $drmSource include/drm/README),'libdrm-headers-README.txt'))){
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
