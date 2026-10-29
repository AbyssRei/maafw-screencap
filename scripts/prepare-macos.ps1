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
$productRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $productRoot
$logsRoot=Join-Path $RunRoot macos-arm64/logs
$inputRoot=Join-Path $RunRoot inputs
New-Item -ItemType Directory -Path $logsRoot -Force | Out-Null
$checks=[Collections.Generic.List[object]]::new()
$inputs=[Collections.Generic.List[object]]::new()
$failed=$false
$referenceArtifacts=$null

function Artifact([string]$Path) {
    $relative=[IO.Path]::GetRelativePath($ReferenceRoot,$Path).Replace('\','/')
    if($relative.StartsWith('../') -or [IO.Path]::IsPathRooted($relative)){throw 'Evidence escaped reference'}
    return [ordered]@{path=$relative;sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
function Save([string]$Path) { $inputs.Add((Artifact $Path)) }
function Json([string]$Path,$Value) {
    $stream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write)
    try{$stream.Write([Text.Encoding]::UTF8.GetBytes(($Value|ConvertTo-Json -Depth 40)+"`n"))}finally{$stream.Dispose()}
}
function Step([string]$Name,[string]$Program,[string[]]$Arguments) {
    $log=Join-Path $logsRoot "$Name.log"
    $status='failed'
    Write-Host "Running $Name ..."
    try {
        "program=$Program`narguments=$($Arguments|ConvertTo-Json -Compress)"|Set-Content -LiteralPath $log -Encoding utf8
        & $Program @Arguments >> $log 2>&1
        $code=$LASTEXITCODE
        "exit=$code"|Add-Content -LiteralPath $log -Encoding utf8
        if($code -eq 77 -and ($Name -in @('native-capture','original-macos-windows','original-macos-display') -or $Name -match '^rust-macos-(capture|exception|timeout|display|manager)(-original-oracle)?$')){$status='missing_environment'}
        elseif($code -eq 0){$status='passed'}
        if($status -ne 'passed'){
            Get-Content -LiteralPath $log -Tail 50|ForEach-Object{Write-Host $_}
            throw "$Name exited with $code ($status); see $log"
        }
        $observed=Get-Content -LiteralPath $log -Raw
        if($Name -eq 'custom-native-parity'){
            $status='failed';Assert-CustomNativeLog $observed;$status='passed'
        }
        if($Name -eq 'custom-core-runtime-tree'){
            $status='failed';Assert-CustomRuntimeTree $observed;$status='passed'
        }
        if($Name -eq 'original-macos-windows' -and $observed -notmatch 'original macOS windows cycles=20 captures=60 retained=60 missing_window=1 joined_callers=1 passed'){
            $status='failed';throw 'Expected twenty actual original macOS window lifecycles and sixty retained known-interior images'
        }
        if($Name -eq 'original-macos-display' -and $observed -notmatch 'original macOS display captures=1 retained=1 known_pixels=not_asserted passed'){
            $status='failed';throw 'Expected one actual original first-display layout/owner observation, separate from pixel acceptance'
        }
        if($Name -in @('rust-macos-core-dependencies','rust-macos-manager-dependencies') -and $observed -match '(?m)^(objc2-app-kit|opencv) v'){
            $status='failed';throw 'Direct capture-only core graph contains an acceptance/AppKit crate dependency'
        }
        if($Name -match '^rust-macos-(capture|exception|timeout|display)-original-oracle$' -and $observed -notmatch 'original macOS external_window captures=1 known_interior=1 passed'){
            $status='failed';throw 'Expected the unchanged source to capture the same owned external window'
        }
        if($Name -eq 'rust-macos-manager-original-oracle' -and -not $observed.Contains('original macOS manager cycles=20 captures=60 retained=60 connect_capture_calls=0 source_owners_released=20 input_attempts=0 passed')){
            $status='failed';throw 'Expected twenty original manager/source lifecycles and sixty same-window oracle images'
        }
        if($Name -eq 'rust-macos-manager-controls' -and $observed -notmatch '(?m)^test result: ok\. 4 passed; 0 failed; 1 ignored;'){
            $status='failed';throw 'Expected four actual portable macOS manager controls'
        }
        if($Name -eq 'rust-macos-manager-metadata' -and ($observed -notmatch 'manager metadata kind=macos cases=66 exact_original_json=1 input_none=1 no_native_connection=1 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;')) {
            $status='failed';throw 'Expected sixty-six actual original/Rust metadata queries, separate from native capture'
        }
        if($Name -eq 'rust-macos-manager-metadata') {
            & $ConfigureTool upstream-metadata macos (Join-Path $env:MFS_P1_MANAGER_METADATA_ARTIFACTS macos) >> $log 2>&1
            if($LASTEXITCODE -ne 0){$status='failed';throw 'Metadata source/Rust artifact validation failed'}
        }
        if($Name -match '^rust-macos-(capture|exception|timeout|display|layout|dimensions|manager|manager-scripts)$'){
            if($observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;'){
                $status='failed';throw 'Expected exactly one actual Rust macOS test, not a zero-test or ignored run'
            }
            $summary=switch($Name){
                'rust-macos-capture' {'native Rust macOS cycles=20 captures=60 retained_raw=60 retained_bgr=60 callback_conversion=60 lock_pairs=60 source_create_refs_retained=60 fixture_reclaimed_refs=60 cross_thread_cpu_release=1 passed'}
                'rust-macos-exception' {'native Rust macOS conversion_exception=1 unlock_after_exception=1 subsequent_capture=1 missing_window=1 fixture_reclaimed_refs=3 passed'}
                'rust-macos-timeout' {'native Rust macOS timeout_seconds=5 delayed_conversion=1 late_conversion=1 native_inputs_retained=1 module_retained=1 late_owners_drained=1 fixture_reclaimed_refs=1 passed'}
                'rust-macos-display' {'native Rust macOS first_display=1 callback_conversion=1 retained_cpu=1 known_pixels=not_asserted fixture_reclaimed_refs=1 passed'}
                'rust-macos-manager' {'native Rust macOS manager cycles=20 captures=60 retained_bgr=60 missing_output_preserved=1 fixture_reclaimed_refs=61 input_none=1 passed'}
                default {$null}
            }
            if($summary -and -not $observed.Contains($summary)){$status='failed';throw 'Expected the complete native Rust macOS ownership/capture summary'}
            if($summary -and -not $observed.Contains('native Rust macOS host main_thread=1 appkit_initialized=1 pumped=1 harness_worker_joined=1')){
                $status='failed';throw 'Expected the explicit original-equivalent main-thread AppKit host and joined libtest worker'
            }
            if($Name -eq 'rust-macos-manager-scripts' -and -not $observed.Contains('original Rust macOS manager scripts=320 exact_bodies=1 output_and_unit_events=1 input_none=1 passed')){
                $status='failed';throw 'Expected all 320 exact-body manager source/Rust script comparisons'
            }
        }
    }finally{
        $checks.Add([ordered]@{name=$Name;status=$status;log=(Artifact $log)})
        Write-Host "${Name}: $status"
    }
}
. (Join-Path $PSScriptRoot prepare-apple-toolchain.ps1)
try {
    if(-not $IsMacOS -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'Arm64'){throw 'Native macOS arm64 is required'}
    if($SourceReference -and (-not $UpstreamRoot -or -not(Test-Path -LiteralPath $UpstreamRoot -PathType Container))){throw 'Original macOS context requires the verified fixed source directory'}
    Initialize-AppleToolchain -InputRoot $inputRoot
    . (Join-Path $PSScriptRoot configure-prototype.ps1) -ConfigureTool $ConfigureTool -RustBatch $RustBatch -ReferenceRoot $ReferenceRoot -Family screen-capture-kit
    $inputs.Add((Artifact $env:MFS_BUILD_CONFIGURATION))
    Step host-identity uname @('-a')
    Step os-identity sw_vers @()
    Step xcode-identity xcodebuild @('-version')
    Step sdk-identity xcrun @('--sdk','macosx','--show-sdk-version')
    Step sdk-location xcrun @('--sdk','macosx','--show-sdk-path')
    Step compiler-identity xcrun @('clang','--version')
    $env:MACOSX_DEPLOYMENT_TARGET='14.0'
    $configuration=Join-Path $inputRoot native-configuration.json
    Json $configuration ([ordered]@{target='aarch64-apple-darwin';deployment_target=$env:MACOSX_DEPLOYMENT_TARGET;scope=($SourceReference ? 'P1 fixed original Objective-C++ source plus initial private Rust migration; full backend/P1 acceptance incomplete.' : 'P0 objc2/ScreenCaptureKit controlled-window sample-buffer capture and callback ownership only');sdk=(& xcrun --sdk macosx --show-sdk-version|Out-String).Trim();developer_dir=$env:DEVELOPER_DIR;sdkroot=$env:SDKROOT})
    $inputs.Add((Artifact $configuration))
    if($SourceReference){
        try { Step rust-macos-migration-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-macos,macos-manager','--','-D','warnings') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        # Direct Cargo consumer entry: explicit capture capability, without the
        # prototype's AppKit feature or its prepared P0 feature configuration.
        # Xcode/SDK selection remains the same; restore the environment exactly.
        $prototypeConfiguration=$env:MFS_BUILD_CONFIGURATION
        try {
            Remove-Item Env:MFS_BUILD_CONFIGURATION -ErrorAction SilentlyContinue
            try { Step rust-macos-core-only-build cargo @('build','--release','--locked','--lib','--no-default-features','--features','macos-screen-capture-kit') }
            catch { $failed=$true;Write-Host $_.Exception.Message }
            try { Step rust-macos-core-dependencies cargo @('tree','--locked','--no-default-features','--features','macos-screen-capture-kit','--target','aarch64-apple-darwin','--edges','normal,build','--prefix','none') }
            catch { $failed=$true;Write-Host $_.Exception.Message }
            try { Step rust-macos-manager-only-build cargo @('build','--release','--locked','--lib','--no-default-features','--features','macos-manager') }
            catch { $failed=$true;Write-Host $_.Exception.Message }
            try { Step rust-macos-manager-dependencies cargo @('tree','--locked','--no-default-features','--features','macos-manager','--target','aarch64-apple-darwin','--edges','normal,build','--prefix','none') }
            catch { $failed=$true;Write-Host $_.Exception.Message }
        }finally{$env:MFS_BUILD_CONFIGURATION=$prototypeConfiguration}
        . (Join-Path $PSScriptRoot prepare-macos-reference.ps1)
        $executable=Prepare-MacosReference $UpstreamRoot
        $referenceArtifacts=Join-Path $RunRoot macos-arm64/reference-u/artifacts
        . (Join-Path $PSScriptRoot prepare-custom-unix-reference.ps1)
        try { Test-CustomUnixReference $UpstreamRoot $referenceArtifacts }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        $windows=Join-Path $referenceArtifacts windows
        $display=Join-Path $referenceArtifacts display
        New-Item -ItemType Directory -Path $windows,$display|Out-Null
        Json (Join-Path $windows geometry.json) ([ordered]@{width=47;height=31;channels=3;row_bytes=141;interior_margin=2;call_colours=@('green','red','blue');scope='Unchanged original source, twenty windows/sixty captures; resource counts observed only, not a balance assertion.'})
        try { Step original-macos-windows $executable @('windows',$windows) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step original-macos-display $executable @('display',$display) }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        $features='prototype-macos,macos-manager'
        try { Step rust-macos-layout cargo @('test','--release','--locked','--lib','--features',$features,'platform::macos::pixel_buffer::tests::rejects_overflowing_or_unreadable_bgra_layouts','--','--exact','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-macos-dimensions cargo @('test','--release','--locked','--lib','--features',$features,'backends::macos::screen_capture_kit::tests::rejects_unrepresentable_source_config_dimensions','--','--exact','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-macos-manager-controls cargo @('test','--release','--locked','--lib','--features',$features,'backends::macos::manager::tests::','--','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        try { Step rust-macos-manager-metadata cargo @('test','--release','--locked','--lib','--features',$features,'backends::macos::manager::tests::metadata_matches_original_signed_selectors_and_window_identity','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        $env:MFS_P1_MACOS_MANAGER_SCRIPTS=Join-Path $referenceArtifacts manager-scripts
        try { Step rust-macos-manager-scripts cargo @('test','--release','--locked','--lib','--features',$features,'backends::macos::screen_capture_kit::tests::matches_original_manager_connect_forwarding_output_and_owner_scripts','--','--exact','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true;Write-Host $_.Exception.Message }
        $rustArtifacts=Join-Path $referenceArtifacts rust
        New-Item -ItemType Directory -Path $rustArtifacts|Out-Null
        foreach($probe in @(
            @('rust-macos-capture','captures_original_window_with_callback_conversion_and_retained_cpu_owners'),
            @('rust-macos-exception','releases_native_lock_after_conversion_exception_and_rejects_missing_window'),
            @('rust-macos-timeout','retains_timed_out_callback_work_inputs_converter_and_late_result'),
            @('rust-macos-display','captures_original_first_display_layout_and_retains_its_cpu_owners'),
            @('rust-macos-manager','forwards_manager_captures_and_keeps_outputs_through_manager_and_window_shutdown')
        )){
            try { Run-RustMacosProbe $probe[0] ('backends::macos::screen_capture_kit::tests::'+$probe[1]) $executable $rustArtifacts }
            catch { $failed=$true;Write-Host $_.Exception.Message }
        }
    }else{
    $messages=Join-Path $inputRoot compiler-artifacts.jsonl
    $log=Join-Path $logsRoot native-build.log
    $buildStatus='failed'
    Write-Host 'Running native-build ...'
    try {
        & cargo test --release --locked --test p0-macos --features prototype-macos --no-run --message-format=json > $messages 2> $log
        $code=$LASTEXITCODE
        "exit=$code"|Add-Content -LiteralPath $log -Encoding utf8
        if($code -ne 0){
            Get-Content -LiteralPath $log -Tail 60|ForEach-Object{Write-Host $_}
            Get-Content -LiteralPath $messages|ForEach-Object{$_|ConvertFrom-Json}|Where-Object reason -EQ compiler-message|ForEach-Object{Write-Host $_.message.rendered}
            throw 'ScreenCaptureKit prototype did not compile'
        }
        $buildStatus='passed'
    }finally{
        $checks.Add([ordered]@{name='native-build';status=$buildStatus;log=(Artifact $log)})
        $inputs.Add((Artifact $messages))
    }
    $artifacts=@(Get-Content -LiteralPath $messages|ForEach-Object{$_|ConvertFrom-Json}|Where-Object{$_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'p0-macos' -and $_.executable})
    if($artifacts.Count -ne 1){throw 'Expected exactly one native prototype executable'}
    $executable=$artifacts[0].executable
    $saved=Join-Path $inputRoot p0-macos
    Copy-Item -LiteralPath $executable -Destination $saved
    $inputs.Add((Artifact $saved))
    Step native-linkage otool @('-L',$executable)
    Step native-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-macos','--','-D','warnings')
    $env:MFS_P0_MACOS_CAPTURE='1'
    Step native-capture $executable @()
    }
}catch{
    $failed=$true
    $log=Join-Path $logsRoot preparation-error.log
    $_.Exception.ToString()|Set-Content -LiteralPath $log -Encoding utf8
    $classification=if($checks.Count -gt 0 -and $checks[$checks.Count-1].status -eq 'missing_environment'){'missing_environment'}else{'failed'}
    $checks.Add([ordered]@{name='macos-preparation';status=$classification;log=(Artifact $log)})
    Write-Host $_.Exception.ToString()
}finally{
    if($referenceArtifacts){Get-ChildItem -LiteralPath $referenceArtifacts -File -Recurse|ForEach-Object{Save $_.FullName}}
    Json (Join-Path $RunRoot native-result.json) ([ordered]@{
        layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase=($SourceReference ? 'P1' : 'P0');phase_status='incomplete'
        checks=@($checks.ToArray());inputs=@($inputs.ToArray())
        pending=@(($SourceReference ? 'Initial private Rust capture/manager slice plus unchanged Objective-C++ capture and extracted exact manager bodies; no complete P1 acceptance. Native dispatch semaphore wait/signal and original unreleased create reference remain U. Rust fixture reclaims that reference only after all actual owners end; it is never product cleanup success. Checked dimensions/layout/locking, CPU copy and contained callback/conversion failures are explicit safety adaptations. Full semaphore/port-growth causality, native failures/formats/late races, known full-display pixels, complete manager/API/metadata and full Rust parity remain pending.' : 'P0 feasibility only; full target/format/failure matrix, original C++ parity and complete platform gates remain outstanding. Formal Xcode selection is distinct from successful SDK use and native execution.'),'Screen Recording/desktop unavailability is missing environment, never capture acceptance. No TCC database or host permission setting is modified.','Rust dependency and convention gates remain required even when this diagnostic native probe passes.')
    })
}
if($failed -or @($checks|Where-Object status -NE passed).Count){exit 1}
