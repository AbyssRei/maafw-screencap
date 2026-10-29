# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit native acceptance preparation only. Caller's Step/Save/Json own
# records; no capture crate or product OpenCV dependency is introduced.
function Prepare-OpenCvReference([string]$Directory,[System.Collections.IDictionary]$Context) {
    New-Item -ItemType Directory -Path $Directory -Force|Out-Null
    $releaseFile=Join-Path $Directory opencv-release.json
    & gh api repos/opencv/opencv/releases/latest > $releaseFile
    if($LASTEXITCODE -ne 0){throw 'Official OpenCV release lookup failed; cache is not a fallback'}
    Save $releaseFile
    $release=Get-Content -LiteralPath $releaseFile -Raw|ConvertFrom-Json
    if($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^\d+\.\d+\.\d+$'){throw 'OpenCV release is not a recognized formal version'}
    $remote=@(& git ls-remote --tags https://github.com/opencv/opencv.git "refs/tags/$($release.tag_name)" "refs/tags/$($release.tag_name)^{}")
    if($LASTEXITCODE -ne 0 -or $remote.Count -eq 0){throw 'OpenCV formal tag identity missing'}
    $tagFile=Join-Path $Directory opencv-tag.txt
    $remote|Set-Content -LiteralPath $tagFile -Encoding utf8;Save $tagFile
    $peeled=@($remote|Where-Object{$_ -match '\^\{\}$'})
    $commit=(($peeled.Count -gt 0 ? $peeled[0] : $remote[0]) -split '\s+')[0]
    $source=Join-Path $DepsRoot "native-sources/OpenCV-$commit"
    if(-not(Test-Path -LiteralPath $source)){
        Step source-opencv-clone git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$release.tag_name,'https://github.com/opencv/opencv.git',$source)
    }
    $actual=(& git -C $source rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $actual -ne $commit){throw 'OpenCV source does not match the resolved formal tag'}
    $dirty=& git -C $source status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw 'OpenCV source cache was modified'}
    $archive=Join-Path $Directory OpenCV-source.tar
    Step source-opencv-archive git @('-C',$source,'archive','--format=tar','--output',$archive,$commit);Save $archive
    $license=Join-Path $Directory opencv-LICENSE.txt
    Copy-Item -LiteralPath (Join-Path $source LICENSE) -Destination $license;Save $license
    $downloads=Join-Path $DepsRoot downloads/opencv
    New-Item -ItemType Directory -Path $downloads -Force|Out-Null
    $appleArm=$IsMacOS -and [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -eq 'Arm64'
    $options=@('-DBUILD_LIST=core,imgproc','-DBUILD_SHARED_LIBS=OFF','-DBUILD_TESTS=OFF','-DBUILD_PERF_TESTS=OFF','-DBUILD_EXAMPLES=OFF','-DBUILD_opencv_apps=OFF','-DWITH_PROTOBUF=OFF','-DWITH_FLATBUFFERS=OFF')
    if($appleArm){
        # IPP is Intel-only. Retain OpenCV's native arm64 NEON baseline and
        # supported dispatch defaults, verified by the real native consumer.
        $options+=@('-DWITH_IPP=OFF','-DBUILD_IPP_IW=OFF')
    }else{
        $options+=@('-DWITH_IPP=ON','-DBUILD_IPP_IW=ON')
    }
    # Excluded image-I/O modules must not export unbuilt codec archives.
    $unusedCodecs=@('AVIF','JPEG','JPEGXL','WEBP','OPENJPEG','JASPER','OPENEXR','PNG','SPNG','TIFF','GDAL','GDCM')
    $options+=@($unusedCodecs|ForEach-Object{"-DWITH_$($_)=OFF"})
    $options+='-DENABLE_PIC=ON'
    $identityRecord=[ordered]@{
        opencv_release=$release.tag_name;opencv_commit=$commit;cc=$env:CC;cxx=$env:CXX
        architecture=[Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
        compiler=(& $env:CXX --version|Out-String);cmake=(& cmake --version|Out-String)
        cflags=$env:CFLAGS;cxxflags=$env:CXXFLAGS;ldflags=$env:LDFLAGS;options=$options
        discovery=[ordered]@{
            pkg_config_path=$env:PKG_CONFIG_PATH;pkg_config_libdir=$env:PKG_CONFIG_LIBDIR
            cmake_prefix_path=$env:CMAKE_PREFIX_PATH;cpath=$env:CPATH
            c_include_path=$env:C_INCLUDE_PATH;cxx_include_path=$env:CPLUS_INCLUDE_PATH
            library_path=$env:LIBRARY_PATH;sdkroot=$env:SDKROOT
        }
    }
    $configuration=Join-Path $Directory native-configuration.json
    Json $configuration ([ordered]@{
        opencv=$identityRecord;context=$Context
        codec_policy=($appleArm ? 'Core/imgproc; unused codec discovery disabled; native NEON/CPU/HAL optimization retained, Intel IPP inapplicable.' : 'Core/imgproc and raw pixel artifacts only; unused codec discovery disabled; CPU/IPP/HAL optimization retained.')
    });Save $configuration
    # Backend context does not alter the OpenCV compilation. Archive it per run
    # but share the exact compiler/source/options cache after fresh resolution.
    $identity=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes(($identityRecord|ConvertTo-Json -Depth 20 -Compress)))).ToLowerInvariant()
    $native=Join-Path $DepsRoot "native/opencv-reference-$identity"
    $build=Join-Path $native opencv-build;$install=Join-Path $native install
    Step source-opencv-configure cmake (@('-S',$source,'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release',"-DCMAKE_C_COMPILER:FILEPATH=$($env:CC)","-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DCMAKE_INSTALL_PREFIX=$install",'-DCMAKE_INSTALL_LIBDIR=lib',"-DOPENCV_DOWNLOAD_PATH=$downloads")+$options)
    $cache=Join-Path $Directory OpenCV-CMakeCache.txt
    Copy-Item -LiteralPath (Join-Path $build CMakeCache.txt) -Destination $cache;Save $cache
    $config=Join-Path $Directory opencv-cvconfig.h
    Copy-Item -LiteralPath (Join-Path $build cvconfig.h) -Destination $config;Save $config
    if((-not $appleArm -and (Get-Content -LiteralPath $config -Raw) -notmatch '(?m)^#define HAVE_IPP\s*$') -or
       (Get-Content -LiteralPath $cache -Raw) -notmatch '(?m)^CV_DISABLE_OPTIMIZATION:BOOL=OFF\s*$' -or
       (Get-Content -LiteralPath $cache -Raw) -notmatch '(?m)^CPU_DISPATCH:STRING=\S+'){
        throw 'Reference OpenCV did not retain required IPP/CPU optimizations'
    }
    try { Step source-opencv-build-install cmake @('--build',$build,'--target','install','--parallel','4') }
    finally {
        $downloadArchive=Join-Path $Directory opencv-downloads.tar
        Step source-opencv-download-archive tar @('-cf',$downloadArchive,'-C',$downloads,'.');Save $downloadArchive
    }
    $packages=@(Get-ChildItem -LiteralPath $install -File -Filter OpenCVConfig.cmake -Recurse)
    if($packages.Count -ne 1){throw 'Reference OpenCV install has no unique package configuration'}
    return [pscustomobject]@{Native=$native;Package=(Split-Path -Parent $packages[0].FullName)}
}
