# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$DepsRoot,
    [Parameter(Mandatory)][string]$RunRoot,
    [Parameter(Mandatory)][string]$ConfigureTool,
    [Parameter(Mandatory)][string]$RustBatch,
    [switch]$SourceReference,
    [string]$UpstreamRoot=''
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference=$false
if($SourceReference -and (-not $UpstreamRoot -or -not(Test-Path -LiteralPath $UpstreamRoot -PathType Container))){throw 'Wlr source context needs its verified upstream directory'}
$productRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $productRoot
$inputRoot=Join-Path $RunRoot inputs
$logsRoot=Join-Path $RunRoot linux-x64-wlr/logs
New-Item -ItemType Directory -Path $logsRoot -Force|Out-Null
$checks=[Collections.Generic.List[object]]::new()
$inputs=[Collections.Generic.List[object]]::new()
$libraries=[Collections.Generic.List[object]]::new()
$server=$null; $serverOut=$null; $serverError=$null; $failed=$false
$referenceArtifacts=$null

function Artifact([string]$Path){
    $relative=[IO.Path]::GetRelativePath($ReferenceRoot,$Path).Replace('\','/')
    if($relative.StartsWith('../') -or [IO.Path]::IsPathRooted($relative)){throw 'Evidence escaped reference'}
    return [ordered]@{path=$relative;sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
function Save([string]$Path){$inputs.Add((Artifact $Path))}
function Json([string]$Path,$Value){
    $stream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write)
    try{$stream.Write([Text.Encoding]::UTF8.GetBytes(($Value|ConvertTo-Json -Depth 50)+"`n"))}finally{$stream.Dispose()}
}
function Step([string]$Name,[string]$Program,[string[]]$Arguments){
    Write-Host "Running $Name ..."
    $log=Join-Path $logsRoot "$Name.log"; $status='failed'
    try{
        "program=$Program`narguments=$($Arguments|ConvertTo-Json -Compress)"|Set-Content -LiteralPath $log -Encoding utf8
        & $Program @Arguments >> $log 2>&1
        $code=$LASTEXITCODE
        "exit=$code"|Add-Content -LiteralPath $log -Encoding utf8
        if($code -ne 0){Get-Content -LiteralPath $log -Tail 45|ForEach-Object{Write-Host $_};throw "$Name exited with $code; see $log"}
        if($Name -eq 'original-wlr-source-context' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original Wlr source cycles=20 captures=60 retained=60 recursive_roundtrip=1 resources_balanced=1 passed'){
            throw 'Expected twenty actual original Wlr lifecycles, recursive roundtrip and sixty independently retained images'
        }
        $sourceSummary=switch($Name){
            'original-wlr-shared-client' {'original Wlr shared_client cycles=20 captures=60 caller_released=20 first_source_released=20 last_client_released=20 retained=60 resources_balanced=1 passed'}
            'original-wlr-reconnect' {'original Wlr reconnect cycles=20 same_path_noop=20 close_reopen=20 changed_path=20 missing_path=20 subsequent_connect=20 captures=20 resources_balanced=1 passed'}
            'original-wlr-fd-zero' {'original MemfdBuffer fd_zero=1 available=1 destructor_fd_retained=1 destructor_mapping_retained=1 fixture_reclaimed=1 product_cleanup_success=0 passed'}
            'original-wlr-manager' {'original Wlr manager cycles=20 captures=60 connects=40 retained=60 same_client_reconnect=20 failed_client_retained=1 failed_output_preserved=1 resources_balanced=1 passed'}
            'original-wlr-nul-names' {'original Wlr nul_names cycles=20 captures=60 retained=60 same_full_name_noop=20 suffix_reconnect=20 leading_nul_rejected=20 empty_rejected=20 manager_full_uuid=20 resources_balanced=1 passed'}
            default {$null}
        }
        if($sourceSummary -and -not (Get-Content -LiteralPath $log -Raw).Contains($sourceSummary)){
            throw 'Expected the complete original shared-client/reconnect/FD-zero context summary'
        }
        if($Name -eq 'rust-wlr-migration-capture'){
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust Wlr cycles=20 captures=60 retained_raw=60 retained_bgr=60 frame_conversion_lifetimes=60 overwritten_maps=60 resources_balanced=1 passed' -or
               $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected twenty native Wlr capture lifecycles and sixty owning raw/BGR images through conversion and shutdown'
            }
        }
        if($Name -eq 'rust-wlr-conversion-exception'){
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Wlr conversion_exception=1 frame_released=1 subsequent_capture=1 native_owners_closed=1 passed' -or
               $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected the native Wlr/OpenCV exception, frame release and distinct subsequent capture'
            }
        }
        if($Name -eq 'rust-wlr-layout-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
            throw 'Expected one checked Wlr layout/conversion test, separate from native capture'
        }
        if($Name -eq 'rust-wlr-fd-zero'){
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or
               -not $observed.Contains('native Rust Wlr fd_zero_cycles=20 captures=20 retained_raw=20 retained_bgr=20 source_descriptor_mapping_retained=20 fixture_reclaimed=20 product_cleanup_success=0 passed')){
                throw 'Expected twenty actual FD-zero lifecycles with observed source residuals and separately reclaimed fixture resources'
            }
        }
        $rustContextSummary=switch($Name){
            'rust-wlr-shared-client' {'native Rust Wlr shared_client cycles=20 captures=60 caller_released=20 first_source_released=20 last_client_released=20 retained_raw=60 retained_bgr=60 independent_buffers=20 resources_balanced=1 passed'}
            'rust-wlr-reconnect' {'native Rust Wlr reconnect cycles=20 same_path_noop=20 close_reopen=20 changed_path=20 missing_path=20 same_missing_retry=20 subsequent_connect=20 captures=20 retained_raw=20 retained_bgr=20 resources_balanced=1 passed'}
            'rust-wlr-manager' {'native Rust Wlr manager cycles=20 captures=60 connects=40 retained_raw=60 retained_bgr=60 same_client_reconnect=20 failed_client_retained=1 failed_output_preserved=1 resources_balanced=1 passed'}
            'rust-wlr-nul-names' {'native Rust Wlr nul_names cycles=20 captures=60 retained_raw=60 retained_bgr=60 same_full_name_noop=20 suffix_reconnect=20 leading_nul_rejected=20 empty_rejected=20 json_manager_full_uuid=20 resources_balanced=1 passed'}
            default {$null}
        }
        if($rustContextSummary){
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or -not $observed.Contains($rustContextSummary)){
                throw 'Expected the complete native Rust shared-client/reconnect test and retained raw/BGR images'
            }
        }
        if($Name -eq 'rust-wlr-manager-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
            throw 'Expected one actual native None/unimplemented-manager control'
        }
        $status='passed'
    }finally{$checks.Add([ordered]@{name=$Name;status=$status;log=(Artifact $log)});Write-Host "${Name}: $status"}
}
function GitSource([string]$Name,[string]$Project,[string]$Repository,[string]$Pattern){
    $tags=[Collections.Generic.List[object]]::new();$page=1
    do{
        $path=Join-Path $inputRoot "$Name-tags-$page.json"
        Step "$Name-tags-$page" curl @('--fail','--silent','--show-error','--location','--connect-timeout','15','--max-time','60','-o',$path,"https://gitlab.freedesktop.org/api/v4/projects/$Project/repository/tags?per_page=100&page=$page")
        Save $path
        $items=@(Get-Content -LiteralPath $path -Raw|ConvertFrom-Json)
        foreach($item in $items){$tags.Add($item)}
        $page++
    }while($items.Count -eq 100)
    $selected=$tags|Where-Object name -Match $Pattern|Sort-Object{[version]([regex]::Match($_.name,$Pattern).Groups[1].Value)} -Descending|Select-Object -First 1
    if(-not $selected){throw "$Name has no recognized formal tag"}
    $version=[regex]::Match($selected.name,$Pattern).Groups[1].Value
    $source=Join-Path $DepsRoot "native-sources/$Name-$($selected.commit.id)"
    if(-not(Test-Path -LiteralPath $source)){
        Step "$Name-clone" git @('-c','core.autocrlf=false','-c','advice.detachedHead=false','clone','--depth=1','--branch',$selected.name,$Repository,$source)
    }
    $head=(& git -C $source rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $head -ne $selected.commit.id){throw "$Name cache differs from the resolved formal tag"}
    $dirty=& git -C $source status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw "$Name cache contains modified/untracked source"}
    $archive=Join-Path $inputRoot "$Name-source.tar"
    Step "$Name-source-archive" git @('-C',$source,'archive','--format=tar','--output',$archive,$head)
    Save $archive
    $record=[ordered]@{name=$Name;version=$version;tag=$selected.name;commit=$head;repository=$Repository;source=$source}
    $libraries.Add($record);return $record
}
function GithubRelease([string]$Name,[string]$Repository,[string]$Pattern){
    $path=Join-Path $inputRoot "$Name-releases.json"
    $log=Join-Path $logsRoot "$Name-releases.log";$status='failed'
    try{
        "gh api --paginate --slurp repos/$Repository/releases?per_page=100"|Set-Content -LiteralPath $log -Encoding utf8
        & gh api --paginate --slurp "repos/$Repository/releases?per_page=100" > $path 2>> $log
        if($LASTEXITCODE -ne 0){throw "$Name official release query failed"}
        $status='passed'
    }finally{$checks.Add([ordered]@{name="$Name-releases";status=$status;log=(Artifact $log)})}
    Save $path
    $releases=@(Get-Content -LiteralPath $path -Raw|ConvertFrom-Json|ForEach-Object{foreach($item in $_){$item}})
    $selected=$releases|Where-Object{-not $_.draft -and -not $_.prerelease -and $_.tag_name -match $Pattern}|Sort-Object{[version]([regex]::Match($_.tag_name,$Pattern).Groups[1].Value.Replace('_','.'))} -Descending|Select-Object -First 1
    if(-not $selected){throw "$Name has no recognized formal release (tag and metadata both checked)"}
    return $selected
}
function GithubGitSource([string]$Name,[string]$Repository,[string]$Pattern){
    $selected=GithubRelease $Name $Repository $Pattern
    $version=[regex]::Match($selected.tag_name,$Pattern).Groups[1].Value
    $remote=@(& git ls-remote --tags "https://github.com/$Repository.git" "refs/tags/$($selected.tag_name)" "refs/tags/$($selected.tag_name)^{}")
    if($LASTEXITCODE -ne 0 -or $remote.Count -eq 0){throw "$Name remote formal tag identity missing"}
    $remotePath=Join-Path $inputRoot "$Name-remote-tag.txt";$remote|Set-Content -LiteralPath $remotePath -Encoding utf8;Save $remotePath
    $peeled=@($remote|Where-Object{$_ -match '\^\{\}$'})
    $expected=(($peeled.Count -gt 0 ? $peeled[0] : $remote[0]) -split '\s+')[0]
    $source=Join-Path $DepsRoot "native-sources/$Name-$expected"
    if(-not(Test-Path -LiteralPath $source)){Step "$Name-clone" git @('-c','core.autocrlf=false','-c','advice.detachedHead=false','clone','--depth=1','--branch',$selected.tag_name,"https://github.com/$Repository.git",$source)}
    $head=(& git -C $source rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0){throw "$Name source identity query failed"}
    $tag=(& git -C $source rev-parse "$($selected.tag_name)^{commit}"|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $tag -ne $head -or $head -ne $expected){throw "$Name cached source differs from the current formal release tag"}
    $dirty=& git -C $source status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw "$Name source cache was modified"}
    $archive=Join-Path $inputRoot "$Name-source.tar"
    Step "$Name-source-archive" git @('-C',$source,'archive','--format=tar','--output',$archive,$head)
    Save $archive
    $record=[ordered]@{name=$Name;version=$version;tag=$selected.tag_name;commit=$head;repository=$Repository;source=$source}
    $libraries.Add($record);return $record
}
function AssetSource([string]$Name,[string]$Repository,[string]$Pattern){
    $selected=GithubRelease $Name $Repository $Pattern
    $version=[regex]::Match($selected.tag_name,$Pattern).Groups[1].Value.Replace('_','.')
    $assets=@($selected.assets|Where-Object{$_.name -match "^$Name-[0-9.]+\.tar\.gz$"})
    if($assets.Count -ne 1){throw "$Name source asset is ambiguous"}
    $asset=$assets[0];$archive=Join-Path $inputRoot $asset.name
    Step "$Name-download" curl @('--fail','--silent','--show-error','--location','--connect-timeout','15','--max-time','120','-o',$archive,$asset.browser_download_url)
    Save $archive
    $hash=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if($asset.digest -and $asset.digest -ne "sha256:$hash"){throw "$Name asset differs from the official SHA-256"}
    # Release archives contain generated configure files. Expand anew for each
    # run, preserving the authoritative archive; avoid a mutable source cache.
    $source=Join-Path $DepsRoot "expanded/$(Split-Path -Leaf $RunRoot)/$Name"
    New-Item -ItemType Directory -Path $source -Force|Out-Null
    Step "$Name-unpack" tar @('-xf',$archive,'--strip-components=1','-C',$source)
    $record=[ordered]@{name=$Name;version=$version;tag=$selected.tag_name;sha256=$hash;repository=$Repository;source=$source}
    $libraries.Add($record);return $record
}
function MesonBuild($Library,[string[]]$Options){
    $name=$Library.name;$build=Join-Path $native "$name-build"
    $setupArguments=@('setup',$build,$Library.source,"--prefix=$prefix",'--libdir=lib','--buildtype=release','--wrap-mode=nodownload')+$Options
    if(Test-Path -LiteralPath (Join-Path $build meson-private/coredata.dat)){$setupArguments+='--reconfigure'}
    Step "$name-configure" meson $setupArguments
    Step "$name-build" meson @('compile','-C',$build,'-j','4')
    Step "$name-install" meson @('install','-C',$build)
    $saved=Join-Path $inputRoot "$name-build-options.json"
    Copy-Item -LiteralPath (Join-Path $build meson-info/intro-buildoptions.json) -Destination $saved;Save $saved
}
function VerifyPackage($Library,[string]$Package){
    Step "$($Library.name)-version" pkg-config @('--modversion',$Package)
    $version=(& pkg-config --modversion $Package|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $version -ne $Library.version){throw "$Package selected $version rather than resolved $($Library.version)"}
}
. (Join-Path $PSScriptRoot prepare-linux-llvm.ps1)
try{
    if(-not $IsLinux -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64'){throw 'Native Linux x64 is required'}
    Step host-identity uname @('-a')
    Initialize-LinuxLlvm -ReferenceRoot $ReferenceRoot -DepsRoot $DepsRoot -RunRoot $RunRoot
    Step compiler-identity $env:CC @('--version')
    Step bison-identity bison @('--version')
    Step make-identity make @('--version')
    Step github-cli-identity gh @('--version')
    $pythonPackages=@{}
    foreach($name in @('pip','meson','ninja','cmake')){
        $path=Join-Path $inputRoot "$name-release.json"
        Step "$name-latest" curl @('--fail','--silent','--show-error','--location','--max-time','60','-o',$path,"https://pypi.org/pypi/$name/json")
        Save $path;$pythonPackages[$name]=(Get-Content -LiteralPath $path -Raw|ConvertFrom-Json).info.version
    }
    $venv=Join-Path $DepsRoot ('python/wlr-'+(($pythonPackages.Values|Sort-Object)-join '-'))
    Step python-environment python3 @('-m','venv',$venv)
    $python=Join-Path $venv bin/python
    $report=Join-Path $inputRoot python-install.json
    Step python-tools $python @('-m','pip','install','--upgrade','--report',$report,'pip','meson','ninja','cmake');Save $report
    $installed=(& $python -m pip list --format=json|ConvertFrom-Json)
    if($LASTEXITCODE -ne 0){throw 'Python tool resolution query failed'}
    foreach($name in $pythonPackages.Keys){if(($installed|Where-Object name -EQ $name).version -ne $pythonPackages[$name]){throw "$name differs from its formal package release"}}
    $wheels=Join-Path $inputRoot python-wheels;New-Item -ItemType Directory -Path $wheels|Out-Null
    Step python-offline-inputs $python @('-m','pip','download','--only-binary=:all:','--dest',$wheels,'pip','meson','ninja','cmake')
    Get-ChildItem -LiteralPath $wheels -File|ForEach-Object{Save $_.FullName}
    $env:PATH=(Join-Path $venv bin)+':'+$env:PATH
    $ffi=AssetSource libffi libffi/libffi '^v(\d+\.\d+\.\d+)$'
    $expat=AssetSource expat libexpat/libexpat '^R_(\d+_\d+_\d+)$'
    $wayland=GitSource wayland 'wayland%2Fwayland' https://gitlab.freedesktop.org/wayland/wayland.git '^(\d+\.\d+\.\d+)$'
    $protocols=GitSource wayland-protocols 'wayland%2Fwayland-protocols' https://gitlab.freedesktop.org/wayland/wayland-protocols.git '^(\d+\.\d+(?:\.\d+)?)$'
    $drm=GitSource libdrm 'mesa%2Fdrm' https://gitlab.freedesktop.org/mesa/drm.git '^libdrm-(\d+\.\d+\.\d+)$'
    $pixman=GitSource pixman 'pixman%2Fpixman' https://gitlab.freedesktop.org/pixman/pixman.git '^pixman-(\d+\.\d+\.\d+)$'
    $xkb=GithubGitSource xkbcommon xkbcommon/libxkbcommon '^xkbcommon-(\d+\.\d+\.\d+)$'
    $wlr=GitSource wlroots 'wlroots%2Fwlroots' https://gitlab.freedesktop.org/wlroots/wlroots.git '^(\d+\.\d+\.\d+)$'
    $options=[ordered]@{
        wayland=@('-Dtests=false','-Ddocumentation=false','-Ddtd_validation=false')
        protocols=@('-Dtests=false')
        drm=@('-Dauto_features=disabled','-Dtests=false')
        pixman=@('-Dtests=disabled','-Ddemos=disabled','-Dgtk=disabled','-Dlibpng=disabled')
        xkb=@('-Denable-tools=false','-Denable-x11=false','-Denable-docs=false','-Denable-wayland=false','-Denable-xkbregistry=false')
        wlr=@('-Dauto_features=disabled','-Dexamples=false','-Drenderers=[]','-Dbackends=[]','-Dallocators=[]','-Dsession=disabled','-Dxwayland=disabled','-Dcolor-management=disabled','-Dlibliftoff=disabled')
    }
    $configuration=Join-Path $inputRoot native-configuration.json
    Json $configuration ([ordered]@{libraries=@($libraries.ToArray());python=$pythonPackages;options=$options;compiler=(& $env:CC --version|Out-String);scope='Owned real wlroots headless/Pixman compositor, system libwayland-client, no host desktop or product backend'})
    Save $configuration
    $key=(Get-FileHash -LiteralPath $configuration -Algorithm SHA256).Hash.ToLowerInvariant()
    $native=Join-Path $DepsRoot "native/wlr-$key";$prefix=Join-Path $native install
    New-Item -ItemType Directory -Path $native -Force|Out-Null
    $env:PKG_CONFIG_PATH=(Join-Path $prefix lib/pkgconfig)+':'+(Join-Path $prefix share/pkgconfig)+':'+$env:PKG_CONFIG_PATH
    $env:LD_LIBRARY_PATH=(Join-Path $prefix lib)+':'+$env:LD_LIBRARY_PATH
    $env:PATH=(Join-Path $prefix bin)+':'+$env:PATH
    $build=Join-Path $native ffi-build;New-Item -ItemType Directory -Path $build|Out-Null
    Push-Location -LiteralPath $build
    try{Step ffi-configure (Join-Path $ffi.source configure) @("--prefix=$prefix","--libdir=$(Join-Path $prefix lib)",'--disable-docs','--disable-static','--disable-multi-os-directory');Step ffi-build make @('-j4');Step ffi-install make @('install')}finally{Pop-Location}
    VerifyPackage $ffi libffi
    Step expat-configure cmake @('-S',$expat.source,'-B',(Join-Path $native expat-build),'-G','Ninja','-DCMAKE_BUILD_TYPE=Release',"-DCMAKE_INSTALL_PREFIX=$prefix",'-DCMAKE_INSTALL_LIBDIR=lib','-DEXPAT_BUILD_DOCS=OFF','-DEXPAT_BUILD_EXAMPLES=OFF','-DEXPAT_BUILD_TESTS=OFF','-DEXPAT_BUILD_TOOLS=OFF','-DEXPAT_SHARED_LIBS=ON')
    Step expat-build cmake @('--build',(Join-Path $native expat-build),'--parallel','4')
    Step expat-install cmake @('--install',(Join-Path $native expat-build));VerifyPackage $expat expat
    MesonBuild $wayland $options.wayland;VerifyPackage $wayland wayland-client
    MesonBuild $protocols $options.protocols;VerifyPackage $protocols wayland-protocols
    MesonBuild $drm $options.drm;VerifyPackage $drm libdrm
    MesonBuild $pixman $options.pixman;VerifyPackage $pixman pixman-1
    MesonBuild $xkb $options.xkb;VerifyPackage $xkb xkbcommon
    MesonBuild $wlr $options.wlr
    $wlrPackage='wlroots-'+(($wlr.version -split '\.')[0..1]-join '.')
    VerifyPackage $wlr $wlrPackage
    $flagPath=Join-Path $inputRoot compositor-flags.json
    # The fixture directly calls libwayland-server. wlroots' private link
    # dependencies do not supply those symbols to this executable.
    & $python -c 'import json,shlex,subprocess,sys;print(json.dumps(shlex.split(subprocess.check_output(["pkg-config","--cflags","--libs",*sys.argv[1:]],text=True))))' $wlrPackage wayland-server > $flagPath
    if($LASTEXITCODE -ne 0){throw 'Native fixture flags query failed'};Save $flagPath
    $flags=Get-Content -LiteralPath $flagPath -Raw|ConvertFrom-Json
    $binary=Join-Path $native mfs-p0-wlr-compositor
    Step native-compositor-build $env:CC (@('-std=c11','-D_POSIX_C_SOURCE=200809L','-DWLR_USE_UNSTABLE','-O2','-Wall','-Wextra','-Werror',(Join-Path $productRoot tests/fixtures/wlr-compositor.c))+$flags+@('-fuse-ld=lld','-o',$binary))
    Step native-compositor-linkage ldd @($binary)
    $nativeArtifacts=Join-Path $inputRoot native-artifacts;New-Item -ItemType Directory -Path $nativeArtifacts|Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $nativeArtifacts mfs-p0-wlr-compositor);Save (Join-Path $nativeArtifacts mfs-p0-wlr-compositor)
    Get-ChildItem -LiteralPath (Join-Path $prefix lib) -File -Filter '*.so.*'|Where-Object{-not $_.LinkType}|ForEach-Object{$saved=Join-Path $nativeArtifacts $_.Name;Copy-Item -LiteralPath $_.FullName -Destination $saved;Save $saved}
    $library=(& readlink -f (Join-Path $prefix lib/libwayland-client.so)|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $library)){throw 'Prepared native client library identity absent'}
    $env:MFS_P0_WAYLAND_LIBRARY=$library
    . (Join-Path $PSScriptRoot configure-prototype.ps1) -ConfigureTool $ConfigureTool -RustBatch $RustBatch -ReferenceRoot $ReferenceRoot -Family wlr
    Save $env:MFS_BUILD_CONFIGURATION
    if($SourceReference){
        try { Step rust-wlr-migration-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-linux-wlr,linux-wlr','--','-D','warnings') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        . (Join-Path $PSScriptRoot prepare-wlr-reference.ps1)
        $sourceExecutable=Prepare-WlrReference $UpstreamRoot
        $referenceArtifacts=Join-Path $RunRoot linux-x64/reference-u/artifacts
        New-Item -ItemType Directory -Path $referenceArtifacts -Force|Out-Null
        Json (Join-Path $referenceArtifacts geometry.json) ([ordered]@{width=64;height=48;format='BGR';stride=192;pattern='top-left red, top-right green, bottom blue';scope='Original source context only; external UUID name provider and suppressed logging; no Rust capture comparison.'})
    }
    $runtime=Join-Path $DepsRoot ('runtime/wlr-'+[Guid]::NewGuid().ToString('N').Substring(0,12))
    $socket=Join-Path $runtime mfs-p0-wlr
    if([Text.Encoding]::UTF8.GetByteCount($socket) -ge 108){throw 'Private Wayland socket exceeds Linux sun_path capacity'}
    New-Item -ItemType Directory -Path $runtime|Out-Null;Step runtime-permissions chmod @('700',$runtime)
    $env:XDG_RUNTIME_DIR=$runtime
    # No host WAYLAND_DISPLAY or WAYLAND_SOCKET is consumed by either client.
    $env:WAYLAND_DISPLAY='mfs-p0-wlr';Remove-Item Env:WAYLAND_SOCKET -ErrorAction SilentlyContinue
    $start=[Diagnostics.ProcessStartInfo]::new($binary);$start.UseShellExecute=$false;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
    if($SourceReference){$start.ArgumentList.Add('--source-client')}
    $server=[Diagnostics.Process]::Start($start);$serverOut=$server.StandardOutput.ReadToEndAsync();$serverError=$server.StandardError.ReadToEndAsync()
    $until=[DateTime]::UtcNow.AddSeconds(10)
    while(-not(Test-Path -LiteralPath $socket)){if($server.HasExited -or [DateTime]::UtcNow -ge $until){throw 'Owned native compositor failed to start'};Start-Sleep -Milliseconds 50}
    $env:MFS_P0_WLR_SOCKET=$socket
    if($SourceReference){
        try { Step original-wlr-source-context timeout @('--signal=TERM','--kill-after=5s','90s',$sourceExecutable,$socket,$referenceArtifacts,$library) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        foreach($probe in @(@('shared-client','shared'),@('reconnect','reconnect'),@('fd-zero','fd-zero'),@('manager','manager'),@('nul-names','nul-names'))){
            $probeArtifacts=Join-Path $referenceArtifacts $probe[0]
            New-Item -ItemType Directory -Path $probeArtifacts|Out-Null
            try { Step ('original-wlr-'+$probe[0]) timeout @('--signal=TERM','--kill-after=5s','90s',$sourceExecutable,$socket,$probeArtifacts,$library,$probe[1]) }
            catch { $failed=$true;Write-Host $_.Exception.Message }
        }
        $env:MFS_P1_WLR_SOURCE_ARTIFACTS=$referenceArtifacts
        $env:MFS_P1_WLR_RUST_ARTIFACTS=Join-Path $referenceArtifacts rust
        New-Item -ItemType Directory -Path $env:MFS_P1_WLR_RUST_ARTIFACTS -Force|Out-Null
        $managerOracles=@(Get-ChildItem (Join-Path $referenceArtifacts manager) -File -Filter '*.bgr'|ForEach-Object{[ordered]@{name=$_.Name;sha256=(Get-FileHash $_.FullName).Hash.ToLowerInvariant()}})
        $managerHashes=Join-Path $referenceArtifacts manager-original-hashes.json
        Json $managerHashes ([ordered]@{recorded_before_rust=$true;images=$managerOracles});Save $managerHashes
        if($managerOracles.Count -ne 60){$failed=$true;Write-Host 'Original manager oracle is incomplete; retain failures without retry'}
        Json (Join-Path $env:MFS_P1_WLR_RUST_ARTIFACTS geometry.json) ([ordered]@{width=64;height=48;bgr_stride=192;raw_format='per-image metadata';scope='Private Rust Wlr migration: preserved recursive callback, frame alive through conversion, one added raw CPU copy; no full Wlr/P1 or public SDK acceptance.'})
        try { Step rust-wlr-layout-controls cargo @('test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::rejects_overflowing_native_layouts_and_unsupported_source_conversions','--','--exact','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-wlr-manager-controls cargo @('test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::manager::native::tests::','--','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        # Each native resource baseline belongs to a fresh isolated process,
        # without unrelated libtest worker/reader teardown in its task set.
        try { Step rust-wlr-migration-capture timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::captures_source_scene_with_recursive_roundtrip_and_retained_outputs','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-wlr-conversion-exception timeout @('--signal=TERM','--kill-after=5s','90s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::releases_frame_after_native_conversion_exception_and_captures_again','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-wlr-fd-zero timeout @('--signal=TERM','--kill-after=5s','90s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::retains_original_fd_zero_residual_and_cpu_outputs_until_fixture_reclaim','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        foreach($probe in @(@('shared-client','retains_shared_client_after_caller_and_first_capture_release'),@('reconnect','reconnects_same_and_changed_paths_after_old_capture_owners_end'),@('nul-names','names::preserves_embedded_nul_native_name_and_full_reconnect_identity'))){
            try { Step ('rust-wlr-'+$probe[0]) timeout @('--signal=TERM','--kill-after=5s','90s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr',('backends::linux::wlr::tests::'+$probe[1]),'--','--exact','--ignored','--test-threads=1','--nocapture') }
            catch { $failed=$true;Write-Host $_.Exception.Message }
        }
        try { Step rust-wlr-manager timeout @('--signal=TERM','--kill-after=5s','90s','cargo','test','--release','--locked','--lib','--features','prototype-linux-wlr,linux-wlr','backends::linux::wlr::tests::forwards_native_manager_reconnects_and_retains_images_after_shutdown','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        $managerPreserved=$managerOracles.Count -eq 60
        foreach($oracle in $managerOracles){if((Get-FileHash (Join-Path (Join-Path $referenceArtifacts manager) $oracle.name)).Hash.ToLowerInvariant() -ne $oracle.sha256){$managerPreserved=$false}}
        Json (Join-Path $referenceArtifacts manager-original-preserved.json) ([ordered]@{checked=$managerOracles.Count;all_original_hashes_unchanged=$managerPreserved})
        if(-not $managerPreserved){$failed=$true;Write-Host 'Original manager oracle hashes changed or were incomplete'}
    }else{
        Step rust-wlr-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-linux-wlr','--','-D','warnings')
        Step rust-wlr-capture timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--test','p0-interop','--features','prototype-linux-wlr','--','--include-ignored','--test-threads=1','--nocapture')
    }
}catch{
    $failed=$true;$log=Join-Path $logsRoot preparation-error.log
    $_.Exception.ToString()|Set-Content -LiteralPath $log -Encoding utf8
    $checks.Add([ordered]@{name='wlr-preparation';status='failed';log=(Artifact $log)});Write-Host $_.Exception.ToString()
}finally{
    if($referenceArtifacts){Get-ChildItem -LiteralPath $referenceArtifacts -File -Recurse|ForEach-Object{Save $_.FullName}}
    if($server){
        $log=Join-Path $logsRoot compositor-stop.log;$status='passed'
        try{
            $unexpected=$server.HasExited;$forced=$false
            if(-not $unexpected){& /bin/kill -TERM $server.Id;if($LASTEXITCODE -ne 0 -and -not $server.HasExited){$server.Kill();$forced=$true};if(-not $server.WaitForExit(10000)){$server.Kill();$server.WaitForExit();$forced=$true}}
            if($unexpected -or $forced -or $server.ExitCode -ne 0){$status='failed'}
            "owned_pid=$($server.Id), exit=$($server.ExitCode), unexpected=$unexpected, forced=$forced"|Set-Content -LiteralPath $log -Encoding utf8
            foreach($item in @(@('compositor-stdout.log',$serverOut),@('compositor-stderr.log',$serverError))){$path=Join-Path $logsRoot $item[0];$item[1].GetAwaiter().GetResult()|Set-Content -LiteralPath $path -Encoding utf8;Save $path}
        }catch{$status='failed';$_.Exception.ToString()|Add-Content -LiteralPath $log -Encoding utf8}
        finally{$checks.Add([ordered]@{name='compositor-stop';status=$status;log=(Artifact $log)});$server.Dispose()}
    }
    Json (Join-Path $RunRoot native-result.json) ([ordered]@{layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase=($SourceReference ? 'P1' : 'P0');phase_status='incomplete';checks=@($checks.ToArray());inputs=@($inputs.ToArray());pending=@(($SourceReference ? 'Fixed original Wlr source context: native buffers/Mat owners and forwarding roundtrip recursion observation; private namespace/logging and a kernel-random external UUID name provider. Passive virtual input manager globals are a source prerequisite; no input actions or host devices. Private Rust migration is now included with separate build/layout/capture/exception probes; full source failure/format/client-close/FD0/partial-state parity and P1 remain unaccepted.' : 'Native headless/Pixman Wlr feasibility only; full compositor/format/failure and C++ parity gates remain outstanding.'),'Compiler/make/bison/OS identities are recorded; complete latest native toolchain policy verification and offline entry remain pending.','The initial private Rust Wlr slice runs only in explicit P1 probes; full backend/P1 acceptance remains pending. No public SDK target policy or ExtImage capture.')})
}
if($failed -or @($checks|Where-Object status -EQ failed).Count){exit 1}
