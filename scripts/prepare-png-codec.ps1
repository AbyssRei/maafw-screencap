# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit acceptance dependency preparation; never called by build.rs.
function Write-PngCodecRecord([string]$Path,[object]$Value,[scriptblock]$Register) {
    $bytes=[Text.Encoding]::UTF8.GetBytes(($Value|ConvertTo-Json -Depth 30)+"`n")
    $stream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write)
    try{$stream.Write($bytes)}finally{$stream.Dispose()}
    & $Register $Path
}
function Get-PngPublicReleaseVersion([string]$Page) {
    $plain=[Net.WebUtility]::HtmlDecode([regex]::Replace($Page,'<[^>]*>',' '))
    $matches=[regex]::Matches($plain,'The\s+current\s+public\s+release,\s+libpng\s+(\d+\.\d+\.\d+)')
    if($matches.Count -ne 1){throw 'Official libpng page has no unique current public release; no tag/cache fallback'}
    return $matches[0].Groups[1].Value
}
function Resolve-PngCodecSource([string]$Repository,[string]$Label,[string]$Directory,[string]$DepsRoot,[scriptblock]$Register,[scriptblock]$Replay,[string]$StepPrefix) {
    $releasePath=Join-Path $Directory "$Label-release.json"
    $resolutionPath=Join-Path $Directory "$Label-resolution.json"
    $archivePath=Join-Path $Directory "$Label-source.tar"
    if($Replay){
        Copy-Item -LiteralPath (& $Replay "$Label-release.json") -Destination $releasePath
        Copy-Item -LiteralPath (& $Replay "$Label-resolution.json") -Destination $resolutionPath
        Copy-Item -LiteralPath (& $Replay "$Label-source.tar") -Destination $archivePath
        $release=Get-Content -LiteralPath $releasePath -Raw|ConvertFrom-Json
        $resolution=Get-Content -LiteralPath $resolutionPath -Raw|ConvertFrom-Json
        & $Register $releasePath;& $Register $resolutionPath;& $Register $archivePath
    }elseif($Label -eq 'PNG'){
        # libpng publishes formal releases on its own site; its official GitHub
        # mirror has tags but no Releases API entry. Development tags are not
        # evidence of the current public release.
        $pagePath=Join-Path $Directory PNG-public-release.html
        Invoke-WebRequest -Uri 'https://libpng.org/pub/png/libpng.html' -OutFile $pagePath
        & $Register $pagePath
        $publicVersion=Get-PngPublicReleaseVersion (Get-Content -LiteralPath $pagePath -Raw)
        $release=[ordered]@{tag_name="v$publicVersion";draft=$false;prerelease=$false;
            registry_url='https://libpng.org/pub/png/libpng.html';queried_at=[DateTime]::UtcNow.ToString('o');
            registry_sha256=(Get-FileHash -LiteralPath $pagePath -Algorithm SHA256).Hash.ToLowerInvariant()}
        Write-PngCodecRecord $releasePath $release $Register
    }else{
        $raw=& gh api "repos/$Repository/releases/latest"
        if($LASTEXITCODE -ne 0){throw "Current formal $Label release query failed; cache is not a fallback"}
        $release=$raw|ConvertFrom-Json
        Write-PngCodecRecord $releasePath $release $Register
    }
    if($Replay -and $Label -eq 'PNG'){
        $pagePath=Join-Path $Directory PNG-public-release.html
        Copy-Item -LiteralPath (& $Replay 'PNG-public-release.html') -Destination $pagePath
        & $Register $pagePath
        if($release.registry_url -ne 'https://libpng.org/pub/png/libpng.html' -or
           (Get-FileHash -LiteralPath $pagePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $release.registry_sha256 -or
           "v$(Get-PngPublicReleaseVersion (Get-Content -LiteralPath $pagePath -Raw))" -ne $release.tag_name){
            throw 'Archived libpng formal release page identity differs'
        }
    }
    if($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^v?(\d+\.\d+\.\d+)$'){
        throw "$Label release is not a recognized formal version"
    }
    $version=[version]$Matches[1]
    if($version.Major -ne 1){throw "New $Label major requires native preparation review; no older release will be selected"}
    if($Replay){
        if($resolution.repository -ne "https://github.com/$Repository.git" -or $resolution.requested_ref -ne $release.tag_name -or
           $resolution.version -ne $version.ToString() -or $resolution.commit -notmatch '^[0-9a-f]{40}$' -or
           (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $resolution.source_archive_sha256){
            throw "Archived $Label source/release identity differs"
        }
        # Fresh replay extraction prevents an unverified source-cache hit from
        # replacing the exact archived batch. The archive remains immutable.
        $source=Join-Path $DepsRoot ("replay/$Label-"+[Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $source|Out-Null
        Step "$StepPrefix-$Label-source-replay" tar @('-xf',$archivePath,'-C',$source)
    }else{
        $remote=@(& git ls-remote --tags "https://github.com/$Repository.git" "refs/tags/$($release.tag_name)" "refs/tags/$($release.tag_name)^{}")
        if($LASTEXITCODE -ne 0 -or -not $remote.Count){throw "Formal $Label tag identity missing"}
        $peeled=@($remote|Where-Object{$_ -match '\^\{\}$'})
        $commit=(($peeled.Count ? $peeled[0] : $remote[0]) -split '\s+')[0]
        if($commit -notmatch '^[0-9a-f]{40}$'){throw "Invalid $Label tag commit"}
        $tagPath=Join-Path $Directory "$Label-tag.txt"
        $remote|Set-Content -LiteralPath $tagPath -Encoding utf8;& $Register $tagPath
        $source=Join-Path $DepsRoot "native-sources/$Label-$commit"
        if(-not(Test-Path -LiteralPath $source)){
            Step "$StepPrefix-$Label-init" git @('init','--quiet',$source)
            Step "$StepPrefix-$Label-remote" git @('-C',$source,'remote','add','origin',"https://github.com/$Repository.git")
            Step "$StepPrefix-$Label-fetch" git @('-C',$source,'fetch','--no-tags','--depth=1','origin',$commit)
            Step "$StepPrefix-$Label-checkout" git @('-C',$source,'-c','core.autocrlf=false','checkout','--quiet','--detach','FETCH_HEAD')
        }
        $actual=(& git -C $source rev-parse HEAD|Out-String).Trim()
        if($LASTEXITCODE -ne 0 -or $actual -ne $commit){throw "Cached $Label source identity differs"}
        $dirty=& git -C $source status --porcelain --untracked-files=all
        if($LASTEXITCODE -ne 0 -or $dirty){throw "Cached $Label source was modified"}
        Step "$StepPrefix-$Label-archive" git @('-C',$source,'-c','core.autocrlf=false','archive','--format=tar','--output',$archivePath,$commit)
        & $Register $archivePath
        $tree=(& git -C $source rev-parse 'HEAD^{tree}'|Out-String).Trim()
        if($LASTEXITCODE -ne 0){throw "Cannot read $Label source tree"}
        $resolution=[ordered]@{repository="https://github.com/$Repository.git";requested_ref=$release.tag_name;version=$version.ToString();commit=$commit;tree=$tree;source_archive_sha256=(Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant();queried_at=[DateTime]::UtcNow.ToString('o')}
        Write-PngCodecRecord $resolutionPath $resolution $Register
    }
    $license=Join-Path $Directory "$Label-LICENSE.txt"
    Copy-Item -LiteralPath (Join-Path $source LICENSE) -Destination $license;& $Register $license
    return [pscustomobject]@{Source=$source;Version=$version.ToString();Identity=[ordered]@{repository=$resolution.repository;ref=$resolution.requested_ref;version=$resolution.version;commit=$resolution.commit;tree=$resolution.tree;archive_sha256=$resolution.source_archive_sha256}}
}
function Prepare-PngCodec {
    param([string]$Directory,[string]$DepsRoot,[string]$CMake='cmake',[string[]]$Generator,[string]$CCompiler,
          [scriptblock]$Register,[scriptblock]$Replay=$null,[string]$StepPrefix='png-codec',[object]$ToolchainContext=$null)
    if(-not $Register -or -not $Generator.Count -or -not $CCompiler){throw 'Explicit codec recording, compiler and native generator are required'}
    $CCompiler=(Get-Command -Name $CCompiler -CommandType Application -ErrorAction Stop|Select-Object -First 1).Source
    $CMake=(Get-Command -Name $CMake -CommandType Application -ErrorAction Stop|Select-Object -First 1).Source
    $zlib=Resolve-PngCodecSource 'madler/zlib' ZLIB $Directory $DepsRoot $Register $Replay $StepPrefix
    $png=Resolve-PngCodecSource 'pnggroup/libpng' PNG $Directory $DepsRoot $Register $Replay $StepPrefix
    if((Get-Content -LiteralPath (Join-Path $zlib.Source zlib.h) -Raw) -notmatch ('#define ZLIB_VERSION "'+[regex]::Escape($zlib.Version)+'"') -or
       (Get-Content -LiteralPath (Join-Path $png.Source png.h) -Raw) -notmatch ('#define PNG_LIBPNG_VER_STRING "'+[regex]::Escape($png.Version)+'"')){
        throw 'Resolved codec headers do not match their formal release versions'
    }
    $common=@('-DCMAKE_INSTALL_LIBDIR=lib','-DCMAKE_POSITION_INDEPENDENT_CODE=ON')
    if(-not $IsWindows){$common+='-DCMAKE_BUILD_TYPE=Release'}
    if($CCompiler){$common+="-DCMAKE_C_COMPILER:FILEPATH=$CCompiler"}
    if($IsWindows){$common+='-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL'}
    if($IsMacOS){$common+=@('-DCMAKE_OSX_ARCHITECTURES=arm64','-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0',"-DCMAKE_OSX_SYSROOT:PATH=$($env:SDKROOT)")}
    $compilerIdentity=if($IsWindows){[Diagnostics.FileVersionInfo]::GetVersionInfo($CCompiler).FileVersion}else{(& $CCompiler --version|Out-String)}
    $identity=[ordered]@{zlib=$zlib.Identity;png=$png.Identity;generator=$Generator;c_compiler=$CCompiler;compiler_version=$compilerIdentity;
        source_locations=[ordered]@{zlib=$zlib.Source;png=$png.Source};
        c_compiler_sha256=(Get-FileHash -LiteralPath $CCompiler -Algorithm SHA256).Hash.ToLowerInvariant();toolchain_context=$ToolchainContext;
        cmake_sha256=(Get-FileHash -LiteralPath $CMake -Algorithm SHA256).Hash.ToLowerInvariant();
        architecture=[Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString();cmake=(& $CMake --version|Out-String);
        cflags=$env:CFLAGS;ldflags=$env:LDFLAGS;sdkroot=$env:SDKROOT;options=$common;
        linkage='static, PIC where applicable, native Release; retain PNG hardware optimization defaults'}
    $digest=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes(($identity|ConvertTo-Json -Depth 20 -Compress)))).ToLowerInvariant()
    $native=Join-Path $DepsRoot ("native/png-codec-"+$digest.Substring(0,20))
    New-Item -ItemType Directory -Path $native -Force|Out-Null
    $identityPath=Join-Path $native identity.json
    $identityJson=$identity|ConvertTo-Json -Depth 20 -Compress
    if(Test-Path -LiteralPath $identityPath){
        if((Get-Content -LiteralPath $identityPath -Raw).Trim() -ne $identityJson){throw 'PNG codec cache identity differs or key prefix collided'}
    }else{[IO.File]::WriteAllText($identityPath,$identityJson+"`n")}
    $install=Join-Path $native install
    $zbuild=Join-Path $native zlib-build
    $pbuild=Join-Path $native png-build
    Step "$StepPrefix-zlib-configure" $CMake (@('-S',$zlib.Source,'-B',$zbuild,"-DCMAKE_INSTALL_PREFIX=$install",'-DZLIB_BUILD_SHARED=OFF','-DZLIB_BUILD_STATIC=ON','-DZLIB_BUILD_TESTING=OFF')+$Generator+$common)
    Step "$StepPrefix-zlib-install" $CMake @('--build',$zbuild,'--config','Release','--target','install','--parallel','4')
    $zlibLibrary=Join-Path $install ($IsWindows ? 'lib/zs.lib' : 'lib/libz.a')
    if(-not(Test-Path -LiteralPath $zlibLibrary -PathType Leaf)){throw 'Expected exact static zlib archive missing; no system fallback'}
    $zargs=@("-DZLIB_INCLUDE_DIR:PATH=$install/include","-DZLIB_LIBRARY:FILEPATH=$zlibLibrary","-DZLIB_LIBRARY_RELEASE:FILEPATH=$zlibLibrary","-DZLIB_LIBRARY_DEBUG:FILEPATH=$zlibLibrary",'-DZLIB_USE_STATIC_LIBS=ON')
    Step "$StepPrefix-png-configure" $CMake (@('-S',$png.Source,'-B',$pbuild,"-DCMAKE_INSTALL_PREFIX=$install",'-DPNG_SHARED=OFF','-DPNG_STATIC=ON','-DPNG_FRAMEWORK=OFF','-DPNG_TESTS=OFF','-DPNG_TOOLS=OFF','-DPNG_HARDWARE_OPTIMIZATIONS=ON')+$Generator+$common+$zargs)
    Step "$StepPrefix-png-install" $CMake @('--build',$pbuild,'--config','Release','--target','install','--parallel','4')
    $pngVersion=[version]$png.Version
    $abi="$($pngVersion.Major)$($pngVersion.Minor)"
    $pngLibrary=Join-Path $install ($IsWindows ? "lib/libpng${abi}_static.lib" : "lib/libpng${abi}.a")
    if(-not(Test-Path -LiteralPath $pngLibrary -PathType Leaf)){throw 'Expected exact static libpng archive missing; no system fallback'}
    $pngArgs=@("-DPNG_PNG_INCLUDE_DIR:PATH=$install/include","-DPNG_LIBRARY:FILEPATH=$pngLibrary","-DPNG_LIBRARY_RELEASE:FILEPATH=$pngLibrary","-DPNG_LIBRARY_DEBUG:FILEPATH=$pngLibrary")
    $options=@('-DWITH_PNG=ON','-DWITH_SPNG=OFF','-DBUILD_PNG=OFF','-DBUILD_ZLIB=OFF','-DWITH_ZLIB_NG=OFF')+$pngArgs+$zargs
    $probeSource=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../tests/upstream-parity/png-codec'))
    $probeIdentity=[ordered]@{source=$probeSource;
        cmake_sha256=(Get-FileHash -LiteralPath (Join-Path $probeSource CMakeLists.txt) -Algorithm SHA256).Hash.ToLowerInvariant();
        main_sha256=(Get-FileHash -LiteralPath (Join-Path $probeSource main.c) -Algorithm SHA256).Hash.ToLowerInvariant()}
    $probeKey=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes(($probeIdentity|ConvertTo-Json -Compress)))).ToLowerInvariant()
    $probe=Join-Path $native ("probe-"+$probeKey.Substring(0,20))
    $probeCommon=@($common|Where-Object{$_ -notlike '-DCMAKE_INSTALL_LIBDIR=*'})
    Step "$StepPrefix-probe-configure" $CMake (@('-S',$probeSource,'-B',$probe)+$Generator+$probeCommon+$pngArgs+$zargs)
    Step "$StepPrefix-probe-build" $CMake @('--build',$probe,'--config','Release','--parallel','4')
    $program=Join-Path $probe ($IsWindows ? 'Release/png_codec_probe.exe' : 'png_codec_probe')
    Step "$StepPrefix-runtime" $program @($png.Version,$zlib.Version)
    foreach($entry in @(@('zlib-CMakeCache.txt',(Join-Path $zbuild CMakeCache.txt)),@('png-CMakeCache.txt',(Join-Path $pbuild CMakeCache.txt)),
        @('png-codec-probe-CMakeCache.txt',(Join-Path $probe CMakeCache.txt)),@('png-codec-probe.bin',$program))){
        $saved=Join-Path $Directory $entry[0];Copy-Item -LiteralPath $entry[1] -Destination $saved;& $Register $saved
    }
    $record=[ordered]@{identity=$identity;install=$install;png_library=$pngLibrary;zlib_library=$zlibLibrary;
        png_library_sha256=(Get-FileHash -LiteralPath $pngLibrary -Algorithm SHA256).Hash.ToLowerInvariant();zlib_library_sha256=(Get-FileHash -LiteralPath $zlibLibrary -Algorithm SHA256).Hash.ToLowerInvariant();
        probe_source=$probeIdentity;actual_probe=$program;actual_probe_sha256=(Get-FileHash -LiteralPath $program -Algorithm SHA256).Hash.ToLowerInvariant();opencv_options=$options}
    Write-PngCodecRecord (Join-Path $Directory png-codec-configuration.json) $record $Register
    return [pscustomobject]@{Identity=$identity;Options=$options;PngVersion=$png.Version;ZlibVersion=$zlib.Version;PngLibrary=$pngLibrary;ZlibLibrary=$zlibLibrary;Install=$install}
}
function Assert-OpenCvPngCodec([string]$Cache,[string]$Configuration,[object]$Codec) {
    $cacheText=Get-Content -LiteralPath $Cache -Raw
    $configText=Get-Content -LiteralPath $Configuration -Raw
    if($configText -notmatch '(?m)^#define HAVE_PNG\s*$'){throw 'Selected OpenCV has no real PNG codec'}
    foreach($entry in @(@('PNG_LIBRARY_RELEASE',$Codec.PngLibrary),@('ZLIB_LIBRARY_RELEASE',$Codec.ZlibLibrary))){
        $match=[regex]::Match($cacheText,('(?m)^'+$entry[0]+':FILEPATH=([^\r\n]+)\r?$'))
        if(-not $match.Success -or [IO.Path]::GetFullPath($match.Groups[1].Value) -ne [IO.Path]::GetFullPath($entry[1])){
            throw "OpenCV did not use the exact prepared $($entry[0]); no bundled/system fallback"
        }
    }
}
