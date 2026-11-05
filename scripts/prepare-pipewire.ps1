# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$DepsRoot,
    [Parameter(Mandatory)][string]$RunRoot,
    [Parameter(Mandatory)][string]$ConfigureTool,
    [Parameter(Mandatory)][string]$RustBatch,
    [switch]$SourceReference,
    [string]$UpstreamRoot = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false
if($SourceReference -and (-not $UpstreamRoot -or -not(Test-Path -LiteralPath $UpstreamRoot -PathType Container))){throw 'Original PipeWire context requires the explicit verified upstream directory'}
$productRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $productRoot
$inputRoot = Join-Path $RunRoot inputs
$logsRoot = Join-Path $RunRoot linux-x64-pipewire/logs
New-Item -ItemType Directory -Path $inputRoot, $logsRoot -Force | Out-Null
$checks = [Collections.Generic.List[object]]::new()
$inputs = [Collections.Generic.List[object]]::new()
$server = $null
$serverOut = $null
$serverError = $null
$fixture = $null
$fixtureOut = $null
$fixtureError = $null
$routeSupervisor = $null
$routeOut = $null
$routeError = $null
$routeReady = $null
$failed = $false
$referenceArtifacts = $null
$rustArtifacts = $null
$fdArtifacts = $null
$bgrArtifacts = $null

