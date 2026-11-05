# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit reference dependency; caller owns Step/Save/Json/DepsRoot/RunRoot.
function Prepare-BoostReference([string]$InputDirectory,[string]$StepPrefix='boost') {
    if($StepPrefix -notmatch '^[a-z][a-z0-9-]*$'){throw 'Invalid Boost step prefix'}
    $metadata=Join-Path $InputDirectory boost-release.json
    & gh api repos/boostorg/boost/releases/latest | Set-Content -LiteralPath $metadata -Encoding utf8
    if($LASTEXITCODE -ne 0){throw 'Cannot resolve current formal Boost release; cache is not a fallback'}
    Save $metadata
    $release=Get-Content -LiteralPath $metadata -Raw|ConvertFrom-Json
    if($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^boost-\d+\.\d+\.\d+$'){
        throw 'Unrecognized formal Boost release'
    }
    $assets=@($release.assets|Where-Object name -Match '-cmake.tar.xz$')
    if($assets.Count -ne 1){throw 'Boost needs exactly one complete CMake source archive'}
    $asset=$assets[0]
    if($asset.digest -cnotmatch '^sha256:[a-f0-9]{64}$' -or $asset.name -cnotmatch '^boost-[0-9.]+-cmake\.tar\.xz$'){
        throw 'Invalid Boost archive identity'
    }
    $curlProgram=if($IsWindows){'curl.exe'}else{'curl'}
    $downloads=Join-Path $DepsRoot downloads
    New-Item -ItemType Directory -Path $downloads -Force|Out-Null
    $archive=Join-Path $downloads $asset.name
    if(-not(Test-Path -LiteralPath $archive)){
        Step ($StepPrefix+'-download') $curlProgram @('--fail','--location','--connect-timeout','15','--max-time','600','--proto','=https','-o',$archive,$asset.browser_download_url)
    }
    if((Get-Item -LiteralPath $archive).Length -ne $asset.size -or
       'sha256:'+(Get-FileHash -LiteralPath $archive).Hash.ToLowerInvariant() -cne $asset.digest){throw 'Boost source archive identity differs'}
    $saved=Join-Path $InputDirectory $asset.name
    Copy-Item -LiteralPath $archive -Destination $saved;Save $saved
    # A new per-batch tree is reconstructed even on a verified archive cache hit.
    $source=Join-Path $DepsRoot ('native-build/boost-reference/'+(Split-Path -Leaf $RunRoot)+'/source')
    if(Test-Path -LiteralPath $source){throw 'Boost reference source directory already exists'}
    New-Item -ItemType Directory -Path $source -Force|Out-Null
    $unpack=@('-xf',$archive,'--strip-components=1','-C',$source)
    if($IsWindows){$unpack+=@('--exclude=*/libs/decimal/doc/modules/ROOT/examples')}
    Step ($StepPrefix+'-unpack') tar $unpack
    $license=Join-Path $InputDirectory Boost-LICENSE_1_0.txt
    Copy-Item -LiteralPath (Join-Path $source LICENSE_1_0.txt) -Destination $license;Save $license
    $identity=Join-Path $InputDirectory boost-prepared.json
    Json $identity ([ordered]@{version=$release.tag_name;archive_digest=$asset.digest;source=$source;scope='Original MaaUtils UUID acceptance only; no core/default product dependency.'});Save $identity
    return [ordered]@{Source=$source;Version=$release.tag_name}
}
