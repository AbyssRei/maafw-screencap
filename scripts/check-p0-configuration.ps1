# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$DepsRoot,
    [Parameter(Mandatory)][string]$RunRoot,
    [Parameter(Mandatory)][string]$ProductSource,
    [Parameter(Mandatory)][string]$ConfigureTool,
    [Parameter(Mandatory)][string]$RustBatch
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference=$false
$logs=Join-Path $RunRoot logs
New-Item -ItemType Directory -Path $logs | Out-Null
$checks=[Collections.Generic.List[object]]::new()
$inputs=[Collections.Generic.List[object]]::new()
$savedEnvironment=@{}
foreach($name in @('MFS_BUILD_CONFIGURATION','PKG_CONFIG','PKG_CONFIG_PATH','PKG_CONFIG_LIBDIR','WindowsSdkDir','DEVELOPER_DIR','CARGO_HOME','CARGO_TARGET_DIR','CARGO_NET_OFFLINE')) {
    $savedEnvironment[$name]=[Environment]::GetEnvironmentVariable($name)
}
function Artifact([string]$Path) {
    $relative=[IO.Path]::GetRelativePath($ReferenceRoot,$Path).Replace('\','/')
    if($relative.StartsWith('../') -or [IO.Path]::IsPathRooted($relative)){throw 'Configuration evidence escaped reference'}
    return [ordered]@{path=$relative;sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
function Check([string]$Name,[string]$Program,[string[]]$Arguments,[string]$FailureText='') {
    $log=Join-Path $logs "$Name.log"
    "program=$Program`narguments=$($Arguments|ConvertTo-Json -Compress)"|Set-Content -LiteralPath $log -Encoding utf8
    & $Program @Arguments >> $log 2>&1
    $code=$LASTEXITCODE
    $passed=if($FailureText){$code -ne 0 -and (Select-String -LiteralPath $log -SimpleMatch $FailureText -Quiet)}else{$code -eq 0}
    "exit=$code; expected_failure=$([bool]$FailureText)"|Add-Content -LiteralPath $log -Encoding utf8
    $checks.Add([ordered]@{name=$Name;status=$(if($passed){'passed'}else{'failed'});log=(Artifact $log)})
    if(-not $passed){Get-Content -LiteralPath $log -Tail 35|ForEach-Object{Write-Host $_};throw "$Name did not satisfy its observable result"}
    Write-Host "${Name}: passed"
    return $log
}
function Configure([string]$Name,[string[]]$Selections,[string]$Failure='') {
    $log=Check $Name $ConfigureTool (@('configure','--root',$ReferenceRoot,'--offline-inputs',$RustBatch)+$Selections) $Failure
    if($Failure){return}
    $line=@(Get-Content -LiteralPath $log|Where-Object{$_.StartsWith('MFS_BUILD_CONFIGURATION=')})
    if($line.Count -ne 1){throw 'Configuration path missing from successful preparation'}
    $path=$line[0].Substring('MFS_BUILD_CONFIGURATION='.Length)
    $inputs.Add((Artifact $path))
    return $path
}
$off=@('--windows','OFF','--pipewire','OFF','--wlr','OFF','--screen-capture-kit','OFF')
$failed=$false
try {
    Remove-Item Env:MFS_BUILD_CONFIGURATION -ErrorAction SilentlyContinue
    $offConfig=Configure all-off $off
    $env:MFS_BUILD_CONFIGURATION=$offConfig
    Check off-build cargo @('check','--release','--locked','--no-default-features')|Out-Null
    if($IsWindows) {
        Check rejects-gdi-when-windows-off cargo @('check','--release','--locked','--no-default-features','--features','windows-gdi') 'windows-gdi cannot be enabled when the prepared Windows family is OFF'|Out-Null
    }
    $foreignFeature=if($IsMacOS){'prototype-linux-wlr'}else{'prototype-macos'}
    Check rejects-feature-mismatch cargo @('check','--release','--locked','--no-default-features','--features',$foreignFeature) 'Cargo features differ from the explicit prepared configuration'|Out-Null
    Remove-Item Env:MFS_BUILD_CONFIGURATION
    if($IsLinux) {
        $empty=Join-Path $DepsRoot ('runtime/no-pkgconfig-'+[Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $empty | Out-Null
        $env:PKG_CONFIG='pkg-config';$env:PKG_CONFIG_PATH='';$env:PKG_CONFIG_LIBDIR=$empty
        $auto=@('--windows','OFF','--pipewire','AUTO','--wlr','AUTO','--screen-capture-kit','OFF')
        $autoConfig=Configure missing-auto $auto
        $text=Get-Content -LiteralPath $autoConfig -Raw
        if(-not $text.Contains('pipewire=AUTO,disabled') -or -not $text.Contains('wlr=AUTO,disabled')){throw 'AUTO did not disable genuinely undiscoverable native packages'}
        $env:MFS_BUILD_CONFIGURATION=$autoConfig
        Check missing-auto-build cargo @('check','--release','--locked','--no-default-features')|Out-Null
        Remove-Item Env:MFS_BUILD_CONFIGURATION
        Configure missing-pipewire-on @('--windows','OFF','--pipewire','ON','--wlr','OFF','--screen-capture-kit','OFF') 'configuration failed'|Out-Null
        Configure missing-wlr-on @('--windows','OFF','--pipewire','OFF','--wlr','ON','--screen-capture-kit','OFF') 'configuration failed'|Out-Null
    } else {
        $empty=Join-Path $DepsRoot ('runtime/no-sdk-'+[Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $empty | Out-Null
        $sdkVariable=if($IsWindows){'WindowsSdkDir'}else{'DEVELOPER_DIR'}
        [Environment]::SetEnvironmentVariable($sdkVariable,$empty)
        $family=if($IsWindows){'windows'}else{'screen-capture-kit'}
        $auto=@()
        foreach($name in @('windows','pipewire','wlr','screen-capture-kit')){$auto+=@("--$name",$(if($name -eq $family){'AUTO'}else{'OFF'}))}
        $autoConfig=Configure missing-sdk-auto $auto
        if(-not (Get-Content -LiteralPath $autoConfig -Raw).Contains("${family}=AUTO,disabled")){throw 'AUTO did not disable the unavailable explicitly selected SDK'}
        $on=@()
        foreach($name in @('windows','pipewire','wlr','screen-capture-kit')){$on+=@("--$name",$(if($name -eq $family){'ON'}else{'OFF'}))}
        Configure missing-sdk-on $on 'configuration failed'|Out-Null
        [Environment]::SetEnvironmentVariable($sdkVariable,$savedEnvironment[$sdkVariable])
        $env:MFS_BUILD_CONFIGURATION=$autoConfig
        Check missing-sdk-auto-build cargo @('check','--release','--locked','--no-default-features')|Out-Null
        Remove-Item Env:MFS_BUILD_CONFIGURATION
    }
    # Reconstruct a local source consumer with intentionally unbuildable capture
    # code. Compile and run its xtask independently with a separate target dir.
    $consumer=Join-Path $DepsRoot ('configuration-consumers/'+[Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $consumer | Out-Null
    Get-ChildItem -LiteralPath $ProductSource -Force|ForEach-Object{Copy-Item -LiteralPath $_.FullName -Destination $consumer -Recurse}
    $dependency=Get-Content -LiteralPath (Join-Path $ReferenceRoot $RustBatch) -Raw|ConvertFrom-Json
    $lock=@($dependency.inputs|Where-Object{$_.path.EndsWith('/inputs/Cargo.lock')})
    if($lock.Count -ne 1){throw 'Consumer requires one archived lockfile'}
    Copy-Item -LiteralPath (Join-Path $ReferenceRoot $lock[0].path) -Destination (Join-Path $consumer Cargo.lock)
    $vendor=@($dependency.inputs|Where-Object{$_.path.Contains('/inputs/vendor/')})|Select-Object -First 1
    if(-not $vendor){throw 'Consumer requires archived vendor sources'}
    $vendorDirectory=Join-Path $ReferenceRoot ($vendor.path.Substring(0,$vendor.path.IndexOf('/inputs/vendor/'))+'/inputs/vendor')
    $cargoConfig=Join-Path $RunRoot inputs/consumer-cargo.toml
    "[source.crates-io]`nreplace-with = `"mfs-prepared`"`n[source.mfs-prepared]`ndirectory = $($vendorDirectory|ConvertTo-Json -Compress)"|Set-Content -LiteralPath $cargoConfig -Encoding utf8
    $inputs.Add((Artifact $cargoConfig))
    $invalidSource=Join-Path $RunRoot inputs/unbuildable-main.rs
    'compile_error!("P0_CAPTURE_MUST_NOT_BUILD_DURING_PREPARATION"); fn main() {}'|Set-Content -LiteralPath $invalidSource -Encoding utf8
    $inputs.Add((Artifact $invalidSource))
    Copy-Item -LiteralPath $invalidSource -Destination (Join-Path $consumer src/main.rs)
    # With a library present, breaking only main.rs would still let a hidden
    # xtask -> capture library dependency compile. Make the actual library fail.
    if(Test-Path -LiteralPath (Join-Path $consumer src/lib.rs)) {
        Copy-Item -LiteralPath $invalidSource -Destination (Join-Path $consumer src/lib.rs)
    }
    Check consumer-git-init git @('-C',$consumer,'init','--quiet')|Out-Null
    Check consumer-git-add git @('-C',$consumer,'-c','core.longpaths=true','-c','core.autocrlf=false','add','.')|Out-Null
    Check consumer-git-commit git @('-C',$consumer,'-c','user.name=Configuration Fixture','-c','user.email=configuration@invalid','commit','--quiet','-m','test(p0): create unbuildable capture consumer')|Out-Null
    $env:CARGO_HOME=Join-Path $consumer cargo-home
    $env:CARGO_TARGET_DIR=Join-Path $consumer target
    $env:CARGO_NET_OFFLINE='true'
    Push-Location -LiteralPath $consumer
    try {
        Check capture-fixture-is-unbuildable cargo @('--config',$cargoConfig,'check','--offline','--locked','--no-default-features') 'P0_CAPTURE_MUST_NOT_BUILD_DURING_PREPARATION'|Out-Null
        Check prepare-with-unbuildable-capture cargo (@('--config',$cargoConfig,'run','--offline','--locked','--package','xtask','--','configure','--root',$ReferenceRoot,'--offline-inputs',$RustBatch)+$off)|Out-Null
    } finally {Pop-Location}
} catch {
    $failed=$true
    $log=Join-Path $logs harness-error.log
    $_.Exception.ToString()|Set-Content -LiteralPath $log -Encoding utf8
    $checks.Add([ordered]@{name='configuration-harness';status='failed';log=(Artifact $log)})
} finally {
    foreach($name in $savedEnvironment.Keys){[Environment]::SetEnvironmentVariable($name,$savedEnvironment[$name])}
    $record=[ordered]@{layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase='P0';phase_status='incomplete';checks=@($checks.ToArray());inputs=@($inputs.ToArray());pending=@('P0 build configuration checks only; native ON capture runs and complete platform/phase gates remain separate.')}
    $file=[IO.File]::Open((Join-Path $RunRoot native-result.json),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write)
    try{$file.Write([Text.Encoding]::UTF8.GetBytes(($record|ConvertTo-Json -Depth 32)+"`n"))}finally{$file.Dispose()}
}
if($failed -or @($checks|Where-Object status -EQ failed).Count){exit 1}