function Artifact([string]$Path) {
    $relative = [IO.Path]::GetRelativePath($ReferenceRoot, $Path).Replace('\', '/')
    if ($relative.StartsWith('../') -or [IO.Path]::IsPathRooted($relative)) { throw 'Evidence escaped reference' }
    return [ordered]@{path=$relative; sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
function Save([string]$Path) { $inputs.Add((Artifact $Path)) }
function Read-PipeWireReleaseIdentity([object]$Release,[object]$Tag=$null) {
    if($Release -isnot [pscustomobject]){throw 'PipeWire release must be a JSON object'}
    $name=$Release.PSObject.Properties['tag_name']
    $upcoming=$Release.PSObject.Properties['upcoming_release']
    if($null -eq $name -or $name.Value -isnot [string] -or [string]::IsNullOrWhiteSpace($name.Value)){
        throw 'PipeWire release tag is missing or invalid'
    }
    if($null -eq $upcoming -or $upcoming.Value -isnot [bool] -or $upcoming.Value){
        throw 'PipeWire release metadata must identify an already released version'
    }
    $releaseCommit=$null
    $commitProperty=$Release.PSObject.Properties['commit']
    if($null -ne $commitProperty -and $null -ne $commitProperty.Value){
        if($commitProperty.Value -isnot [pscustomobject]){throw 'PipeWire release commit is malformed'}
        $id=$commitProperty.Value.PSObject.Properties['id']
        if($null -eq $id -or $id.Value -isnot [string] -or $id.Value -cnotmatch '^[0-9a-fA-F]{40}$'){
            throw 'PipeWire release commit must contain a complete Git identity'
        }
        $releaseCommit=$id.Value.ToLowerInvariant()
    }
    $commit=$releaseCommit
    $source='release.commit'
    if($null -ne $Tag){
        if($Tag -isnot [pscustomobject]){throw 'PipeWire repository tag must be a JSON object'}
        $tagName=$Tag.PSObject.Properties['name']
        $tagCommit=$Tag.PSObject.Properties['commit']
        if($null -eq $tagName -or $tagName.Value -isnot [string] -or $tagName.Value -cne $name.Value -or $null -eq $tagCommit -or $tagCommit.Value -isnot [pscustomobject]){
            throw 'PipeWire repository tag differs from the selected release'
        }
        $id=$tagCommit.Value.PSObject.Properties['id']
        if($null -eq $id -or $id.Value -isnot [string] -or $id.Value -cnotmatch '^[0-9a-fA-F]{40}$'){
            throw 'PipeWire repository tag has no complete commit identity'
        }
        $tagIdentity=$id.Value.ToLowerInvariant()
        if($null -ne $releaseCommit -and $releaseCommit -cne $tagIdentity){
            throw 'PipeWire release and repository tag commits differ'
        }
        $commit=$tagIdentity
        $source='repository.tag.commit'
    }
    return [pscustomobject]@{tag=$name.Value;commit=$commit;source=$source;release_commit=$releaseCommit;needs_tag=($null -eq $commit)}
}
function Json([string]$Path, [object]$Value) {
    $stream = [IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write)
    try { $stream.Write([Text.Encoding]::UTF8.GetBytes(($Value | ConvertTo-Json -Depth 40)+"`n")) } finally { $stream.Dispose() }
}
function Step([string]$Name,[string]$Program,[string[]]$Arguments) {
    Write-Host "Running $Name ..."
    $log = Join-Path $logsRoot "$Name.log"
    $status = 'failed'
    try {
        "program=$Program`narguments=$($Arguments | ConvertTo-Json -Compress)" | Set-Content -LiteralPath $log -Encoding utf8
        & $Program @Arguments >> $log 2>&1
        $code = $LASTEXITCODE
        "exit=$code" | Add-Content -LiteralPath $log -Encoding utf8
        if($code -eq 77 -and $Name -in @('original-kwin-portal-failure','rust-kwin-portal-failure','kwin-portal-failure-records')) {
            throw 'Portal matrix has no environment-skip result; preserve the unexpected exit as a failure'
        }
        if($code -eq 77){
            $status='missing_environment'
            Get-Content -LiteralPath $log -Tail 10 | ForEach-Object {Write-Host $_}
            throw "$Name requires a missing native environment; see $log"
        }
        if ($code -ne 0) {
            Get-Content -LiteralPath $log -Tail 50 | ForEach-Object { Write-Host $_ }
            throw "$Name exited with $code; see $log"
        }
        if($Name -eq 'custom-native-parity'){Assert-CustomNativeLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-metadata-parity'){Assert-CustomMetadataLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-api-parity'){Assert-CustomApiLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-reentry-parity'){Assert-CustomReentryLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-images-parity'){Assert-CustomImagesLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-raw-parity'){Assert-CustomRawLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-buffers-parity'){Assert-CustomBuffersLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-buffers-records'){Assert-CustomBufferRecordsLog (Get-Content -LiteralPath $log -Raw)}
        if($Name -eq 'custom-core-runtime-tree'){Assert-CustomRuntimeTree (Get-Content -LiteralPath $log -Raw)}
        if ($Name -eq 'rust-pipewire-consumer-control' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 4 passed; 0 failed; 1 ignored;') {
            throw 'Expected four portable consumer tests; the external source oracle remains explicitly ignored on this Linux entry'
        }
        if ($Name -eq 'rust-pipewire-row-ownership' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 4 passed; 0 failed; 0 ignored;') {
            throw 'Expected four checked row/storage tests, separate from native stream capture'
        }
        if ($Name -eq 'linux-none-original-caller') {
            $observed=[IO.File]::ReadAllText($log)
            if ([regex]::Matches($observed,'(?m)^original NoneScreencap cases=4 virtual_init=4 false_calls=8 inactive=4 virtual_deletes=4 caller_checks=12 held_callers=4 source_bodies_unchanged=1 native_captures=0 full_P1_acceptance=0 passed\r?$').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^original NoneScreencap .*').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^original None caller .*').Count -ne 4) {
                throw 'Expected the actual original None header/base and complete native caller preservation matrix'
            }
            for ($shape=0;$shape -lt 4;$shape++) {
                if ([regex]::Matches($observed,"(?m)^original None caller shape=$shape false_calls=2 header_and_full_pixels=unchanged held_after_virtual_delete=1\r?$").Count -ne 1) {
                    throw 'Missing or repeated original None caller shape'
                }
            }
        }
        if ($Name -eq 'original-pipewire-source-context' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire source cycles=20 captures=60 retained=60 passed') {
            throw 'Expected twenty original PipeWire source lifecycles and sixty retained images'
        }
        if($Name -eq 'original-pipewire-dma-stream' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire source cycles=20 captures=60 retained=60 passed'){
            throw 'Expected twenty original captures from the real kernel DMA producer with sixty retained BGR owners'
        }
        if($Name -eq 'rust-pipewire-dma-stream'){
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust PipeWire DMA stream cycles=20 captures=60 NULL_data=60 READ_START=60 READ_END=60 unmaps=60 retained=60 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected one real incoming NULL-data DMA test with paired native sync/unmap, queue and CPU owner checks'
            }
        }
        if($Name -eq 'original-pipewire-memfd-stream' -and -not (Get-Content -LiteralPath $log -Raw).Contains('original PipeWire MemFd NULL intervention cycles=20 captures=60 restored_all=1 retained=60 passed')) {
            throw 'Expected twenty original real MemFd streams with restored native pointers and retained known BGR'
        }
        if($Name -eq 'rust-pipewire-memfd-stream') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('native Rust PipeWire MemFd NULL intervention cycles=20 captures=60 owning_maps=60 unmaps=60 restored_all=1 retained=60 passed') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected one real MemFd callback intervention test with sixty owning maps/unmaps and native pointer restoration'
            }
        }
        if($Name -match '^(original|rust)-pipewire-format-(RGB|RGBA|RGBx|BGR|BGRA|BGRx|YUY2)$') {
            $side=$Matches[1];$format=$Matches[2]
            $observed=Get-Content -LiteralPath $log -Raw
            $expected=if($side -eq 'original'){"original PipeWire format=$format cycles=20 captures=60 retained=60 phases=2 passed"}else{"native Rust PipeWire format=$format cycles=20 captures=60 retained_raw=60 retained_bgr=60 phases=2 passed"}
            if(-not $observed.Contains($expected) -or ($side -eq 'rust' -and $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;')) {
                throw 'Expected complete real format stream captures, both fixture phases and retained owners'
            }
        }
        if ($Name -eq 'pipewire-fd-error-observation' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'connect_fd_early_error result=null errno=\d+ input_fd_open=[01] observed') {
            throw 'Expected the real native early-error descriptor observation'
        }
        if ($Name -eq 'pipewire-fd-add-io-error-observation' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'connect_fd_add_io_error result=null errno=\d+ input_fd_open=[01] after_teardown_fd_open=[01] intercepted_calls=1 observed') {
            throw 'Expected the actual native late connect_fd error with one explicit add_io interception'
        }
        if ($Name -eq 'native-joined-task-observation') {
            $observed=Get-Content -LiteralPath $log -Raw
            $memory=[regex]::Matches($observed,'(?m)^native joined-task memory samples=10000 absent_open=(\d+) absent_read=(\d+) exiting_with_memory=(\d+) exiting_without_memory=(\d+) nonexiting=(\d+) source_cleanup_acceptance=0 observed\r?$')
            if($observed -notmatch 'native joined-task observation cycles=10000 residual_enumerations=\d+ observed' -or $memory.Count -ne 1) {
                throw 'Expected one complete separate native join/proc memory observation; this is not source cleanup acceptance'
            }
            $total=0L
            for($index=1;$index -le 5;++$index) {
                $value=[long]$memory[0].Groups[$index].Value
                if($value -gt 10000){throw 'Joined-task observation count exceeds its actual iterations'}
                $total+=$value
            }
            if($total -ne 10000){throw 'Joined-task observation counts do not cover all iterations'}
        }
        if ($Name -eq 'native-task-diagnostic-fixtures' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'native task diagnostic empty_stream_failure=1 empty_file=1 bytes_file=1 missing_file=1 real_output_failure=1 passed') {
            throw 'Expected empty/readable/missing task inputs and a real diagnostic write-error control'
        }
        if($Name -eq 'rust-task-retirement-read-fixture') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('native Rust task observer exact_read_ESRCH=1 strict_read_rejected=1 known_joined_source=1 fresh_stat_and_directory_absent=1 real_live_path_refused=1 real_EMFILE_refused=1 fd_limit_restored=1 exact_fd_task_owners_closed=1 native_capture=0 passed') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected one isolated Rust procfs causal test: real held-stat ESRCH, live-path/EMFILE refusal and restored limits/owners'
            }
        }
        if($Name -eq 'rust-task-observer-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 5 passed; 0 failed; 0 ignored;') {
            throw 'Expected five Rust task read/parser/identity/absence and post-join memory controls'
        }
        if ($Name -eq 'pipewire-source-negotiation-pods' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire negotiation pods=3 saved') {
            throw 'Expected three original negotiation PODs'
        }
        if ($Name -eq 'pipewire-source-format-fixtures' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire format fixtures=28 formats=7 choice_none=14 linear_modifier=14 saved') {
            throw 'Expected twenty-eight original native POD/parser/conversion fixtures'
        }
        if($Name -eq 'pipewire-source-format-branches' -and -not (Get-Content -LiteralPath $log -Raw).Contains('original PipeWire parser branches cases=8 positive_media=8 positive_raw=8 fallback_last_property=8 accepted=5 unsupported=3 capture=0 saved')) {
            throw 'Expected eight actual original parser branch observations'
        }
        if($Name -eq 'rust-pipewire-format-branches') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('native PipeWire parser branches cases=8 differences=0 capture=0 recorded') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected one native parser branch regression and all eight source/Rust observations'
            }
        }
        if($Name -eq 'pipewire-format-branch-records' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'PipeWire parser branch records cases=8 source_rust_differences=[0-8] diagnostic_only=1 capture=0 full_P1_acceptance=0 verified') {
            throw 'Expected complete actual parser branch diagnostic records; native Rust parity stays a separate gate'
        }
        if($Name -in @('original-pipewire-thread-mobility-0','original-pipewire-thread-mobility-1')) {
            $mode=$Name.Substring($Name.Length-1)
            if(-not (Get-Content -LiteralPath $log -Raw).Contains("original PipeWire thread mobility mode=$mode objects=20 callers=3 captures=60 retained=60 fd_tasks_balanced=20 passed")) {
                throw 'Expected twenty original native lifecycles with three distinct live callers'
            }
        }
        if($Name -eq 'rust-pipewire-thread-mobility') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('native PipeWire thread mobility objects=40 callers=3 captures=120 retained=120 controlled_unwinds=40 fd_tasks_balanced=40 passed') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected one actual native Rust mobility test, forty lifecycles and controlled unwinds'
            }
        }
        if($Name -eq 'pipewire-thread-mobility-records' -and -not (Get-Content -LiteralPath $log -Raw).Contains('PipeWire thread mobility records lifecycles=80 captures=240 payloads=720 rust_unwinds=40 full_P1_acceptance=0 verified')) {
            throw 'Expected complete source/Rust caller/native generation, resource and pixel records'
        }
        if ($Name -eq 'pipewire-source-memfd-fixtures' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original MemFd helper cycles=20 maps=60 unmaps=60 retained_bgr=60 readonly_private=60 fd_preserved=60 passed') {
            throw 'Expected real original MemFd helper mappings, FD preservation and retained BGR owners'
        }
        if ($Name -eq 'rust-pipewire-memfd-helper') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native MemFd helper cycles=20 copies=60 owning_maps=60 observed_maps=60 unmap_calls=120 rejected=6 retained_rgb=60 retained_bgr=60 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected one native MemFd helper test with real map/unmap, six failures and sixty RGB/BGR owners'
            }
        }
        if($Name -eq 'rust-pipewire-dma-error-control' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'native non-DMA error control START=ENOTTY END=skipped borrowed_pointer=skips_sync invalid_layouts=5 passed'){
            throw 'Expected the real non-DMA ioctl failure and source borrowed-pointer controls'
        }
        if($Name -eq 'pipewire-source-dma-error-control' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original non-DMA error control START=ENOTTY borrowed_pointer=skips_sync retained_bytes=1 passed'){
            throw 'Expected compiled original helper non-DMA ioctl failure and existing-pointer observations'
        }
        if($Name -eq 'pipewire-source-dma-fixtures' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original DMA helper exporter=[12] cycles=20 maps=60 retained_bgr=60 fd_preserved=60 passed'){
            throw 'Expected original helper copies from a real kernel DMA exporter'
        }
        if($Name -eq 'rust-pipewire-kernel-dma-helper'){
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native kernel DMA helper cycles=20 copies=60 observation_maps=60 READ_START=120 READ_END=120 unmaps=120 retained_rgb=60 retained_bgr=60 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected real kernel DMA/CPU/OpenCV owners with paired sync and unmap evidence'
            }
        }
        if ($Name -eq 'rust-pipewire-native-formats') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native PipeWire format oracle cases=28 formats=7 conversions=24 bgr_reuses=4 exact_bytes=1 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected one native seven-format parser and exact OpenCV conversion/reuse comparison'
            }
        }
        if ($Name -eq 'rust-pipewire-daemon-shm') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust PipeWire daemon SHM cycles=20 captures=60 retained=60 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected one native driver test, twenty Rust daemon/SHM lifecycles and sixty retained raw RGB samples'
            }
        }
        if ($Name -eq 'rust-pipewire-supplied-fd-shm') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust PipeWire supplied fd SHM cycles=20 captures=60 retained=60 consumed_sockets_closed=20 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected twenty supplied-fd SHM lifecycles, sixty owning CPU images and twenty consumed sockets closed'
            }
        }
        if ($Name -eq 'rust-pipewire-opencv') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust PipeWire OpenCV cycles=20 captures=60 retained=60 conversion_unlock_witnesses=20 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected twenty OpenCV conversion/owner lifecycles, sixty held BGR images and twenty actual native unlock witnesses'
            }
        }
        if ($Name -eq 'linux-manager-source-model') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('Linux manager source model cases=960 source_body_comparisons=960 two_connects=1920 real_mat_outputs=1 scripted_backends=1 passed') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected 960 exact original-body/Rust manager scripts and one actual test'
            }
        }
        if($Name -in @('linux-config-rust-parity','linux-whole-config-rust-parity')) {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'Rust Linux config cases=946 accepted=426 rejected=520 exact_uuid_metadata=1 input_actions=0 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected all original/Rust configuration acceptance, UUID and metadata comparisons in one actual test'
            }
        }
        if($Name -eq 'linux-whole-none-source') {
            $observed=Get-Content -LiteralPath $log -Raw
            $expected=@(
                'whole_none method=0 cold_features=0 connected=1 active_features=3 inactive_features=3 reconstructed_features=3 actual_capture_none=1 actual_input_none=1 caller_unchanged=1 original_destroyed=1',
                'whole_none method=2 cold_features=0 connected=0 active_features=0 inactive_features=0 reconstructed_features=0 actual_capture_none=0 actual_input_none=0 caller_unchanged=1 original_destroyed=1',
                'whole_none method=18446744073709551615 cold_features=0 connected=0 active_features=0 inactive_features=0 reconstructed_features=0 actual_capture_none=0 actual_input_none=0 caller_unchanged=1 original_destroyed=1',
                'Original whole Linux None manager cases=3 active_NoneInput=1 active_features=3 original_class_api=1 caller_checks=3 native_connection=0 native_captures=0 input_actions=0 full_P1_acceptance=0 passed'
            )
            $actual=@($observed -split '\r?\n'|Where-Object {$_ -match '^(whole_none |Original whole Linux None manager |caller_caught_exception_hex=)'})
            if(($actual -join "`n") -cne ($expected -join "`n")){throw 'Expected complete original NoneInput owner/features and false-capture caller observations'}
        }
        if($Name -in @('linux-none-features-debug','linux-none-features-release')) {
            $observed=Get-Content -LiteralPath $log -Raw
            $prefix='(?m)^(?:test backends::linux::manager::native::tests::none_and_unimplemented_native_methods_do_not_enter_capture_adapters \.\.\. )?'
            if([regex]::Matches($observed,'(?m)^test result:').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;').Count -ne 1 -or
                [regex]::Matches($observed,$prefix+'Rust native Linux None cases=').Count -ne 1 -or
                [regex]::Matches($observed,$prefix+'Rust native Linux None cases=3 cold_features=0 active_NoneInput_features=3 unsupported_features=0 reconstructed=3 inactive_preserved=1 false_output_preserved=1 adapter_calls=0 passed\r?$').Count -ne 1) {
                throw 'Expected actual native NoneInput feature/state regression in one exact test'
            }
        }
        if($Name -eq 'linux-whole-config-source-corpus') {
            $observed=Get-Content -LiteralPath $log -Raw
            if([regex]::Matches($observed,'(?m)^Original whole Linux manager configuration ').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^Original Linux configuration ').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^Original whole Linux manager configuration cases=946 complete_class_api=1 no_native_connection=1 input_actions=0 full_P1_acceptance=0 passed\r?$').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^Original Linux configuration cases=946 accepted=426 rejected=520 normalized_roundtrips=426 no_native_connection=1 passed\r?$').Count -ne 1) {
                throw 'Expected complete whole-class original configuration and RTTI corpus'
            }
        }
        if($Name -eq 'linux-whole-config-rust-parity') {
            $prefix='(?m)^(?:test backends::linux::manager::configuration::tests::matches_original_configuration_creation_uuid_and_metadata_bytes \.\.\. )?'
            if([regex]::Matches($observed,'(?m)^test result:').Count -ne 1 -or
                [regex]::Matches($observed,'(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;').Count -ne 1 -or
                [regex]::Matches($observed,$prefix+'Rust Linux config cases=').Count -ne 1 -or
                [regex]::Matches($observed,$prefix+'Rust Linux config cases=946 accepted=426 rejected=520 exact_uuid_metadata=1 input_actions=0 passed\r?$').Count -ne 1) {
                throw 'Incomplete or repeated whole-class Rust configuration comparison'
            }
        }
        if($Name -eq 'linux-compat-rust-parity'){
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('Rust Linux compat cases=120 exact_uuid_metadata=1 portal_call_order=1 created=38 source_open_failure_residuals=6 native_authorization=0 passed') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected the full original/Rust compatibility constructor and host-order comparison, not a zero-test run'
            }
        }
        if($Name -eq 'linux-manager-host-input-model') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('Linux manager host boundary cases=2304 source_body_comparisons=2304 two_connects=4608 retained_capture_and_features=1 input_actions=0 native_capture=0 passed') -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected one actual 2304-case original/Rust host construction/init/features/capture forwarding comparison'
            }
        }
        if($Name -eq 'linux-config-manager-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 11 passed; 0 failed; 2 ignored;') {
            throw 'Expected eleven manager controls; the two explicit source models remain ignored on this default filter'
        }
        if($Name -eq 'manager-macos-metadata') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'manager metadata kind=macos cases=66 exact_original_json=1 input_none=1 no_native_connection=1 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected actual original-body/Rust macOS metadata comparison on this portable Linux model entry'
            }
            & $ConfigureTool upstream-metadata macos (Join-Path $env:MFS_P1_MANAGER_METADATA_ARTIFACTS macos) >> $log 2>&1
            if($LASTEXITCODE -ne 0){throw 'Metadata source/Rust artifact validation failed'}
        }
        if ($Name -eq 'rust-pipewire-manager-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 4 passed; 0 failed; 0 ignored;') {
            throw 'Expected native None/unimplemented, missing provider, plain and hosted unsafe-FD boundary controls'
        }
        if($Name -in @('original-pipewire-manager-host','rust-pipewire-manager-host','rust-kwin-native-host')) {
            $observed=Get-Content -LiteralPath $log -Raw
            $expected=$Name.StartsWith('original-') ?
                'original PipeWire host manager objects=20 init_true=7 init_false=7 init_error=6 captures=60 retained=60 helper_ended_before_connect=20 native_closed_before_host=20 host_fds_closed=40 input_actions=0 passed' :
                'native Rust PipeWire host manager objects=20 init_true=7 init_false=7 init_error=6 captures=60 retained_raw=60 retained_bgr=60 helper_ended_before_connect=20 native_closed_before_host=20 host_fds_closed=40 input_actions=0 passed'
            if(-not $observed.Contains($expected) -or ($Name.StartsWith('rust-') -and $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;')) {
                throw 'Expected real native host true/false/error capture, retained pixels and native-before-host close'
            }
        }
        if($Name -in @('original-pipewire-host-offer','rust-pipewire-host-offer')) {
            $observed=Get-Content -LiteralPath $log -Raw
            $prefix=$Name.StartsWith('original-') ? 'original' : 'native Rust'
            $expected=$prefix+' PipeWire host offer cycles=20 create_errors=20 native_init=0 retained_identity=20 unconsumed_after_destroy=20 fixture_reclaims=20 capture=0 passed'
            if(-not $observed.Contains($expected) -or ($Name.StartsWith('rust-') -and $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;')) {
                throw 'Expected exact native pre-init host error, retained socket identity and separately labelled fixture reclaim'
            }
        }
        if($Name -in @('original-kwin-portal-failure','rust-kwin-portal-failure','kwin-portal-failure-records')) {
            $observed=Get-Content -LiteralPath $log -Raw
            $summary=switch($Name) {
                'original-kwin-portal-failure' {'original KWin Portal failures cases=100 stages=5 residual_sockets=100 fixture_reclaims=100 manager_created=0 native_capture=0 full_P1_acceptance=0 passed'}
                'rust-kwin-portal-failure' {'Rust KWin Portal failures cases=100 stages=5 residual_sockets=100 fixture_reclaims=100 manager_created=0 native_capture=0 full_P1_acceptance=0 passed'}
                'kwin-portal-failure-records' {'KWin Portal failure records cases=200 stages=5 residual_sockets=200 fixture_reclaims=200 files=204 native_capture=0 full_P1_acceptance=0 passed'}
            }
            $prefix=switch($Name) {
                'original-kwin-portal-failure' {'original KWin Portal failures'}
                'rust-kwin-portal-failure' {'Rust KWin Portal failures'}
                'kwin-portal-failure-records' {'KWin Portal failure records'}
            }
            $leader=if($Name -eq 'rust-kwin-portal-failure') {
                '(?:test backends::linux::manager::compat::tests::portal_failures::retains_host_socket_after_each_portal_failure_before_manager_construction \.\.\. )?'
            }else{''}
            if([regex]::Matches($observed,'(?m)^'+$leader+[regex]::Escape($summary)+'\r?$').Count -ne 1 -or
               [regex]::Matches($observed,'(?m)^'+$leader+[regex]::Escape($prefix)+'.*\r?$').Count -ne 1 -or
               ($Name -eq 'rust-kwin-portal-failure' -and
                ([regex]::Matches($observed,'(?m)^test result:').Count -ne 1 -or
                 $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out; finished in [^\r\n]+\r?$'))) {
                throw 'Expected all five pre-manager Portal failures, real residual socket identities and separately labelled fixture cleanup'
            }
        }
        if($Name -eq 'original-kwin-native-host' -and -not (Get-Content -LiteralPath $log -Raw).Contains('original KWin ABI objects=20 host_true_false_error=7,7,6 captures=60 retained=60 exact_compat_create_destroy=20 real_virtual_handles=20 helper_call_order=20 duplicated_socket_identity=20 input_actions=0 passed')) {
            throw 'Expected exact original KWin C create/destroy, real public virtual handles and native host captures'
        }
        if($Name -match '^(original|rust)-pipewire-whole-(daemon|fd)$') {
            $observed=Get-Content -LiteralPath $log -Raw
            $mode=$Matches[2]
            $sessions=$mode -eq 'fd' ? 20 : 40
            $images=$sessions*3
            $owners=20*(1+5*($sessions/20))
            $expected=$Name.StartsWith('original-') ?
                "original whole PipeWire manager mode=$mode objects=20 sessions=$sessions captures=$images held=$images owner_observations=$owners original_class_api=1 real_NoneInput=1 active_features=3 resources_balanced=1 input_actions=0 full_P1_acceptance=0 passed" :
                "Rust whole PipeWire comparison mode=$mode objects=20 sessions=$sessions captures=$images held_bgr=$images held_rgb=$images active_features=3 resources_balanced=1 input_actions=0 full_P1_acceptance=0 passed"
            if([regex]::Matches($observed,'(?m)^'+[regex]::Escape($expected)+'\r?$').Count -ne 1 -or
                ($Name.StartsWith('rust-') -and [regex]::Matches($observed,'(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;').Count -ne 1)) {
                throw 'Expected complete native whole-manager invocation, features, pixels and lifetimes'
            }
        }
        if($Name -in @('original-pipewire-manager-daemon','original-pipewire-manager-fd','rust-pipewire-manager-daemon','rust-pipewire-manager-fd')) {
            $observed=Get-Content -LiteralPath $log -Raw
            $fdMode=$Name.EndsWith('-fd')
            $mode=$fdMode ? 'fd' : 'daemon'
            $sessions=$fdMode ? 20 : 40
            $images=$fdMode ? 60 : 120
            $closed=$fdMode ? 20 : 0
            $expected=$Name.StartsWith('original-') ?
                "original PipeWire manager mode=$mode objects=20 sessions=$sessions captures=$images retained=$images consumed_sockets_closed=$closed resources_balanced=1 passed" :
                "native Rust PipeWire manager mode=$mode objects=20 sessions=$sessions captures=$images retained_raw=$images retained_bgr=$images consumed_sockets_closed=$closed resources_balanced=1 passed"
            if(-not $observed.Contains($expected) -or ($Name.StartsWith('rust-') -and $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;')) {
                throw 'Expected the complete independent native manager capture, retained-image and resource checks'
            }
            if($Name.StartsWith('original-') -and -not $observed.Contains('manager Mat control shallow_output_reuse_mutates_alias=1 moved_owner_preserved=1 passed')) {
                throw 'Expected the real OpenCV shallow-alias/moved-owner premise control'
            }
        }
        if($Name -in @('original-pipewire-sustained','rust-pipewire-sustained')) {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'PipeWire sustained warmup=3 steady_ns=(\d+) calls=(\d+) retained=4 errors=0 resources_balanced=1 passed' -or
                [uint64]$Matches[1] -lt 120000000000 -or [uint64]$Matches[2] -lt 4) {
                throw 'Expected at least 120 steady seconds and bounded retained native images'
            }
            $rust=$Name -eq 'rust-pipewire-sustained'
            if($rust -and $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') { throw 'Expected one actual native sustained Rust test' }
            $traceDirectory=Join-Path ($rust ? $bgrArtifacts : $referenceArtifacts) sustained
            & $ConfigureTool upstream-trace pipewire ($rust ? 'rust' : 'source') $traceDirectory >> $log 2>&1
            if($LASTEXITCODE -ne 0) { throw 'Sustained trace/pixel/resource validation failed; see native log' }
        }
        if ($Name -eq 'native-task-read-exit-fixture' -and -not (Get-Content -LiteralPath $log -Raw).Contains('native task read-exit opened_live=1 joined_before_read=1 path_absent=1 read=ESRCH strict_observer_rejected=1 fresh_open_absent=1 resources_balanced=1 passed')) {
            throw 'Expected native read ESRCH after controlled task exit and the unchanged strict observer rejection'
        }
        if($Name -eq 'native-task-retirement-read-fixture') {
            $observed=Get-Content -LiteralPath $log -Raw
            if(-not $observed.Contains('native task retirement-read exact_ESRCH=1 joined_before_read=1 fresh_stat_and_directory_absent=1 real_live_path_refused=1 real_EMFILE_refused=1 synthetic_metadata_negatives=24 fd_limit_restored=1 both_joined_controls_absent=1 resources_balanced=1 passed') -or
                -not $observed.Contains('task retirement-memory controls synthetic_cases=3 real_gated_nonexiting_refused=1 memory_fields_observed_only=1 passed')) {
                throw 'Expected actual held-stat ESRCH, strict absence/live/EMFILE refusals and both memory/identity metadata controls'
            }
        }
        if ($Name -eq 'original-pipewire-source-reinit' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire reinit objects=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed') {
            throw 'Expected twenty original same-object lifecycles, forty sessions and 120 held BGR images'
        }
        if ($Name -eq 'rust-pipewire-supplied-fd-reinit') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust PipeWire reinit objects=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected twenty Rust same-object lifecycles, forty sessions and 120 held raw RGB images'
            }
        }
        if ($Name -eq 'original-pipewire-source-fd-retry' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire fd-retry objects=20 injected_failures=20 proven_live_retries=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed') {
            throw 'Expected twenty complete original init failures, same-socket retries and 120 held BGR images'
        }
        if ($Name -eq 'rust-pipewire-supplied-fd-retry') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native Rust PipeWire fd-retry objects=20 injected_failures=20 proven_live_retries=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected twenty complete Rust init failures, same-socket retries and 120 held RGB images'
            }
        }
        if ($Name -eq 'pipewire-opencv-private-abi') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'native OpenCV private ABI failures=6 exception=1 independent_bgr_owner=1 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                throw 'Expected actual OpenCV ABI/failure-output/exception and independent-owner checks'
            }
        }
        $status = 'passed'
    } finally {
        $checks.Add([ordered]@{name=$Name;status=$status;log=(Artifact $log)})
        Write-Host "${Name}: $status"
    }
}
function Capture-TestProgram([string]$Name) {
    Step $Name cargo @('test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','--no-run','--message-format=json')
    $programs=@(Get-Content -LiteralPath (Join-Path $logsRoot "$Name.log")|Where-Object{$_ -match '^\{'}|ForEach-Object{$_|ConvertFrom-Json}|Where-Object{ $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'maafw_screencap' -and $_.profile.test -and $_.executable }|ForEach-Object{$_.executable})
    if($programs.Count -ne 1 -or -not(Test-Path -LiteralPath $programs[0] -PathType Leaf)){throw 'Cargo did not identify exactly one capture libtest executable'}
    return $programs[0]
}
function Prepare-DmaKernelFixture {
    $log=Join-Path $logsRoot dma-kernel-environment.log
    "scope=owned acceptance fixture only; no local/self-hosted module or permission changes"|Set-Content -LiteralPath $log -Encoding utf8
    # Root preparation is restricted to the ephemeral GitHub-hosted Linux VM.
    # Use only its installed kernel module; no module download/build.
    if($env:GITHUB_ACTIONS -eq 'true' -and $env:RUNNER_ENVIRONMENT -eq 'github-hosted'){
        if(-not(Test-Path -LiteralPath '/dev/udmabuf')){
            $modinfo=Get-Command modinfo -ErrorAction SilentlyContinue
            if($modinfo){
                & $modinfo.Source -F filename udmabuf >> $log 2>&1
                if($LASTEXITCODE -eq 0){
                    & sudo -n modprobe udmabuf >> $log 2>&1
                    "module_load_exit=$LASTEXITCODE"|Add-Content -LiteralPath $log -Encoding utf8
                }
            }
        }
        # Hosted images may already load udmabuf as root:kvm. Its presence
        # does not establish access by this isolated acceptance process.
        if(Test-Path -LiteralPath '/dev/udmabuf'){
            $fixtureUid=(& id -u|Out-String).Trim()
            if($LASTEXITCODE -ne 0 -or $fixtureUid -notmatch '^\d+$'){throw 'Cannot identify fixture user'}
            & sudo -n chown -- $fixtureUid /dev/udmabuf >> $log 2>&1
            "device_owner_exit=$LASTEXITCODE"|Add-Content -LiteralPath $log -Encoding utf8
        }
    }
    foreach($path in @('/dev/udmabuf','/dev/dma_heap/system')){
        "path=$path present=$(Test-Path -LiteralPath $path)"|Add-Content -LiteralPath $log -Encoding utf8
        if(Test-Path -LiteralPath $path){ & stat $path >> $log 2>&1 }
    }
    Save $log
}
function StopOwned([string]$Name, $Process, $OutputTask, $ErrorTask) {
    if (-not $Process) { return }
    $log = Join-Path $logsRoot "$Name-stop.log"
    $status = 'passed'
    try {
        $unexpectedExit = $Process.HasExited
        $forced = $false
        if (-not $unexpectedExit) {
            & /bin/kill -TERM $Process.Id
            if($LASTEXITCODE -ne 0 -and -not $Process.HasExited) { $forced=$true; $Process.Kill() }
            if(-not $Process.WaitForExit(10000)) { $Process.Kill(); $Process.WaitForExit(); $forced=$true }
        }
        if ($unexpectedExit -or $forced -or $Process.ExitCode -ne 0) { $status='failed' }
        "owned_pid=$($Process.Id), exit=$($Process.ExitCode), unexpected_exit=$unexpectedExit, forced=$forced" | Set-Content -LiteralPath $log -Encoding utf8
        foreach($item in @(@("$Name-stdout.log",$OutputTask),@("$Name-stderr.log",$ErrorTask))) {
            $path=Join-Path $logsRoot $item[0]
            $item[1].GetAwaiter().GetResult() | Set-Content -LiteralPath $path -Encoding utf8
            Save $path
        }
    } catch {
        $status='failed'
        $_.Exception.ToString() | Add-Content -LiteralPath $log -Encoding utf8
    } finally {
        $checks.Add([ordered]@{name="$Name-stop";status=$status;log=(Artifact $log)})
        $Process.Dispose()
    }
}

