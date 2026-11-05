# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit reference dependency; caller owns Step/Save/Json/DepsRoot.
function Prepare-MaaUtilsReference([string]$InputDirectory,[string]$StepPrefix='maautils') {
    if($StepPrefix -notmatch '^[a-z][a-z0-9-]*$'){throw 'Invalid MaaUtils step prefix'}
    # Resolve the current official policy even when the source cache exists.
    $repositoryFile=Join-Path $InputDirectory maautils-repository.json
    & gh api repos/MaaXYZ/MaaUtils > $repositoryFile
    if($LASTEXITCODE -ne 0){throw 'Official MaaUtils repository query failed'}
    Save $repositoryFile
    $releasesFile=Join-Path $InputDirectory maautils-releases.json
    & gh api --paginate --slurp 'repos/MaaXYZ/MaaUtils/releases?per_page=100' > $releasesFile
    if($LASTEXITCODE -ne 0){throw 'Official MaaUtils release query failed; cache is not a fallback'}
    Save $releasesFile
    $pages=Get-Content -LiteralPath $releasesFile -Raw|ConvertFrom-Json -NoEnumerate
    $formal=@($pages|ForEach-Object{$_}|Where-Object{-not $_.draft -and -not $_.prerelease}|Sort-Object published_at -Descending)
    $repository=Get-Content -LiteralPath $repositoryFile -Raw|ConvertFrom-Json
    $ref=if($formal.Count){$formal[0].tag_name}else{$repository.default_branch}
    $refKind=if($formal.Count){'tags'}else{'heads'}
    $remote=@(& git ls-remote https://github.com/MaaXYZ/MaaUtils.git "refs/$refKind/$ref" "refs/$refKind/$ref^{}")
    if($LASTEXITCODE -ne 0 -or -not $remote.Count){throw 'Resolved MaaUtils ref has no source identity'}
    $peeled=@($remote|Where-Object{$_ -match '\^\{\}$'})
    $commit=(($peeled.Count ? $peeled[0] : $remote[0]) -split '\s+')[0]
    if($commit -notmatch '^[0-9a-f]{40}$'){throw 'Invalid MaaUtils commit identity'}
    $policy=Join-Path $InputDirectory maautils-release-policy.json
    Json $policy ([ordered]@{formal_release=[bool]$formal.Count;selected_ref=$ref;commit=$commit;remote=$remote})
    Save $policy
    $source=Join-Path $DepsRoot "native-sources/MaaUtils-$commit"
    if(-not(Test-Path -LiteralPath $source)){
        Step ($StepPrefix+'-clone') git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$ref,'https://github.com/MaaXYZ/MaaUtils.git',$source)
    }
    $actual=(& git -C $source rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $actual -ne $commit){throw 'MaaUtils cached source identity differs'}
    $dirty=& git -C $source status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw 'MaaUtils source cache was modified'}
    $archive=Join-Path $InputDirectory MaaUtils-source.tar
    Step ($StepPrefix+'-archive') git @('-C',$source,'archive','--format=tar','--output',$archive,$commit)
    Save $archive
    $license=Join-Path $InputDirectory MaaUtils-LICENSE.md
    Copy-Item -LiteralPath (Join-Path $source LICENSE.md) -Destination $license
    Save $license
    return [ordered]@{Source=$source;Commit=$commit;Ref=$ref}
}
