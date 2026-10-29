# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Dot-sourced only by explicit native Linux/macOS source acceptance. The
# caller owns Step/Save/Json, selected compilers, Rust batch and final result.
function Assert-CustomNativeLog([string]$Text) {
    if($Text -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or $Text -notmatch 'native Custom cases=96 source_rust_captures=192 retained=36 real_imagebuffer=1 caller_callbacks=1 false_output_preserved=1 true_empty=1 raw_setter_clone=1 borrowed_table_changes=1 native_exception=1 subsequent_capture=1 native_mat_heads_closed=1 passed'){
        throw 'Custom native entry did not execute the complete one-test matrix'
    }
}
function Assert-CustomRuntimeTree([string]$Text) {
    if($Text -notmatch '(?m)^maafw-screencap v' -or $Text -match '[├└]'){throw 'Custom-only core unexpectedly includes a normal runtime dependency'}
}
function Test-CustomUnixReference([string]$FixedUpstream,[string]$Artifacts) {
    $log=Join-Path $logsRoot custom-reference.log
    $status='failed'
    try {
        'Explicit native Unix Custom preparation/callback/record/isolation entry; no desktop or input actions.'|Set-Content -LiteralPath $log -Encoding utf8
        Invoke-CustomUnixReference $FixedUpstream $Artifacts
        $status='passed'
        'Complete native callback, payload/owner records and core isolation checks passed.'|Add-Content -LiteralPath $log -Encoding utf8
    }catch{
        $_.Exception.Message|Add-Content -LiteralPath $log -Encoding utf8
        throw
    }finally{
        $checks.Add([ordered]@{name='custom-reference';status=$status;log=(Artifact $log)})
    }
}
function Invoke-CustomUnixReference([string]$FixedUpstream,[string]$Artifacts) {
    if(-not($IsLinux -or $IsMacOS)){throw 'Unix Custom acceptance requires a native Linux/macOS host'}
    $customInputs=Join-Path $inputRoot custom-reference
    New-Item -ItemType Directory -Path $customInputs|Out-Null
    # Resolve the current official policy even when the source cache exists.
    $repositoryFile=Join-Path $customInputs maautils-repository.json
    & gh api repos/MaaXYZ/MaaUtils > $repositoryFile
    if($LASTEXITCODE -ne 0){throw 'Official MaaUtils repository query failed'}
    Save $repositoryFile
    $releasesFile=Join-Path $customInputs maautils-releases.json
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
    $policy=Join-Path $customInputs maautils-release-policy.json
    Json $policy ([ordered]@{formal_release=[bool]$formal.Count;selected_ref=$ref;commit=$commit;remote=$remote})
    Save $policy
    $source=Join-Path $DepsRoot "native-sources/MaaUtils-$commit"
    if(-not(Test-Path -LiteralPath $source)){
        Step custom-maautils-clone git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$ref,'https://github.com/MaaXYZ/MaaUtils.git',$source)
    }
    $actual=(& git -C $source rev-parse HEAD|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $actual -ne $commit){throw 'MaaUtils cached source identity differs'}
    $dirty=& git -C $source status --porcelain --untracked-files=all
    if($LASTEXITCODE -ne 0 -or $dirty){throw 'MaaUtils source cache was modified'}
    $archive=Join-Path $customInputs MaaUtils-source.tar
    Step custom-maautils-archive git @('-C',$source,'archive','--format=tar','--output',$archive,$commit)
    Save $archive
    $license=Join-Path $customInputs MaaUtils-LICENSE.md
    Copy-Item -LiteralPath (Join-Path $source LICENSE.md) -Destination $license
    Save $license
    . (Join-Path $PSScriptRoot prepare-opencv-reference.ps1)
    $opencv=Prepare-OpenCvReference $customInputs ([ordered]@{
        backend='custom-capture';os=[Runtime.InteropServices.RuntimeInformation]::OSDescription
        scope='Same selected original bodies/raw setter, real ImageBuffer/vtable and Rust callback implementation as Windows; no desktop or input actions. Full metadata/framework/P1 acceptance remains incomplete.'
    }) -ImageCodecs -StepPrefix custom-opencv
    $build=Join-Path $opencv.Native ((Split-Path -Leaf $RunRoot)+'-custom')
    $configure=@('-S',(Join-Path $productRoot tests/upstream-parity/custom-capture),'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',"-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$($opencv.Package)","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_MAAUTILS_ROOT=$source")
    if($IsMacOS){$configure+=@('-DCMAKE_OSX_ARCHITECTURES=arm64','-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0',"-DCMAKE_OSX_SYSROOT:PATH=$($env:SDKROOT)")}
    Step custom-reference-configure cmake $configure
    Step custom-reference-build cmake @('--build',$build,'--parallel','4')
    foreach($name in 'CMakeCache.txt','compile_commands.json','custom-source.inc','custom-raw-source.inc','source-scope.txt'){
        $saved=Join-Path $customInputs $name
        Copy-Item -LiteralPath (Join-Path $build $name) -Destination $saved
        Save $saved
    }
    $libraryName=if($IsMacOS){'libmfs_custom_capture.dylib'}else{'libmfs_custom_capture.so'}
    $library=Join-Path $customInputs $libraryName
    Copy-Item -LiteralPath (Join-Path $build $libraryName) -Destination $library
    Save $library
    if($IsMacOS){Step custom-native-linkage otool @('-L',$library)}else{Step custom-native-linkage ldd @($library)}
    $evidence=Join-Path $Artifacts custom-capture
    New-Item -ItemType Directory -Path $evidence|Out-Null
    $oldConfiguration=$env:MFS_BUILD_CONFIGURATION
    $oldLibrary=$env:MFS_CUSTOM_CAPTURE_LIBRARY
    $oldArtifacts=$env:MFS_CUSTOM_CAPTURE_ARTIFACTS
    try {
        # Exercise the direct Custom-only consumer, independent of native
        # capture features or the prototype's prepared feature configuration.
        Remove-Item Env:MFS_BUILD_CONFIGURATION -ErrorAction SilentlyContinue
        $env:MFS_CUSTOM_CAPTURE_LIBRARY=$library
        $env:MFS_CUSTOM_CAPTURE_ARTIFACTS=$evidence
        Step custom-native-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::preserves_original_native_callbacks_pixels_empty_failure_and_owners','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-native-records $ConfigureTool @('upstream-custom',$evidence)
        Step custom-native-clippy cargo @('clippy','--workspace','--all-targets','--locked','--no-default-features','--features','custom-capture','--','-D','warnings')
        Step custom-core-direct-build cargo @('build','--release','--locked','--lib','--no-default-features','--features','custom-capture')
        Step custom-core-runtime-tree cargo @('tree','--locked','-p','maafw-screencap','--edges','normal','--no-default-features','--features','custom-capture')
    }finally{
        $env:MFS_BUILD_CONFIGURATION=$oldConfiguration
        $env:MFS_CUSTOM_CAPTURE_LIBRARY=$oldLibrary
        $env:MFS_CUSTOM_CAPTURE_ARTIFACTS=$oldArtifacts
        Get-ChildItem -LiteralPath $evidence -File -Recurse|ForEach-Object{Save $_.FullName}
    }
}
