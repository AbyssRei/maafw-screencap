# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot ../../scripts/prepare-apple-toolchain.ps1)

function Installed([string]$Version,[string]$Build,[string]$Path,[int]$Exit=0) {
    [pscustomobject]@{version=$Version;build=$Build;developer_dir=$Path;exit=$Exit;identity="Xcode $Version Build version $Build"}
}
$release=[pscustomobject]@{version='27.2';build='27B52'}
$older=Installed '27.1' '27A9275' '/installed/older'
$rc=Installed '27.2' '27B48' '/installed/runner-default'
$latest=Installed '27.2' '27B52' '/installed/latest'
if (@(Select-InstalledAppleToolchain @($older,$rc) $release latest '').Count) {throw 'Local latest policy accepted a mismatching build'}
if ((Select-InstalledAppleToolchain @($older,$latest) $release latest '').developer_dir -ne $latest.developer_dir) {throw 'Formal build selection differs'}
if ((Select-InstalledAppleToolchain @($older,$rc) $release preinstalled $older.developer_dir).developer_dir -ne $older.developer_dir) {throw 'CI rejected its usable older selected toolchain'}
if ((Select-InstalledAppleToolchain @($older,$rc) $release preinstalled $rc.developer_dir).build -ne '27B48') {throw 'CI rejected/relabelled the installed RC build'}
if ((Select-InstalledAppleToolchain @($older) $null preinstalled '').developer_dir -ne $older.developer_dir) {throw 'Unavailable release lookup blocked a usable installed Xcode'}
if (@(Select-InstalledAppleToolchain @((Installed '' '' '/broken' 1)) $release preinstalled '').Count) {throw 'Unusable Xcode accepted'}
foreach($version in @('4.2.0','4.4.3')) {
    $caps=[pscustomobject]@{version=[pscustomobject]@{string=$version};generators=@([pscustomobject]@{name='Visual Studio 18 2026'})}
    if (-not(Test-CMakeGenerator $caps 'Visual Studio 18 2026')) {throw 'Installed generator rejected because of its version'}
    if (Test-CMakeGenerator $caps Ninja) {throw 'Missing generator was accepted'}
}
$expected=[pscustomobject]@{vs_version='18.11.12328.437';sdk=[pscustomobject]@{version='10.0.28000.2957'}}
$vs=[pscustomobject]@{installationVersion='18.8.10000.1'}
$sdks=@([pscustomobject]@{DisplayName='Windows Software Development Kit - Windows 10.0.26100.9457'})
$ready=Get-WindowsToolchainReadiness preinstalled $vs $sdks @('10.0.26100.0') $expected
if(!$ready.vs -or !$ready.sdk){throw 'CI rejected older usable VS/SDK'}
$strict=Get-WindowsToolchainReadiness latest $vs $sdks @('10.0.26100.0') $expected
if($strict.vs -or $strict.sdk){throw 'Local latest requirement was weakened'}
$missing=Get-WindowsToolchainReadiness preinstalled $null @() @() $expected
if($missing.vs -or $missing.sdk){throw 'Missing native tools accepted'}
$oldPolicy=$env:MFS_TOOLCHAIN_POLICY
try {
    $env:MFS_TOOLCHAIN_POLICY='preinstalled'
    if((Get-ToolchainPolicy) -ne 'preinstalled'){throw 'CI policy not selected'}
    $env:MFS_TOOLCHAIN_POLICY='typo'
    $rejected=$false;try{Get-ToolchainPolicy|Out-Null}catch{$rejected=$true}
    if(!$rejected){throw 'Unknown toolchain policy accepted'}
}finally{$env:MFS_TOOLCHAIN_POLICY=$oldPolicy}
$rustScript=(Resolve-Path (Join-Path $PSScriptRoot ../../scripts/prepare-rust-toolchain.ps1)).Path.Replace("'","''")
foreach($case in @(@('preinstalled',$true,0),@('preinstalled',$false,1),@('latest',$true,1))) {
    $control=@'
$env:MFS_TOOLCHAIN_POLICY='__POLICY__'
$global:mockHasNightly=__INSTALLED__
$global:mockRustupCalls=[Collections.Generic.List[string]]::new()
function global:rustup {
    $global:mockRustupCalls.Add(($args -join ' '))
    $global:LASTEXITCODE=0
    if($args[0] -eq 'run') {
        if(!$global:mockHasNightly){$global:LASTEXITCODE=1;return}
        'rustc mock-nightly'
    }elseif($args[0] -eq 'update'){$global:mockHasNightly=$true}
    elseif($args[0] -eq 'component' -and $args[1] -eq 'list'){'rustfmt-mock';'clippy-mock'}
}
& '__SCRIPT__'
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
'MOCK_UPDATES='+@($global:mockRustupCalls|Where-Object{$_ -like 'update *'}).Count
'MOCK_PREPARATION_COMPLETED=1'
'@.Replace('__POLICY__',$case[0]).Replace('__INSTALLED__',$(if($case[1]){'$true'}else{'$false'})).Replace('__SCRIPT__',$rustScript)
    $output=(& (Get-Command pwsh).Source -NoProfile -Command $control 2>&1|Out-String)
    if($LASTEXITCODE -ne 0 -or !$output.Contains('MOCK_PREPARATION_COMPLETED=1') -or !$output.Contains('MOCK_UPDATES='+$case[2])){
        throw "Rust toolchain orchestration control differs: $output"
    }
}
Write-Output 'Toolchain policy controls passed: older installed tools, exact build identity, advisory lookup absence, real capability rejection and local latest policy'
