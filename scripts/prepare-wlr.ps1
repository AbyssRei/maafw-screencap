# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$DepsRoot,
    [Parameter(Mandatory)][string]$RunRoot,
    [Parameter(Mandatory)][string]$ConfigureTool,
    [Parameter(Mandatory)][string]$RustBatch
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference=$false
$productRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $productRoot
$inputRoot=Join-Path $RunRoot inputs
$logsRoot=Join-Path $RunRoot linux-x64-wlr/logs
New-Item -ItemType Directory -Path $logsRoot -Force|Out-Null
$checks=[Collections.Generic.List[object]]::new()
$inputs=[Collections.Generic.List[object]]::new()
$libraries=[Collections.Generic.List[object]]::new()
$server=$null; $serverOut=$null; $serverError=$null; $failed=$false

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
try{
    if(-not $IsLinux -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64'){throw 'Native Linux x64 is required'}
    Step host-identity uname @('-a')
    Step compiler-identity cc @('--version')
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
    Json $configuration ([ordered]@{libraries=@($libraries.ToArray());python=$pythonPackages;options=$options;compiler=(& cc --version|Out-String);scope='Owned real wlroots headless/Pixman compositor, system libwayland-client, no host desktop or product backend'})
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
    Step native-compositor-build cc (@('-std=c11','-D_POSIX_C_SOURCE=200809L','-DWLR_USE_UNSTABLE','-O2','-Wall','-Wextra','-Werror',(Join-Path $productRoot tests/fixtures/wlr-compositor.c))+$flags+@('-o',$binary))
    Step native-compositor-linkage ldd @($binary)
    $nativeArtifacts=Join-Path $inputRoot native-artifacts;New-Item -ItemType Directory -Path $nativeArtifacts|Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $nativeArtifacts mfs-p0-wlr-compositor);Save (Join-Path $nativeArtifacts mfs-p0-wlr-compositor)
    Get-ChildItem -LiteralPath (Join-Path $prefix lib) -File -Filter '*.so.*'|Where-Object{-not $_.LinkType}|ForEach-Object{$saved=Join-Path $nativeArtifacts $_.Name;Copy-Item -LiteralPath $_.FullName -Destination $saved;Save $saved}
    $library=(& readlink -f (Join-Path $prefix lib/libwayland-client.so)|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $library)){throw 'Prepared native client library identity absent'}
    $env:MFS_P0_WAYLAND_LIBRARY=$library
    . (Join-Path $PSScriptRoot configure-prototype.ps1) -ConfigureTool $ConfigureTool -RustBatch $RustBatch -ReferenceRoot $ReferenceRoot -Family wlr
    Save $env:MFS_BUILD_CONFIGURATION
    $runtime=Join-Path $DepsRoot ('runtime/wlr-'+[Guid]::NewGuid().ToString('N').Substring(0,12))
    $socket=Join-Path $runtime mfs-p0-wlr
    if([Text.Encoding]::UTF8.GetByteCount($socket) -ge 108){throw 'Private Wayland socket exceeds Linux sun_path capacity'}
    New-Item -ItemType Directory -Path $runtime|Out-Null;Step runtime-permissions chmod @('700',$runtime)
    $env:XDG_RUNTIME_DIR=$runtime
    # No host WAYLAND_DISPLAY or WAYLAND_SOCKET is consumed by either client.
    $env:WAYLAND_DISPLAY='mfs-p0-wlr';Remove-Item Env:WAYLAND_SOCKET -ErrorAction SilentlyContinue
    $start=[Diagnostics.ProcessStartInfo]::new($binary);$start.UseShellExecute=$false;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
    $server=[Diagnostics.Process]::Start($start);$serverOut=$server.StandardOutput.ReadToEndAsync();$serverError=$server.StandardError.ReadToEndAsync()
    $until=[DateTime]::UtcNow.AddSeconds(10)
    while(-not(Test-Path -LiteralPath $socket)){if($server.HasExited -or [DateTime]::UtcNow -ge $until){throw 'Owned native compositor failed to start'};Start-Sleep -Milliseconds 50}
    $env:MFS_P0_WLR_SOCKET=$socket
    Step rust-wlr-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-linux-wlr','--','-D','warnings')
    Step rust-wlr-capture timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--test','p0-interop','--features','prototype-linux-wlr','--','--include-ignored','--test-threads=1','--nocapture')
}catch{
    $failed=$true;$log=Join-Path $logsRoot preparation-error.log
    $_.Exception.ToString()|Set-Content -LiteralPath $log -Encoding utf8
    $checks.Add([ordered]@{name='wlr-preparation';status='failed';log=(Artifact $log)});Write-Host $_.Exception.ToString()
}finally{
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
    Json (Join-Path $RunRoot native-result.json) ([ordered]@{layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase='P0';phase_status='incomplete';checks=@($checks.ToArray());inputs=@($inputs.ToArray());pending=@('Native headless/Pixman Wlr feasibility only; full compositor/format/failure and C++ parity gates remain outstanding.','Compiler/make/bison/OS identities are recorded; complete latest native toolchain policy verification and offline entry remain pending.','No product backend, SDK target policy or ExtImage capture has been implemented.')})
}
if($failed -or @($checks|Where-Object status -EQ failed).Count){exit 1}
