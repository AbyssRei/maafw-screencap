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
function Assert-CustomMetadataLog([string]$Text) {
    if($Text -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or
       -not $Text.Contains('Custom metadata cases=592 payloads=37 source_rust_outputs=1184 actual_stringbuffer_meojson=1 parse_inside_scope=1 type_after_scope=1 borrowed_table_changes=1 exceptions_contained=1 retained_pairs=592 input_actions=0 passed')){
        throw 'Custom metadata entry did not execute the complete native parser/callback/owner matrix'
    }
}
function Assert-CustomApiLog([string]$Text) {
    if($Text -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or
       -not $Text.Contains('Custom native API lifecycles=20 source_rust_virtual_queries=1308 actual_create_destroy=1 public_bases=15 rust_drop_callbacks=20 scoped_caller_publications=112 retained_pixel_pairs=20 input_actions=0 passed')){
        throw 'Custom actual public API consumer did not execute its complete lifecycle/publication matrix'
    }
}
function Assert-CustomReentryLog([string]$Text) {
    if($Text -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or
       -not $Text.Contains('Custom reentry lifecycles=20 operation_pairs=49 fault_variants=2 actual_queries=7840 nested_cpp_exceptions=1960 held_native_values=240 scope_controls=7 input_actions=0 passed')){
        throw 'Custom nested original/Rust virtual callback, exception and scope matrix did not execute'
    }
}
function Assert-CustomImagesLog([string]$Text) {
    if($Text -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or
       -not $Text.Contains('Custom images lifecycles=20 cases=144 source_rust_captures=288 retained_native_images=288 inspection_decodes=576 input_actions=0 passed')){
        throw 'Expected actual Custom encoded callbacks, cache/resize checks and retained native image owners'
    }
}
function Assert-CustomRawLog([string]$Text) {
    if($Text -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;' -or
       -not $Text.Contains('Custom raw types=1664 cases=4992 source_rust_captures=9984 native_owners_retained=9984 rust_scopes=4992 exact_type_catalog=1 input_actions=0 passed')){
        throw 'Expected every actual compiled raw type, both manager callbacks and retained native owners'
    }
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
    . (Join-Path $PSScriptRoot prepare-meojson-reference.ps1)
    $customJson=Prepare-MeoJsonReference $customInputs -StepPrefix custom-meojson
    . (Join-Path $PSScriptRoot prepare-opencv-reference.ps1)
    $opencv=Prepare-OpenCvReference $customInputs ([ordered]@{
        backend='custom-capture';os=[Runtime.InteropServices.RuntimeInformation]::OSDescription
        scope='Same selected original screenshot/UUID/features/get_info bodies and raw setter, real ImageBuffer/StringBuffer/vtables/meojson and Rust callback implementation as Windows; no desktop or input actions. Full manager/framework/P1 acceptance remains incomplete.'
    }) -ImageCodecs -StepPrefix custom-opencv
    $build=Join-Path $opencv.Native ((Split-Path -Leaf $RunRoot)+'-custom')
    $configure=@('-S',(Join-Path $productRoot tests/upstream-parity/custom-capture),'-B',$build,'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',"-DCMAKE_CXX_COMPILER:FILEPATH=$($env:CXX)","-DOpenCV_DIR=$($opencv.Package)","-DMFS_UPSTREAM_ROOT=$FixedUpstream","-DMFS_MAAUTILS_ROOT=$source","-DMFS_MEOJSON_INCLUDE=$($customJson.Include)")
    if($IsMacOS){$configure+=@('-DCMAKE_OSX_ARCHITECTURES=arm64','-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0',"-DCMAKE_OSX_SYSROOT:PATH=$($env:SDKROOT)")}
    Step custom-reference-configure cmake $configure
    Step custom-reference-build cmake @('--build',$build,'--parallel','4')
    foreach($name in 'CMakeCache.txt','compile_commands.json','custom-source.inc','custom-raw-source.inc','source-scope.txt'){
        $saved=Join-Path $customInputs $name
        Copy-Item -LiteralPath (Join-Path $build $name) -Destination $saved
        Save $saved
    }
    $apiSources=Join-Path $customInputs native-api-source
    Copy-Item -LiteralPath (Join-Path $build native-api-source) -Destination $apiSources -Recurse
    Get-ChildItem -LiteralPath $apiSources -File -Recurse|ForEach-Object{Save $_.FullName}
    $rawHeaders=Join-Path $customInputs raw-type-headers
    Copy-Item -LiteralPath (Join-Path $build raw-type-headers) -Destination $rawHeaders -Recurse
    Get-ChildItem -LiteralPath $rawHeaders -File -Recurse|ForEach-Object{Save $_.FullName}
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
        Step custom-identity-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::identity::preserves_uuid_bytes_features_failures_and_borrowed_callbacks','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-identity-records $ConfigureTool @('upstream-trace','custom-identity',(Join-Path $evidence identity))
        Step custom-metadata-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::metadata::preserves_metadata_parser_bytes_scope_order_defaults_and_exceptions','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-metadata-records $ConfigureTool @('upstream-trace','custom-metadata',(Join-Path $evidence metadata))
        Step custom-api-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::native_api::consumes_original_creation_virtual_capture_and_destruction_with_rust_manager','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-api-records $ConfigureTool @('upstream-trace','custom-native-api',(Join-Path $evidence native-api))
        Step custom-reentry-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::native_api::reentry::preserves_same_unit_nested_queries_faults_and_independent_callers','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-reentry-records $ConfigureTool @('upstream-trace','custom-reentry',(Join-Path $evidence reentry))
        Step custom-images-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::images::preserves_encoded_callback_cache_resize_failures_and_native_owners','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-images-records $ConfigureTool @('upstream-trace','custom-images',(Join-Path $evidence images))
        Step custom-raw-parity cargo @('test','--release','--locked','--lib','--no-default-features','--features','custom-capture','backends::custom::native_tests::raw::preserves_every_compiled_raw_type_callback_bytes_and_native_owners','--','--exact','--ignored','--test-threads=1','--nocapture')
        Step custom-raw-records $ConfigureTool @('upstream-trace','custom-raw',(Join-Path $evidence raw))
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
