# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit acceptance dependency only; no capture runtime/build.rs dependency.
# Caller owns Step/Save/Json/DepsRoot and an already-new external input folder.
function Prepare-MeoJsonReference([string]$InputDirectory) {
    $releasePath=Join-Path $InputDirectory meojson-latest-release.json
    # Query once: the archived response is also the actual resolution input.
    & gh api repos/MistEO/meojson/releases/latest | Set-Content -LiteralPath $releasePath -Encoding utf8
    if($LASTEXITCODE -ne 0){throw 'Cannot record current formal meojson release'}
    Save $releasePath
    $release=Get-Content -LiteralPath $releasePath -Raw|ConvertFrom-Json
    if($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^v\d+\.\d+\.\d+$'){
        throw 'Unrecognized formal meojson release'
    }
    $tagPath=Join-Path $InputDirectory meojson-tag.json
    & gh api ('repos/MistEO/meojson/git/ref/tags/'+$release.tag_name) | Set-Content -LiteralPath $tagPath -Encoding utf8
    if($LASTEXITCODE -ne 0){throw 'Cannot resolve formal meojson tag'}
    Save $tagPath
    $object=(Get-Content -LiteralPath $tagPath -Raw|ConvertFrom-Json).object
    $depth=0
    while($object.type -eq 'tag') {
        if($depth -ge 8){throw 'Too many annotated meojson tag indirections'}
        $tagObject=Join-Path $InputDirectory "meojson-tag-object-$depth.json"
        & gh api ('repos/MistEO/meojson/git/tags/'+$object.sha)|Set-Content -LiteralPath $tagObject -Encoding utf8
        if($LASTEXITCODE -ne 0){throw 'Cannot resolve annotated meojson tag'}
        Save $tagObject
        $object=(Get-Content -LiteralPath $tagObject -Raw|ConvertFrom-Json).object
        $depth++
    }
    if($object.type -ne 'commit' -or $object.sha -notmatch '^[0-9a-f]{40}$'){throw 'meojson tag does not resolve to a commit'}
    $source=Join-Path $DepsRoot ('native-sources/meojson-'+$object.sha)
    if(-not(Test-Path -LiteralPath $source)){
        Step meojson-clone git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$release.tag_name,'https://github.com/MistEO/meojson.git',$source)
    }
    $actual=(& git -C $source rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $actual -ne $object.sha){throw 'meojson cache differs from the current formal tag'}
    $dirty=& git -C $source status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw 'meojson source cache was modified'}
    $archive=Join-Path $InputDirectory meojson-source.tar
    Step meojson-source-archive git @('-C',$source,'archive','--format=tar','--output',$archive,$actual)
    Save $archive
    $license=Join-Path $InputDirectory meojson-LICENSE
    Copy-Item -LiteralPath (Join-Path $source LICENSE) -Destination $license;Save $license
    $prefix=Join-Path $DepsRoot ('native-includes/meojson-'+$actual)
    $headers=Join-Path $prefix meojson
    if(-not(Test-Path -LiteralPath $prefix)){
        New-Item -ItemType Directory -Path $prefix|Out-Null
        Copy-Item -LiteralPath (Join-Path $source include) -Destination $headers -Recurse
    }
    $identities=@(Get-ChildItem -LiteralPath (Join-Path $source include) -File -Recurse|ForEach-Object{
        $relative=[IO.Path]::GetRelativePath((Join-Path $source include),$_.FullName)
        $prepared=Join-Path $headers $relative
        $hash=(Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant()
        if(-not(Test-Path -LiteralPath $prepared) -or (Get-FileHash -LiteralPath $prepared).Hash.ToLowerInvariant() -ne $hash){throw 'meojson prepared header differs from its formal source'}
        [ordered]@{path=$relative.Replace('\','/');sha256=$hash}
    })
    if(@(Get-ChildItem -LiteralPath $headers -File -Recurse).Count -ne $identities.Count){throw 'Unexpected meojson prepared headers'}
    $identity=Join-Path $InputDirectory meojson-prepared.json
    Json $identity ([ordered]@{version=$release.tag_name;commit=$actual;headers=$identities;scope='MIT header-only configuration/API reference oracle dependency, outside capture runtime and default product assets.'});Save $identity
    return [ordered]@{Include=$prefix;Version=$release.tag_name;Commit=$actual}
}

function Restore-MeoJsonReference([string]$Archive,[string]$Identity,[string]$Destination) {
    # Caller verified archived input digests. Reconstruct headers exclusively
    # from that batch, then validate every staged byte; never query online here.
    New-Item -ItemType Directory -Path $Destination|Out-Null
    Step custom-meojson-replay tar @('-xf',$Archive,'-C',$Destination)
    $prefix=Join-Path $Destination staged
    New-Item -ItemType Directory -Path $prefix|Out-Null
    Copy-Item -LiteralPath (Join-Path $Destination include) -Destination (Join-Path $prefix meojson) -Recurse
    $identityValue=Get-Content -LiteralPath $Identity -Raw|ConvertFrom-Json
    foreach($header in $identityValue.headers) {
        if([IO.Path]::IsPathRooted($header.path) -or $header.path -match '(^|[/\\])\.\.([/\\]|$)'){throw 'Invalid archived meojson header path'}
        $path=Join-Path (Join-Path $prefix meojson) $header.path
        if((Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant() -ne $header.sha256){throw 'Archived meojson header changed'}
    }
    if(@(Get-ChildItem (Join-Path $prefix meojson) -File -Recurse).Count -ne @($identityValue.headers).Count){throw 'Archived meojson header inventory differs'}
    return [ordered]@{Include=$prefix;Version=$identityValue.version;Commit=$identityValue.commit}
}

# The owning native batch builds the explicit target alongside its source
# reference. Preserve exact copied declarations/extracted bodies and every case,
# including partial records on a failed observation; never rerun a failed case.
function Observe-LinuxConfigReference([string]$Build,[string]$InputDirectory,[string]$FixedUpstream) {
    $managerBuild=Join-Path $Build linux-manager
    $executable=Join-Path $managerBuild mfs_linux_config_model
    $saved=Join-Path $InputDirectory mfs_linux_config_model
    Copy-Item -LiteralPath $executable -Destination $saved;Save $saved
    Step linux-config-model-linkage ldd @($executable)
    $declarations=Join-Path $InputDirectory config-oracle-sources
    New-Item -ItemType Directory -Path $declarations|Out-Null
    foreach($relative in @('linux-config-api.inc','linux-config-manager.inc','config-overlay/Base/Config.h','config-overlay/Base/UnitBase.h','config-overlay/Manager/LinuxControlUnitMgr.h')){
        $destination=Join-Path $declarations $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force|Out-Null
        Copy-Item -LiteralPath (Join-Path $managerBuild $relative) -Destination $destination;Save $destination
    }
    $observations=Join-Path $InputDirectory configuration-observations
    try { Step linux-config-source-corpus $ConfigureTool @('upstream-config','linux',$executable,$observations) }
    finally { if(Test-Path -LiteralPath $observations){Get-ChildItem -LiteralPath $observations -Recurse -File|ForEach-Object{Save $_.FullName}} }
    $env:MFS_P1_LINUX_CONFIG_OBSERVATIONS=$observations
    $env:MFS_P1_LINUX_CONFIG_RUST_ARTIFACTS=Join-Path $InputDirectory configuration-rust
    $compat=Join-Path $InputDirectory compat
    New-Item -ItemType Directory -Path $compat|Out-Null
    foreach($name in @('WlRootsControlUnitCompat.cpp','KWinControlUnitCompat.cpp')){
        $source=Join-Path $FixedUpstream "source/MaaLinuxControlUnit/Compat/$name"
        $destination=Join-Path $declarations $name
        Copy-Item -LiteralPath $source -Destination $destination;Save $destination
    }
    try{Step linux-compat-source-corpus $ConfigureTool @('upstream-config','linux-compat',$executable,(Join-Path $compat source))}
    finally{if(Test-Path -LiteralPath $compat){Get-ChildItem -LiteralPath $compat -Recurse -File|ForEach-Object{Save $_.FullName}}}
    $env:MFS_P1_LINUX_COMPAT_OBSERVATIONS=Join-Path $compat source
    $env:MFS_P1_LINUX_COMPAT_RUST_ARTIFACTS=Join-Path $compat rust
    $env:MFS_P1_LINUX_COMPAT_MATRIX=$compat
    . (Join-Path $PSScriptRoot prepare-manager-metadata.ps1)
    Save-ManagerMetadataReference (Join-Path $managerBuild metadata) $InputDirectory (Join-Path $RunRoot linux-x64/reference-u/artifacts/manager-metadata)
}
