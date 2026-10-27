# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$DepsRoot,
    [Parameter(Mandatory)][string]$RunRoot
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

function Artifact([string]$Path) {
    $relative=[IO.Path]::GetRelativePath($ReferenceRoot,$Path).Replace('\','/')
    if($relative.StartsWith('../') -or [IO.Path]::IsPathRooted($relative)){throw 'Evidence escaped reference'}
    return [ordered]@{path=$relative;sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
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
        if($code -eq 77 -and $Name -eq 'native-capture'){$status='missing_environment'}
        elseif($code -eq 0){$status='passed'}
        if($status -ne 'passed'){
            Get-Content -LiteralPath $log -Tail 50|ForEach-Object{Write-Host $_}
            throw "$Name exited with $code ($status); see $log"
        }
    }finally{
        $checks.Add([ordered]@{name=$Name;status=$status;log=(Artifact $log)})
        Write-Host "${Name}: $status"
    }
}
try {
    if(-not $IsMacOS -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'Arm64'){throw 'Native macOS arm64 is required'}
    Step host-identity uname @('-a')
    Step os-identity sw_vers @()
    Step xcode-identity xcodebuild @('-version')
    Step sdk-identity xcrun @('--sdk','macosx','--show-sdk-version')
    Step sdk-location xcrun @('--sdk','macosx','--show-sdk-path')
    Step compiler-identity xcrun @('clang','--version')
    $env:MACOSX_DEPLOYMENT_TARGET='14.0'
    $configuration=Join-Path $inputRoot native-configuration.json
    Json $configuration ([ordered]@{target='aarch64-apple-darwin';deployment_target=$env:MACOSX_DEPLOYMENT_TARGET;scope='P0 objc2/ScreenCaptureKit controlled-window sample-buffer capture and callback ownership only';sdk=(& xcrun --sdk macosx --show-sdk-version|Out-String).Trim();developer_dir=(& xcode-select -p|Out-String).Trim()})
    $inputs.Add((Artifact $configuration))
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
}catch{
    $failed=$true
    $log=Join-Path $logsRoot preparation-error.log
    $_.Exception.ToString()|Set-Content -LiteralPath $log -Encoding utf8
    $classification=if($checks.Count -gt 0 -and $checks[$checks.Count-1].status -eq 'missing_environment'){'missing_environment'}else{'failed'}
    $checks.Add([ordered]@{name='macos-preparation';status=$classification;log=(Artifact $log)})
    Write-Host $_.Exception.ToString()
}finally{
    Json (Join-Path $RunRoot native-result.json) ([ordered]@{
        layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase='P0';phase_status='incomplete'
        checks=@($checks.ToArray());inputs=@($inputs.ToArray())
        pending=@('P0 feasibility only; full target/format/failure matrix, original C++ parity, latest Apple SDK policy verification and complete platform gates remain outstanding.','Screen Recording/desktop unavailability is missing environment, never capture acceptance. No TCC database or host permission setting is modified.','Rust dependency and convention gates remain required even when this diagnostic native probe passes.')
    })
}
if($failed -or @($checks|Where-Object status -NE passed).Count){exit 1}