. (Join-Path $PSScriptRoot prepare-linux-llvm.ps1)
try {
    if (-not $IsLinux -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64') { throw 'Native Linux x64 is required' }
    Step host-identity uname @('-a')
    Initialize-LinuxLlvm -ReferenceRoot $ReferenceRoot -DepsRoot $DepsRoot -RunRoot $RunRoot -BindingGenerator
    Step compiler-identity $env:CC @('--version')
    Step clang-identity clang @('--version')
    Step python-identity python3 @('--version')
    $packages = @{}
    $pythonTools=@('pip','meson','ninja')
    if($SourceReference){$pythonTools+='cmake'}
    foreach ($name in $pythonTools) {
        $path = Join-Path $inputRoot "$name-release.json"
        Step "$name-latest" curl @('--fail','--location','--silent','--show-error','--connect-timeout','15','--max-time','60','-o',$path,"https://pypi.org/pypi/$name/json")
        Save $path
        $packages[$name] = (Get-Content -LiteralPath $path -Raw | ConvertFrom-Json).info.version
    }
    $environmentKey = (($packages.Values | Sort-Object) -join '-')
    $venv = Join-Path $DepsRoot "python/pipewire-$environmentKey"
    Step python-environment python3 @('-m','venv',$venv)
    $python = Join-Path $venv bin/python
    $pipReport = Join-Path $inputRoot pip-install.json
    Step python-tools $python (@('-m','pip','install','--upgrade','--report',$pipReport)+$pythonTools)
    Save $pipReport
    $installedPath = Join-Path $inputRoot python-packages.json
    & $python -m pip list --format=json | Set-Content -LiteralPath $installedPath -Encoding utf8
    if ($LASTEXITCODE -ne 0) { throw 'Installed Python package query failed' }
    Save $installedPath
    $installed = Get-Content -LiteralPath $installedPath -Raw | ConvertFrom-Json
    foreach ($name in $packages.Keys) {
        if (($installed | Where-Object name -EQ $name).version -ne $packages[$name]) { throw "Python tool $name differs from latest formal release" }
    }
    $wheels = Join-Path $inputRoot python-wheels
    New-Item -ItemType Directory -Path $wheels | Out-Null
    Step python-offline-inputs $python (@('-m','pip','download','--only-binary=:all:','--dest',$wheels)+$pythonTools)
    Get-ChildItem -LiteralPath $wheels -File | ForEach-Object { Save $_.FullName }
    $env:PATH = (Join-Path $venv bin) + ':' + $env:PATH

    $releasePath = Join-Path $inputRoot pipewire-release.json
    Step pipewire-latest curl @('--fail','--location','--silent','--show-error','--connect-timeout','15','--max-time','60','-o',$releasePath,'https://gitlab.freedesktop.org/api/v4/projects/pipewire%2Fpipewire/releases/permalink/latest')
    Save $releasePath
    $release = Get-Content -LiteralPath $releasePath -Raw | ConvertFrom-Json
    $identity=Read-PipeWireReleaseIdentity $release
    if($identity.needs_tag){
        $tagPath=Join-Path $inputRoot pipewire-release-tag.json
        $tagName=[uri]::EscapeDataString($identity.tag)
        Step pipewire-release-tag curl @('--fail','--location','--silent','--show-error','--connect-timeout','15','--max-time','60','-o',$tagPath,"https://gitlab.freedesktop.org/api/v4/projects/pipewire%2Fpipewire/repository/tags/$tagName")
        Save $tagPath
        $identity=Read-PipeWireReleaseIdentity $release (Get-Content -LiteralPath $tagPath -Raw | ConvertFrom-Json)
    }
    if($identity.needs_tag){throw 'Latest PipeWire release tag did not resolve a complete commit'}
    $commit=$identity.commit
    $identityPath=Join-Path $inputRoot pipewire-source-identity.json
    Json $identityPath ([ordered]@{release_tag=$identity.tag;commit=$commit;commit_source=$identity.source;release_commit=$identity.release_commit;scope='Same latest released tag; no older release or cached commit fallback. Git checkout still must match this exact resolved commit.'})
    Save $identityPath
    $source = Join-Path $DepsRoot "native-sources/PipeWire-$commit"
    if (-not(Test-Path -LiteralPath $source)) {
        Step pipewire-source git @('-c','core.autocrlf=false','clone','--depth=1','--branch',$release.tag_name,'https://gitlab.freedesktop.org/pipewire/pipewire.git',$source)
    }
    $actual = (& git -C $source rev-parse HEAD | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -ne $commit) { throw 'PipeWire checkout identity differs from the release' }
    $dirty = & git -C $source status --porcelain --untracked-files=all
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Cached PipeWire source was modified' }
    $archive = Join-Path $inputRoot PipeWire-source.tar
    Step pipewire-source-archive git @('-C',$source,'archive','--format=tar','--output',$archive,$commit)
    Save $archive
    $configuration = Join-Path $inputRoot native-configuration.json
    $options = @('--buildtype=release','--libdir=lib','--wrap-mode=nodownload','-Dauto_features=disabled','-Ddocs=disabled','-Dman=disabled','-Dtests=disabled','-Dexamples=disabled','-Ddbus=disabled','-Dflatpak=disabled','-Dpipewire-jack=disabled','-Dpipewire-v4l2=disabled','-Dsession-managers=[]','-Drlimits-install=false','-Dsupport=enabled','-Dvideotestsrc=disabled','-Dvideoconvert=enabled')
    Json $configuration ([ordered]@{pipewire_commit=$commit;pipewire_release=$release.tag_name;python_packages=$packages;compiler=(& $env:CC --version | Out-String);options=$options;scope='Owned native C producer and Rust capture client feasibility; no portal/desktop acceptance or migrated backend.'})
    Save $configuration
    $key = (Get-FileHash -LiteralPath $configuration -Algorithm SHA256).Hash.ToLowerInvariant()
    $native = Join-Path $DepsRoot "native/pipewire-$key"
    $build = Join-Path $native build
    $prefix = Join-Path $native install
    $setup = @('setup',$build,$source,"--prefix=$prefix") + $options
    if(Test-Path -LiteralPath (Join-Path $build meson-private/coredata.dat)) { $setup += '--reconfigure' }
    Step pipewire-configure meson $setup
    Step pipewire-build meson @('compile','-C',$build,'-j','4')
    Step pipewire-install meson @('install','-C',$build)
    $savedOptions = Join-Path $inputRoot meson-build-options.json
    Copy-Item -LiteralPath (Join-Path $build meson-info/intro-buildoptions.json) -Destination $savedOptions
    Save $savedOptions
    $nativeArtifacts = Join-Path $inputRoot native-artifacts
    New-Item -ItemType Directory -Path $nativeArtifacts | Out-Null
    foreach($path in @((Join-Path $prefix lib/pkgconfig/libpipewire-0.3.pc),(Join-Path $prefix lib/pkgconfig/libspa-0.2.pc),(Join-Path $prefix bin/pipewire),(Join-Path $prefix bin/pw-dump))) {
        $saved=Join-Path $nativeArtifacts (Split-Path -Leaf $path)
        Copy-Item -LiteralPath $path -Destination $saved
        Save $saved
    }
    Get-ChildItem -LiteralPath (Join-Path $prefix lib) -File -Filter 'libpipewire-0.3.so.*' | Where-Object { -not $_.LinkType } | ForEach-Object {
        $saved=Join-Path $nativeArtifacts $_.Name
        Copy-Item -LiteralPath $_.FullName -Destination $saved
        Save $saved
    }
    $env:PKG_CONFIG_PATH = (Join-Path $prefix lib/pkgconfig) + ':' + $env:PKG_CONFIG_PATH
    $env:LD_LIBRARY_PATH = (Join-Path $prefix lib) + ':' + $env:LD_LIBRARY_PATH
    $env:SPA_PLUGIN_DIR = Join-Path $prefix lib/spa-0.2
    $env:PIPEWIRE_MODULE_DIR = Join-Path $prefix lib/pipewire-0.3
    $env:PIPEWIRE_CONFIG_DIR = Join-Path $prefix share/pipewire
    $env:PATH = (Join-Path $prefix bin) + ':' + $env:PATH
    Step native-version pkg-config @('--modversion','libpipewire-0.3')
    $version = (& pkg-config --modversion libpipewire-0.3 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $version -ne $release.tag_name) { throw 'pkg-config selected a different PipeWire library' }
    $env:MFS_P0_PIPEWIRE_VERSION = $version
    . (Join-Path $PSScriptRoot configure-prototype.ps1) -ConfigureTool $ConfigureTool -RustBatch $RustBatch -ReferenceRoot $ReferenceRoot -Family pipewire
    Save $env:MFS_BUILD_CONFIGURATION
    # Compile the private consumer on native Linux and run its portable unit
    # cases separately from the real P0 stream probe below. No native driver
    # or source oracle is claimed by these four tests.
    Step rust-pipewire-consumer-control cargo @('test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::tests::','--','--test-threads=1','--nocapture')
    Step rust-pipewire-row-ownership cargo @('test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::buffer::tests::','--','--test-threads=1','--nocapture')
    $fixtureBinary = Join-Path $native mfs-p0-pipewire-source
    Step native-source-build $env:CC @('-std=c11','-D_GNU_SOURCE','-O2','-Wall','-Wextra','-Werror',"-I$(Join-Path $prefix include/pipewire-0.3)","-I$(Join-Path $prefix include/spa-0.2)",(Join-Path $productRoot tests/fixtures/pipewire-source.c),"-L$(Join-Path $prefix lib)",'-lpipewire-0.3','-pthread','-fuse-ld=lld','-o',$fixtureBinary)
    $savedFixture = Join-Path $nativeArtifacts mfs-p0-pipewire-source
    Copy-Item -LiteralPath $fixtureBinary -Destination $savedFixture
    Save $savedFixture
    if($SourceReference){
        $routeBinary=Join-Path $native mfs-p1-pipewire-route
        Step native-route-supervisor-build $env:CC @('-std=c11','-D_GNU_SOURCE','-O2','-Wall','-Wextra','-Werror',"-I$(Join-Path $prefix include/pipewire-0.3)","-I$(Join-Path $prefix include/spa-0.2)",(Join-Path $productRoot tests/fixtures/pipewire-route.c),"-L$(Join-Path $prefix lib)",'-lpipewire-0.3','-pthread','-fuse-ld=lld','-o',$routeBinary)
        $saved=Join-Path $nativeArtifacts mfs-p1-pipewire-route
        Copy-Item -LiteralPath $routeBinary -Destination $saved
        Save $saved
        $env:MFS_P1_PIPEWIRE_LIBRARY=(& readlink -f (Join-Path $prefix lib/libpipewire-0.3.so)|Out-String).Trim()
        if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $env:MFS_P1_PIPEWIRE_LIBRARY -PathType Leaf)){throw 'Cannot identify the selected native PipeWire library'}
        . (Join-Path $PSScriptRoot prepare-pipewire-reference.ps1)
        $sourceExecutable=Prepare-PipeWireReference $UpstreamRoot
        try { Step pipewire-opencv-private-abi timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::validates_opencv_abi_failures_and_owner_transfer','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $referenceArtifacts=Join-Path $RunRoot linux-x64/reference-u/artifacts
        New-Item -ItemType Directory -Path $referenceArtifacts -Force|Out-Null
        try { Step linux-none-original-caller $env:MFS_P1_LINUX_NONE_REFERENCE @() }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        . (Join-Path $PSScriptRoot prepare-custom-unix-reference.ps1)
        try { Test-CustomUnixReference $UpstreamRoot $referenceArtifacts }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        Json (Join-Path $referenceArtifacts geometry.json) ([ordered]@{width=28;height=24;stride=84;format='BGR';pattern='asymmetric RGB bars with row/column/channel/producer-serial xor; RGB(0,0) holds 24-bit serial';scope='Fixture serial/retention observed; original API capture time and replay status are not exposed. Repeated markers are diagnostic, not rejected.'})
        $podDirectory=Join-Path $referenceArtifacts negotiation
        $rustArtifacts=Join-Path $RunRoot linux-x64/rust-shm/artifacts
        $fdArtifacts=Join-Path $RunRoot linux-x64/rust-fd/artifacts
        $bgrArtifacts=Join-Path $RunRoot linux-x64/rust-opencv/artifacts
        New-Item -ItemType Directory -Path $rustArtifacts,$fdArtifacts,$bgrArtifacts -Force|Out-Null
        $env:MFS_P1_PIPEWIRE_RUST_ARTIFACTS=$rustArtifacts
        $env:MFS_P1_PIPEWIRE_FD_ARTIFACTS=$fdArtifacts
        $env:MFS_P1_PIPEWIRE_BGR_ARTIFACTS=$bgrArtifacts
        $env:MFS_P1_PIPEWIRE_POD_ORACLE=$podDirectory
        $formatDirectory=Join-Path $referenceArtifacts formats
        $env:MFS_P1_PIPEWIRE_FORMAT_ORACLE=$formatDirectory
        $memfdDirectory=Join-Path $referenceArtifacts memfd
        $env:MFS_P1_PIPEWIRE_MEMFD_ORACLE=$memfdDirectory
        $dmaDirectory=Join-Path $referenceArtifacts dma
        $env:MFS_P1_PIPEWIRE_DMA_ORACLE=$dmaDirectory
        Json (Join-Path $rustArtifacts geometry.json) ([ordered]@{width=28;height=24;stride=84;format='RGB';scope='Raw native driver probe; RawAudit preserves RGB and does not implement OpenCV conversion or source/BGR parity.'})
        Json (Join-Path $fdArtifacts geometry.json) ([ordered]@{width=28;height=24;stride=84;format='RGB';scope='Explicit unsafe supplied private socket success path, twenty consumed descriptors closed after native teardown; no Portal authorization, generic failure ownership or safe host FD contract is established.'})
        Json (Join-Path $bgrArtifacts geometry.json) ([ordered]@{width=28;height=24;stride=84;format='BGR';scope='Actual C++/OpenCV conversion after native unlock; twenty callback-progress witnesses add diagnostic waiting, not source latency parity. Native opaque Mat/loader mobility and full all-format/fault parity remain pending.'})
        try { Step pipewire-source-negotiation-pods $sourceExecutable @('--pods',$podDirectory) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-source-format-fixtures $sourceExecutable @('--format-fixtures',$formatDirectory) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-native-formats timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::matches_original_format_pods_and_padded_conversion_bytes','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $formatBranches=Join-Path $referenceArtifacts format-branches
        $env:MFS_P1_PIPEWIRE_FORMAT_BRANCHES=$formatBranches
        New-Item -ItemType Directory -Path $formatBranches -Force|Out-Null
        try { Step pipewire-source-format-branches $sourceExecutable @('--format-branches',(Join-Path $formatBranches source)) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-pipewire-format-branches timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::format_branches::matches_original_standard_result_and_fallback_selection','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step pipewire-format-branch-records $ConfigureTool @('upstream-trace','pipewire-format-branches',$formatBranches) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step pipewire-source-memfd-fixtures timeout @('--signal=TERM','--kill-after=5s','60s',$sourceExecutable,'--memfd-fixtures',$memfdDirectory) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-memfd-helper timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::mapping::matches_original_memfd_maps_and_retains_cpu_images_after_unmap','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-dma-error-control timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::dma::preserves_source_non_dma_ioctl_failure_and_borrowed_pointer_selection','--','--exact','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-source-dma-error-control timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--dma-error-control') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Prepare-DmaKernelFixture }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-source-dma-fixtures timeout @('--signal=TERM','--kill-after=5s','60s',$sourceExecutable,'--dma-fixtures',$dmaDirectory) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try {
            $dmaTest=Capture-TestProgram rust-pipewire-dma-helper-build
            Step rust-pipewire-kernel-dma-helper timeout @('--signal=TERM','--kill-after=5s','180s',$dmaTest,'backends::linux::pipewire::native::tests::opencv::dma::matches_original_kernel_dma_maps_and_retains_independent_cpu_images','--exact','--ignored','--test-threads=1','--nocapture')
        } catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step native-task-diagnostic-fixtures timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--task-diagnostic-fixtures',(Join-Path $referenceArtifacts task-diagnostic)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step native-task-read-exit-fixture timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--task-read-exit-fixture',(Join-Path $referenceArtifacts task-read-exit)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step native-task-retirement-read-fixture timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--task-retirement-read-fixture',(Join-Path $referenceArtifacts task-retirement-read)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try {
            $env:MFS_P1_RUST_TASK_OBSERVER_ARTIFACTS=Join-Path $referenceArtifacts rust-task-retirement-read
            Step rust-task-retirement-read-fixture timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::task_observer::native_fixture::validates_joined_read_esrch_without_accepting_live_or_unobservable_paths','--','--exact','--ignored','--test-threads=1','--nocapture')
        } catch { $failed=$true; Write-Host $_.Exception.Message }
        # The enclosing finally archives all referenceArtifacts, including a
        # partial causal fixture on failure. Do not register the same file twice.
        try { Step rust-task-observer-controls timeout @('--signal=TERM','--kill-after=5s','30s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::task_observer::tests::','--','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $offerMatrix=Join-Path $referenceArtifacts manager-host-offer-matrix
        New-Item -ItemType Directory -Path $offerMatrix|Out-Null
        $env:MFS_P1_PIPEWIRE_OFFER_ARTIFACTS=Join-Path $offerMatrix rust
        try { Step original-pipewire-host-offer timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--manager-host-offer',(Join-Path $offerMatrix source)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-host-offer timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::manager::native::offer_tests::retains_the_unconsumed_offer_after_host_construction_error_until_fixture_reclaim','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-host-offer-records $ConfigureTool @('upstream-trace','pipewire-host-offer',$offerMatrix) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $managerTest=$null
        try {
            $env:MFS_P1_LINUX_MANAGER_MODEL_ARTIFACTS=Join-Path $referenceArtifacts manager-model
            $managerTest=Capture-TestProgram linux-manager-model-rust-build
            Step linux-manager-source-model timeout @('--signal=TERM','--kill-after=5s','180s',$managerTest,'backends::linux::manager::tests::matches_fixed_source_manager_reconnect_forwarding_and_uuid','--exact','--ignored','--test-threads=1','--nocapture')
        } catch { $failed=$true; Write-Host $_.Exception.Message }
        if($managerTest){
            try {
                $env:MFS_P1_LINUX_HOST_INPUT_MODEL_ARTIFACTS=Join-Path $referenceArtifacts manager-host-input-model
                Step linux-manager-host-input-model timeout @('--signal=TERM','--kill-after=5s','180s',$managerTest,'backends::linux::manager::tests::host_boundary::matches_original_host_input_order_failures_features_and_retained_capture','--exact','--ignored','--test-threads=1','--nocapture')
            } catch { $failed=$true; Write-Host $_.Exception.Message }
            try { Step manager-macos-metadata timeout @('--signal=TERM','--kill-after=5s','30s',$managerTest,'backends::macos::manager::tests::metadata_matches_original_signed_selectors_and_window_identity','--exact','--ignored','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            try { Step linux-config-rust-parity timeout @('--signal=TERM','--kill-after=5s','90s',$managerTest,'backends::linux::manager::configuration::tests::matches_original_configuration_creation_uuid_and_metadata_bytes','--exact','--ignored','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            finally {if(Test-Path -LiteralPath $env:MFS_P1_LINUX_CONFIG_RUST_ARTIFACTS){Get-ChildItem -LiteralPath $env:MFS_P1_LINUX_CONFIG_RUST_ARTIFACTS -Recurse -File|ForEach-Object{Save $_.FullName}}}
            try {Compare-LinuxWholeConfigReference timeout @('--signal=TERM','--kill-after=5s','90s',$managerTest)}
            catch {$failed=$true;Write-Host $_.Exception.Message}
            try{
                Step linux-compat-rust-parity timeout @('--signal=TERM','--kill-after=5s','60s',$managerTest,'backends::linux::manager::compat::tests::matches_original_compatibility_arguments_metadata_and_portal_call_order','--exact','--ignored','--test-threads=1','--nocapture')
                Step linux-compat-records $ConfigureTool @('upstream-config','linux-compat-records',$env:MFS_P1_LINUX_COMPAT_MATRIX)
            }catch{$failed=$true;Write-Host $_.Exception.Message}
            finally{if(Test-Path -LiteralPath $env:MFS_P1_LINUX_COMPAT_RUST_ARTIFACTS){Get-ChildItem -LiteralPath $env:MFS_P1_LINUX_COMPAT_RUST_ARTIFACTS -Recurse -File|ForEach-Object{Save $_.FullName}};if(Test-Path -LiteralPath (Join-Path $env:MFS_P1_LINUX_COMPAT_MATRIX compat-validation.json)){Save (Join-Path $env:MFS_P1_LINUX_COMPAT_MATRIX compat-validation.json)}}
            try { Step linux-config-manager-controls timeout @('--signal=TERM','--kill-after=5s','30s',$managerTest,'backends::linux::manager::tests::','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            try { Step rust-pipewire-manager-controls timeout @('--signal=TERM','--kill-after=5s','30s',$managerTest,'backends::linux::manager::native::tests::','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            foreach($noneProfile in @('debug','release')) {
                $noneArguments=@('test','--locked','--lib','--features','linux-pipewire')
                if($noneProfile -eq 'release'){$noneArguments+='--release'}
                $noneArguments+=@('backends::linux::manager::native::tests::none_and_unimplemented_native_methods_do_not_enter_capture_adapters','--','--exact','--test-threads=1','--nocapture')
                try{Step ('linux-none-features-'+$noneProfile) cargo $noneArguments}
                catch{$failed=$true;Write-Host $_.Exception.Message}
            }
        }
        try { Step native-joined-task-observation timeout @('--signal=TERM','--kill-after=5s','60s',$sourceExecutable,'--joined-task-observation',(Join-Path $referenceArtifacts task-observation)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
    }
    # Linux sockaddr_un.sun_path has 108 bytes including the terminator. Keep
    # live sockets outside the long content-addressed native build directory.
    $runtime = Join-Path $DepsRoot ('runtime/pw-'+[Guid]::NewGuid().ToString('N').Substring(0,12))
    $socket = Join-Path $runtime mfs-p0-pipewire
    if ([Text.Encoding]::UTF8.GetByteCount($socket) -ge 108) { throw 'Private PipeWire socket exceeds Linux sun_path capacity' }
    New-Item -ItemType Directory -Path $runtime | Out-Null
    Step runtime-permissions chmod @('700',$runtime)
    $env:XDG_RUNTIME_DIR = $runtime
    $env:PIPEWIRE_RUNTIME_DIR = $runtime
    $env:PIPEWIRE_REMOTE = 'mfs-p0-pipewire'
    $env:PIPEWIRE_DEBUG = $SourceReference ? '3,pw.thread-loop:4,pw.data-loop:4' : '3'
    $start = [Diagnostics.ProcessStartInfo]::new((Join-Path $prefix bin/pipewire))
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.ArgumentList.Add('-c')
    $start.ArgumentList.Add((Join-Path $productRoot tests/fixtures/pipewire-probe.conf))
    $server = [Diagnostics.Process]::Start($start)
    $serverOut = $server.StandardOutput.ReadToEndAsync()
    $serverError = $server.StandardError.ReadToEndAsync()
    $until = [DateTime]::UtcNow.AddSeconds(10)
    while (-not(Test-Path -LiteralPath $socket)) {
        if ($server.HasExited -or [DateTime]::UtcNow -ge $until) { throw 'Private PipeWire daemon did not become ready' }
        Start-Sleep -Milliseconds 50
    }
    $fixtureStart = [Diagnostics.ProcessStartInfo]::new($fixtureBinary)
    $fixtureStart.UseShellExecute = $false
    if($SourceReference){$fixtureStart.ArgumentList.Add('--asymmetric')}
    $fixtureStart.RedirectStandardOutput = $true
    $fixtureStart.RedirectStandardError = $true
    $fixture = [Diagnostics.Process]::Start($fixtureStart)
    $fixtureOut = $fixture.StandardOutput.ReadToEndAsync()
    $fixtureError = $fixture.StandardError.ReadToEndAsync()
    $dumpPath = Join-Path $inputRoot private-registry.json
    $until = [DateTime]::UtcNow.AddSeconds(15)
    do {
        if ($fixture.HasExited -or $server.HasExited) { throw 'Owned PipeWire fixture or daemon exited during registration' }
        # Bound this diagnostic subprocess; timeout is retained as a failed probe.
        & timeout --signal=TERM --kill-after=5s 10s pw-dump | Set-Content -LiteralPath $dumpPath -Encoding utf8
        if ($LASTEXITCODE -ne 0) { throw "Private registry query failed with exit $LASTEXITCODE" }
        $nodes = @(Get-Content -LiteralPath $dumpPath -Raw | ConvertFrom-Json | Where-Object { $_.type -eq 'PipeWire:Interface:Node' -and $_.info.props.'node.name' -eq 'mfs-p0-fixture' })
        if($nodes.Count -eq 0) { Start-Sleep -Milliseconds 50 }
    } while ($nodes.Count -eq 0 -and [DateTime]::UtcNow -lt $until)
    Save $dumpPath
    if ($nodes.Count -ne 1) { throw 'Expected exactly the owned SPA fixture node' }
    $env:MFS_P0_PIPEWIRE_SOCKET = $socket
    $env:MFS_P0_PIPEWIRE_NODE_ID = [string]$nodes[0].id
    if($SourceReference){
        # The P0 test creates links itself. Complete U consumers request
        # AUTOCONNECT and require an external routing premise. Supply only the
        # owned source/client route here, without changing backend properties.
        $routeReady=Join-Path $logsRoot route-ready.txt
        $routeStart=[Diagnostics.ProcessStartInfo]::new($routeBinary)
        $routeStart.UseShellExecute=$false
        $routeStart.RedirectStandardOutput=$true
        $routeStart.RedirectStandardError=$true
        $routeStart.ArgumentList.Add($env:MFS_P0_PIPEWIRE_NODE_ID)
        $routeStart.ArgumentList.Add($routeReady)
        $routeSupervisor=[Diagnostics.Process]::Start($routeStart)
        $routeOut=$routeSupervisor.StandardOutput.ReadToEndAsync()
        $routeError=$routeSupervisor.StandardError.ReadToEndAsync()
        $until=[DateTime]::UtcNow.AddSeconds(10)
        while(-not(Test-Path -LiteralPath $routeReady)){
            if($routeSupervisor.HasExited -or [DateTime]::UtcNow -ge $until){throw 'Owned exact-source route supervisor did not become ready'}
            Start-Sleep -Milliseconds 20
        }
        Save $routeReady
    }
    if(-not $SourceReference){
        Step rust-pipewire-capture cargo @('test','--release','--locked','--test','p0-interop','--features','prototype-linux-pipewire','--','--include-ignored','--test-threads=1','--nocapture')
    }
    try { Step rust-pipewire-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-linux-pipewire,linux-pipewire','--','-D','warnings') }
    catch {
        if(-not $SourceReference){throw}
        # Keep the failed gate while collecting independently runnable native
        # source/driver observations. No failure becomes an overall pass.
        $failed=$true
        Write-Host $_.Exception.Message
    }
    if($SourceReference){
        # Independent processes; a failed ownership premise must not erase the
        # separate original-source lifecycle evidence or become an overall pass.
        try { Step pipewire-fd-error-observation timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--fd-error',$socket) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-fd-add-io-error-observation timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--fd-add-io-error',$socket) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step original-pipewire-source-context timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,$referenceArtifacts) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-daemon-shm timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::captures_requested_shm_rows_and_retains_sixty_cpu_samples_after_shutdown','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-supplied-fd-shm timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::captures_supplied_fd_shm_and_closes_twenty_consumed_sockets','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-opencv timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::captures_bgr_after_native_unlock_and_retains_opencv_owners_after_shutdown','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        foreach($probe in @(
            @('daemon','--manager','reconnects_daemon_manager_and_retains_native_cpu_images'),
            @('fd','--manager-fd','forwards_supplied_fd_manager_and_closes_consumed_socket')
        )) {
            try { Step ('original-pipewire-manager-'+$probe[0]) timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,$probe[1],$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $referenceArtifacts ('manager-'+$probe[0]))) }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            try { Step ('rust-pipewire-manager-'+$probe[0]) timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire',('backends::linux::pipewire::native::tests::opencv::manager_capture::'+$probe[2]),'--','--exact','--ignored','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
        }
        $wholeMatrix=Join-Path $referenceArtifacts whole-native-manager
        New-Item -ItemType Directory -Path $wholeMatrix|Out-Null
        $env:MFS_P1_PIPEWIRE_WHOLE_ARTIFACTS=$wholeMatrix
        foreach($probe in @(
            @('daemon','--whole-manager-daemon','compares_whole_daemon_manager_capture_inactive_and_reconstruction'),
            @('fd','--whole-manager-fd','compares_whole_fd_manager_capture_inactive_and_consumed_socket')
        )) {
            $modeRoot=Join-Path $wholeMatrix $probe[0]
            New-Item -ItemType Directory -Path $modeRoot|Out-Null
            try { Step ('original-pipewire-whole-'+$probe[0]) timeout @('--signal=TERM','--kill-after=5s','180s',$env:MFS_P1_PIPEWIRE_WHOLE_SOURCE,$probe[1],$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $modeRoot source)) }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            try { Step ('rust-pipewire-whole-'+$probe[0]) timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire',('backends::linux::pipewire::native::tests::opencv::manager_capture::'+$probe[2]),'--','--exact','--ignored','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
        }
        try { Step pipewire-whole-manager-records $ConfigureTool @('upstream-trace','pipewire-whole-manager',$wholeMatrix) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $hostMatrix=Join-Path $referenceArtifacts manager-host-matrix
        New-Item -ItemType Directory -Path $hostMatrix|Out-Null
        $env:MFS_P1_PIPEWIRE_HOST_ARTIFACTS=Join-Path $hostMatrix rust
        try { Step original-pipewire-manager-host timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,'--manager-host',$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $hostMatrix source)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-manager-host timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::host_manager::retains_real_capture_after_host_init_false_or_error_and_closes_native_before_host_owner','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-host-manager-records $ConfigureTool @('upstream-trace','pipewire-host',$hostMatrix) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $apiMatrix=Join-Path $referenceArtifacts kwin-native-api-matrix
        New-Item -ItemType Directory -Path $apiMatrix|Out-Null
        $env:MFS_P1_PIPEWIRE_HOST_ARTIFACTS=Join-Path $apiMatrix rust
        try { Step original-kwin-native-host timeout @('--signal=TERM','--kill-after=5s','180s',$env:MFS_P1_KWIN_NATIVE_SOURCE,'--kwin-native',$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $apiMatrix source)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-kwin-native-host timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::host_manager::retains_real_capture_after_host_init_false_or_error_and_closes_native_before_host_owner','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step kwin-native-api-records $ConfigureTool @('upstream-trace','pipewire-host-api',$apiMatrix) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        $portalFailures=Join-Path $referenceArtifacts kwin-portal-failure
        New-Item -ItemType Directory -Path $portalFailures|Out-Null
        $env:MFS_P1_KWIN_PORTAL_FAILURE_ROOT=$portalFailures
        try { Step original-kwin-portal-failure timeout @('--signal=TERM','--kill-after=5s','30s',$env:MFS_P1_KWIN_NATIVE_SOURCE,'--kwin-portal-failure',$socket,(Join-Path $portalFailures source)) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        finally {if(Test-Path -LiteralPath (Join-Path $portalFailures source)){Get-ChildItem -LiteralPath (Join-Path $portalFailures source) -File -Recurse|ForEach-Object{Save $_.FullName}}}
        try { Step rust-kwin-portal-failure timeout @('--signal=TERM','--kill-after=5s','90s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','--','backends::linux::manager::compat::tests::portal_failures::retains_host_socket_after_each_portal_failure_before_manager_construction','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        finally {if(Test-Path -LiteralPath (Join-Path $portalFailures rust)){Get-ChildItem -LiteralPath (Join-Path $portalFailures rust) -File -Recurse|ForEach-Object{Save $_.FullName}}}
        try { Step kwin-portal-failure-records $ConfigureTool @('upstream-trace','kwin-portal-failure',$portalFailures) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        finally {if(Test-Path -LiteralPath (Join-Path $portalFailures portal-failure-validation.json)){Save (Join-Path $portalFailures portal-failure-validation.json)}}
        $mobilityMatrix=Join-Path $referenceArtifacts thread-mobility
        $env:MFS_P1_PIPEWIRE_THREAD_MOBILITY=$mobilityMatrix
        foreach($mode in 0,1) {
            try { Step "original-pipewire-thread-mobility-$mode" timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,'--thread-mobility',$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,"$mode",(Join-Path $mobilityMatrix "source/mode-$mode")) }
            catch { $failed=$true; Write-Host $_.Exception.Message }
        }
        try { Step rust-pipewire-thread-mobility timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::thread_mobility::moves_complete_capture_between_live_callers_and_unwinds_before_foreign_close','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step pipewire-thread-mobility-records $ConfigureTool @('upstream-trace','pipewire-thread-mobility',$mobilityMatrix) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step original-pipewire-sustained timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,'--sustained-manager',$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $referenceArtifacts sustained)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-sustained timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::opencv::sustained::sustains_native_manager_for_two_minutes_with_bounded_retained_owners','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step original-pipewire-source-reinit timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,'--reinit',$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $referenceArtifacts reinit)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-supplied-fd-reinit timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::reinit::reinitializes_after_consumed_fd_and_retains_both_sessions_cpu_images','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step original-pipewire-source-fd-retry timeout @('--signal=TERM','--kill-after=5s','180s','env',"LD_PRELOAD=$env:MFS_P1_PIPEWIRE_FD_FAULT_LIBRARY",$sourceExecutable,'--fd-retry',$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,(Join-Path $referenceArtifacts fd-retry)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try {
            # Resolve the actual libtest executable through Cargo's structured
            # output. Only that isolated consumer receives LD_PRELOAD; Cargo,
            # rustc, producer, router and daemon keep their ordinary environment.
            $fdTest=Capture-TestProgram rust-pipewire-fd-retry-build
            Step rust-pipewire-supplied-fd-retry timeout @('--signal=TERM','--kill-after=5s','180s','env',"LD_PRELOAD=$env:MFS_P1_PIPEWIRE_FD_FAULT_LIBRARY",$fdTest,'backends::linux::pipewire::native::tests::reinit::retries_proven_live_fd_after_native_add_io_failure_and_retains_both_sessions','--exact','--ignored','--test-threads=1','--nocapture')
        } catch { $failed=$true; Write-Host $_.Exception.Message }
        # End the ordinary shared-offset producer before separately allocated
        # zero-offset MemFd and kernel DMA fixtures. Consumer behavior stays F.
        StopOwned route-supervisor $routeSupervisor $routeOut $routeError
        $routeSupervisor=$null
        StopOwned source $fixture $fixtureOut $fixtureError
        $fixture=$null
        . (Join-Path $PSScriptRoot validate-pipewire-dma-stream.ps1)
        $formatTest=Capture-TestProgram rust-pipewire-format-stream-build
        $savedFormatTest=Join-Path $nativeArtifacts mfs-p1-pipewire-format-stream-test
        Copy-Item -LiteralPath $formatTest -Destination $savedFormatTest
        Save $savedFormatTest
        $formatTest=$savedFormatTest
        foreach($format in @('RGB','RGBA','RGBx','BGR','BGRA','BGRx','YUY2')) {
            try { Invoke-PipeWireOwnedStream -Format $format }
            catch {
                $failed=$true
                $formatError=Join-Path $logsRoot "pipewire-format-$format-setup-error.log"
                $_.Exception.ToString()|Set-Content -LiteralPath $formatError -Encoding utf8
                $checks.Add([ordered]@{name="pipewire-format-$format-setup";status='failed';log=(Artifact $formatError)})
                Write-Host $_.Exception.Message
            }
        }
        try { Step pipewire-format-stream-records $ConfigureTool @('upstream-trace','pipewire-format-stream',(Join-Path $referenceArtifacts format-stream)) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Invoke-PipeWireOwnedStream -MemFd }
        catch {
            $failed=$true
            $memfdError=Join-Path $logsRoot pipewire-memfd-stream-error.log
            $_.Exception.ToString()|Set-Content -LiteralPath $memfdError -Encoding utf8
            $checks.Add([ordered]@{name='pipewire-memfd-stream-setup';status='failed';log=(Artifact $memfdError)})
            Write-Host $_.Exception.Message
        }
        try { Invoke-PipeWireOwnedStream }
        catch {
            $failed=$true
            $dmaStreamError=Join-Path $logsRoot pipewire-dma-stream-error.log
            $_.Exception.ToString()|Set-Content -LiteralPath $dmaStreamError -Encoding utf8
            $checks.Add([ordered]@{name='pipewire-dma-stream-setup';status='failed';log=(Artifact $dmaStreamError)})
            Write-Host $_.Exception.Message
        }
    }
} catch {
    $failed = $true
    $log = Join-Path $logsRoot preparation-error.log
    $_.Exception.ToString() | Set-Content -LiteralPath $log -Encoding utf8
    $checks.Add([ordered]@{name='pipewire-preparation';status='failed';log=(Artifact $log)})
    Write-Host $_.Exception.ToString()
} finally {
    foreach($directory in @($referenceArtifacts,$rustArtifacts,$fdArtifacts,$bgrArtifacts)){
        if($directory){Get-ChildItem -LiteralPath $directory -File -Recurse|ForEach-Object{Save $_.FullName}}
    }
    StopOwned route-supervisor $routeSupervisor $routeOut $routeError
    StopOwned source $fixture $fixtureOut $fixtureError
    StopOwned daemon $server $serverOut $serverError
    Json (Join-Path $RunRoot native-result.json) ([ordered]@{
        layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase=($SourceReference ? 'P1' : 'P0');phase_status='incomplete'
        checks=@($checks.ToArray());inputs=@($inputs.ToArray())
        pending=@('Owned native producer only: portal/desktop capture, DMA-BUF, Wlr, macOS, source/method review and complete phase gates remain outstanding. SourceReference routing is supplied by a separately owned, exact-producer/consumer supervisor on the private daemon; product AUTOCONNECT/waiting/properties and P0 explicit links are not changed.','Four portable consumer tests and four checked row/storage tests are separate from native capture.',($SourceReference ? 'Original-source context selected: complete fixed PipeWireScreencap header/cpp included unedited in one translation unit, exact extracted screenshot base, private namespace/logging overlays, asymmetric padded-RGB fixture. Twenty FD/init/three-output/inactive/close source cycles require sixty known BGR results and retained images plus FD/thread observations. Separate native Rust daemon/SHM probe requires twenty cycles/sixty raw RGB outputs with exact negotiation POD bytes, queue ownership and held CPU samples. RawAudit preserves RGB. Separate private OpenCV ABI/exception/failure-output checks plus twenty lifecycles/sixty owning BGR images and twenty native unlock witnesses are required by an independent probe, not accepted by the raw entry. Its witness adds waiting, so timings are not source latency parity. Opaque Mat/loader mobility and full source/OpenCV parity remain pending. Both raw global initializations are balanced by test-only deinit after destruction. Timings are diagnostic; source API freshness/replay is not exposed. Portal FD, owned mapping/DMA-BUF, full format/failure/alias matrices, framework/input and complete P1 remain pending. P0 Rust native probe stays a separate default-pattern entry.' : 'Original-source context/driver probe not selected. The default P0 RGB-bar producer and native Rust stream prototype remain separate from later source/Rust parity.'),'Latest Rust dependency checks are recorded by the parent batch and remain required even if these diagnostic native probes pass.')
    })
}
if($failed -or @($checks | Where-Object status -EQ failed).Count) { exit 1 }
