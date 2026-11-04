# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Actual existing Foundation/Blocks acceptance program; no screenshot or SDK.
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference=$false
$productRoot=Split-Path -Parent $PSScriptRoot
$siblingRoot=Split-Path -Parent $productRoot
$referenceRoot=Join-Path $siblingRoot maafw-screencap-reference
$depsRoot=Join-Path $siblingRoot maafw-screencap-deps
$id='p1-macos-foundation-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffffffZ')
$runRoot=Join-Path $referenceRoot "runs/$id"
$inputRoot=Join-Path $runRoot inputs
$logsRoot=Join-Path $runRoot logs
New-Item -ItemType Directory -Path $inputRoot,$logsRoot|Out-Null
$checks=[Collections.Generic.List[object]]::new()
$inputs=[Collections.Generic.List[object]]::new()
function Artifact([string]$Path) {
    $relative=[IO.Path]::GetRelativePath($referenceRoot,$Path).Replace('\','/')
    if([IO.Path]::IsPathRooted($relative) -or $relative.StartsWith('../')){throw 'Evidence escaped reference'}
    [ordered]@{path=$relative;sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
function Step([string]$Name,[string]$Program,[string[]]$Arguments) {
    $log=Join-Path $logsRoot "$Name.log";$status='failed'
    try {
        "program=$Program`narguments=$($Arguments|ConvertTo-Json -Compress)"|Set-Content -LiteralPath $log -Encoding utf8
        & $Program @Arguments >> $log 2>&1
        $code=$LASTEXITCODE
        "exit=$code"|Add-Content -LiteralPath $log -Encoding utf8
        if($code -ne 0){throw "$Name exited with $code; see $log"}
        if($Name -eq 'native-foundation-control') {
            $observed=Get-Content -LiteralPath $log -Raw
            foreach($summary in @(
                'macOS native MRC completion_inputs cases=2 invoked=1 uninvoked=1 heap_alias=2 deallocated=4 fixture_reclaimed_refs=2 capture=0 full_P1_acceptance=0 passed',
                'macOS native input weak control pairs=2 objects=4 joined_worker_releases=4 alias_keeps_live=2 zero_after_final_release=4 native_capture=0 passed',
                'macOS native block leases cases=4 stack_to_heap=4 heap_alias=1 handler_return_keeps_lease=1 uninvoked_disposal=1 joined_worker_release=1 acquired=12 released=12 live=0 native_callbacks=0 capture=0 passed'
            )) {
                if([regex]::Matches($observed,'(?m)^'+[regex]::Escape($summary)+'\r?$').Count -ne 1){throw 'Incomplete native Foundation ownership assertions'}
            }
        }
        $status='passed'
    } finally {$checks.Add([ordered]@{name=$Name;status=$status;log=(Artifact $log)})}
}
try {
    $hostLog=Join-Path $logsRoot host.log
    $platform=[Runtime.InteropServices.RuntimeInformation]::OSDescription
    $architecture=[Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    $processArchitecture=[Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
    "platform=$platform`nos_architecture=$architecture`nprocess_architecture=$processArchitecture`npolicy=preinstalled`nlatest_version_gate=0"|Set-Content -LiteralPath $hostLog
    $nativeMac=[Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([Runtime.InteropServices.OSPlatform]::OSX)
    $hostStatus=if($nativeMac -and $architecture -eq [Runtime.InteropServices.Architecture]::Arm64){'passed'}else{'missing_environment'}
    $checks.Add([ordered]@{name='host';status=$hostStatus;log=(Artifact $hostLog)})
    if($hostStatus -ne 'passed'){throw 'Foundation control requires native macOS arm64'}
    Step source-head git @('-C',$productRoot,'rev-parse','HEAD')
    Step xcode xcodebuild @('-version')
    Step sdk-version xcrun @('--show-sdk-version')
    $compiler=(& xcrun --find clang++|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $compiler)){throw 'Installed clang++ missing'}
    $sdk=(& xcrun --show-sdk-path|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $sdk -PathType Container)){throw 'Installed SDK missing'}
    Step compiler $compiler @('--version')
    $source=Join-Path $productRoot tests/upstream-parity/macos-screencap
    foreach($name in @('block-lease-control.mm','callback-ledger.hpp','input-weak-observer.hpp')) {
        $saved=Join-Path $inputRoot $name
        Copy-Item -LiteralPath (Join-Path $source $name) -Destination $saved
        $inputs.Add((Artifact $saved))
    }
    Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $inputRoot check-macos-foundation.ps1)
    $inputs.Add((Artifact (Join-Path $inputRoot check-macos-foundation.ps1)))
    $build=Join-Path $depsRoot "native/$id"
    New-Item -ItemType Directory -Path $build|Out-Null
    $program=Join-Path $build mfs_mac_block_lease_control
    Step compile $compiler @('-std=gnu++23','-O3','-DNDEBUG','-arch','arm64','-mmacosx-version-min=14.0','-isysroot',$sdk,'-fno-objc-arc','-fblocks','-Wall','-Wextra','-Werror','-framework','Foundation',(Join-Path $source block-lease-control.mm),'-o',$program)
    $saved=Join-Path $inputRoot mfs_mac_block_lease_control
    Copy-Item -LiteralPath $program -Destination $saved
    $inputs.Add((Artifact $saved))
    $identity=Join-Path $inputRoot toolchain.json
    [ordered]@{compiler=$compiler;sdk=$sdk;policy='preinstalled';strict_latest_gate=$false;target='aarch64-apple-darwin';deployment='14.0';program_sha256=(Get-FileHash -LiteralPath $program -Algorithm SHA256).Hash.ToLowerInvariant();program_unix_mode=[int][IO.File]::GetUnixFileMode($program);saved_unix_mode=[int][IO.File]::GetUnixFileMode($saved)}|ConvertTo-Json|Set-Content -LiteralPath $identity
    $inputs.Add((Artifact $identity))
    Step native-foundation-control $program @()
} finally {
    [ordered]@{layout_version=1;run_id=$id;phase='P1';phase_status='incomplete';checks=$checks;inputs=$inputs;pending=@('Only the native Foundation/Blocks control. No ScreenCaptureKit request, screenshot, complete backend or P1 acceptance. Installed tool age is advisory; compilation and every native assertion remain strict.')}|ConvertTo-Json -Depth 20|Set-Content -LiteralPath (Join-Path $runRoot manifest.json)
    Write-Host "Native Foundation evidence: $runRoot"
}
