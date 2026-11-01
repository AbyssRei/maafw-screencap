# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot ../../scripts/native-shared-inputs.ps1)
$testRoot=Join-Path ([IO.Path]::GetTempPath()) ('mfs-shared-input-controls-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot|Out-Null
function Write-Fixture([string]$Name,$Value) {
    $path=Join-Path $testRoot $Name
    New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force|Out-Null
    [IO.File]::WriteAllText($path,($Value|ConvertTo-Json -Depth 20)+"`n")
    return [pscustomobject]@{path=$Name;sha256=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}
}
function Require-Rejected([scriptblock]$Operation,[string]$Label) {
    $rejected=$false
    try {& $Operation|Out-Null} catch {$rejected=$true}
    if(!$rejected){throw "Invalid shared fixture accepted: $Label"}
}
function Workflow-Block([string]$Name) {
    $lines=[IO.File]::ReadAllLines((Join-Path $PSScriptRoot ../../.github/workflows/p0.yml))
    $start=[Array]::IndexOf($lines,"      - name: $Name")
    if($start -lt 0){throw "Workflow step absent: $Name"}
    $end=$start+1
    while($end -lt $lines.Count -and !$lines[$end].StartsWith('      - name: ')){$end++}
    $run=$start+1
    while($run -lt $end -and $lines[$run] -cne '        run: |'){$run++}
    if($run -eq $end){throw "Workflow block absent: $Name"}
    return (($lines[($run+1)..($end-1)]|ForEach-Object{if($_.StartsWith('          ')){$_.Substring(10)}else{$_}})-join "`n")
}
try {
    $producerPath='runs/p1-print-window-windows-100/manifest.json'
    $base='runs/p1-print-window-windows-100/inputs/'
    $inputs=@()
    foreach($name in @('native-configuration.json','OpenCV-CMakeCache.txt','opencv-downloads.tar',
        'opencv-release.json','OpenCV-resolution.json','OpenCV-source.tar','maautils-release-policy.json',
        'MaaUtils-resolution.json','MaaUtils-source.tar','ZLIB-release.json','ZLIB-resolution.json','ZLIB-source.tar',
        'PNG-public-release.html','PNG-release.json','PNG-resolution.json','PNG-source.tar','cmake-preinstalled.json')) {
        $inputs+=Write-Fixture ($base+$name) ([ordered]@{fixture=$name;native_tool_or_capture_evidence=$false})
    }
    $boostArchive=Write-Fixture ($base+'boost-test-cmake.tar.xz') 'mock source archive bytes'
    $inputs+=$boostArchive
    $inputs+=Write-Fixture ($base+'boost-release.json') ([ordered]@{assets=@([ordered]@{name='boost-test-cmake.tar.xz';digest=('sha256:'+$boostArchive.sha256)})})
    $rustInputs=@()
    foreach($name in @('Cargo.lock','toolchain.txt','dependency-target.txt','vendor/mock/source.rs')) {
        $rustInputs+=Write-Fixture ('runs/p0-windows-99/inputs/'+$name) ('mock-'+$name)
    }
    $rust=[ordered]@{layout_version=1;checks=@([ordered]@{name='dependency-update';status='passed'});inputs=$rustInputs}
    $inputs+=Write-Fixture 'runs/p0-windows-99/manifest.json' $rust
    # A pixel failure after successful preparation must remain failed, while
    # independent consumers can use its complete, explicitly selected inputs.
    $run=[ordered]@{layout_version=1;phase='P1';phase_status='incomplete';run_id='p1-print-window-windows-100';inputs=$inputs;checks=@([ordered]@{name='native-capture';status='failed'})}
    $producer=Write-Fixture $producerPath $run
    if((Get-SharedNativeSeed $testRoot $producer) -cne $producerPath){throw 'Complete preparation with capture failure was rejected'}

    $run.inputs=@($inputs|Where-Object{!$_.path.EndsWith('/PNG-release.json')})
    $partial=Write-Fixture $producerPath $run
    if(Get-SharedNativeSeed $testRoot $partial){throw 'Missing PNG release selected as a shared input batch'}
    $run.inputs=$inputs
    $producer=Write-Fixture $producerPath $run
    $environment=Join-Path $testRoot github-env.txt
    [void](Write-Fixture 'manifest.json' ([ordered]@{runs=@($producer)}))
    Publish-SharedNativeSeed $testRoot @($producerPath) $environment
    if(Test-Path $environment){throw 'Historical producer was silently reused'}
    Publish-SharedNativeSeed $testRoot @() $environment
    if((Get-Content $environment).Trim() -cne "MFS_WINDOWS_SHARED_NATIVE_INPUTS=$producerPath"){throw 'Exact selected batch not exported'}

    $failedLater=Write-Fixture 'runs/p1-manager-windows-101/manifest.json' ([ordered]@{inputs=@();checks=@([ordered]@{name='dependency-download';status='failed'})})
    [void](Write-Fixture 'manifest.json' ([ordered]@{runs=@($producer,$failedLater)}))
    Remove-Item -LiteralPath $environment
    Publish-SharedNativeSeed $testRoot @() $environment
    if((Get-Content $environment).Trim() -cne "MFS_WINDOWS_SHARED_NATIVE_INPUTS=$producerPath"){throw 'Later incomplete manager replaced the observed producer'}
    Require-Rejected {Get-SharedNativeSeed $testRoot $failedLater} wrong_producer
    $bad=[pscustomobject]@{path='runs/../outside/manifest.json';sha256=$producer.sha256}
    Require-Rejected {Read-SharedNativeArtifact $testRoot $bad} escaped_path
    $bad=[pscustomobject]@{path=$producer.path;sha256=('0'*64)}
    Require-Rejected {Read-SharedNativeArtifact $testRoot $bad} changed_manifest
    $png=$inputs|Where-Object{$_.path.EndsWith('/PNG-source.tar')}
    [IO.File]::WriteAllText((Join-Path $testRoot $png.path),'changed')
    Require-Rejected {Get-SharedNativeSeed $testRoot $producer} changed_input
    # Repair only the self-owned mock fixture for the next independent control.
    $fixed=Write-Fixture $png.path ([ordered]@{fixture='PNG-source.tar';native_tool_or_capture_evidence=$false})
    if($fixed.sha256 -cne $png.sha256){throw 'Mock fixture repair changed its identity'}
    $rust.checks[0].status='failed'
    $failedRust=Write-Fixture 'runs/p0-windows-99/manifest.json' $rust
    $run.inputs=@($inputs|Where-Object{!$_.path.EndsWith('/p0-windows-99/manifest.json')})+@($failedRust)
    $producer=Write-Fixture $producerPath $run
    if(Get-SharedNativeSeed $testRoot $producer){throw 'Failed Rust preparation was selected'}
    $rust.checks[0].status='passed'
    $fixedRust=Write-Fixture 'runs/p0-windows-99/manifest.json' $rust
    $run.inputs=@($inputs|Where-Object{!$_.path.EndsWith('/p0-windows-99/manifest.json')})+@($fixedRust)
    $producer=Write-Fixture $producerPath $run

    # Execute the actual workflow snippets in child shells. Only cargo and
    # Windows installation are replaced in this self-owned mock workspace.
    $workspace=Join-Path $testRoot workspace
    New-Item -ItemType Directory -Path (Join-Path $workspace scripts) -Force|Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot ../../scripts/native-shared-inputs.ps1) -Destination (Join-Path $workspace scripts/native-shared-inputs.ps1)
    [IO.File]::WriteAllText((Join-Path $workspace scripts/prepare-windows-toolchain.ps1),'$global:LASTEXITCODE=0; Write-Output "mock tool presence only; no installer"')
    $mock=[ordered]@{root=$testRoot;workspace=$workspace;producer=$producer;rust=$fixedRust;native_exit=17;environment_file=$environment}
    $mockPath=Join-Path $testRoot mock.json
    [IO.File]::WriteAllText($mockPath,($mock|ConvertTo-Json -Depth 12))
    $childPath=Join-Path $testRoot child.ps1
    [IO.File]::WriteAllText($childPath,@'
param([string]$Configuration,[string]$Block,[string]$ArgumentsPath,[string]$Shared)
$ErrorActionPreference='Stop'
$PSNativeCommandUseErrorActionPreference=$true
$config=Get-Content -LiteralPath $Configuration -Raw|ConvertFrom-Json
$env:MFS_REFERENCE_ROOT=$config.root
$env:MFS_WINDOWS_SHARED_NATIVE_INPUTS=$Shared
$env:GITHUB_ENV=$config.environment_file
Set-Location -LiteralPath $config.workspace
function global:cargo {
    [IO.File]::WriteAllText($ArgumentsPath,($args|ConvertTo-Json -Compress))
    if($args -contains 'print-window') {
        [IO.File]::WriteAllText((Join-Path $config.root manifest.json),(@{runs=@($config.rust,$config.producer)}|ConvertTo-Json -Depth 12))
        # A real nonzero native child also exercises PowerShell's optional
        # native-error preference; the workflow must retain control of exit17.
        & (Get-Command pwsh).Source -NoProfile -Command 'exit 17'
        $global:LASTEXITCODE=$config.native_exit
    }else{$global:LASTEXITCODE=0}
}
& ([scriptblock]::Create([IO.File]::ReadAllText($Block)))
exit $LASTEXITCODE
'@)
    Remove-Item -LiteralPath $environment
    [void](Write-Fixture 'manifest.json' ([ordered]@{runs=@($fixedRust)}))
    $block=Join-Path $testRoot workflow-print.ps1
    [IO.File]::WriteAllText($block,(Workflow-Block 'Validate Windows GDI and PrintWindow migration and native interop'))
    & (Get-Command pwsh).Source -NoProfile -File $childPath $mockPath $block (Join-Path $testRoot print-args.json) '' | Out-Null
    if($LASTEXITCODE -ne 17 -or !(Test-Path $environment) -or (Get-Content $environment).Trim() -cne "MFS_WINDOWS_SHARED_NATIVE_INPUTS=$producerPath") {
        throw 'Actual producer workflow block lost the native failure or explicit dependency seed'
    }
    $withoutPreference=(Workflow-Block 'Validate Windows GDI and PrintWindow migration and native interop').Replace('$PSNativeCommandUseErrorActionPreference = $false','')
    $badBlock=Join-Path $testRoot workflow-native-preference-negative.ps1
    [IO.File]::WriteAllText($badBlock,$withoutPreference)
    Remove-Item -LiteralPath $environment
    [void](Write-Fixture 'manifest.json' ([ordered]@{runs=@($fixedRust)}))
    $negativeOutput=(& (Get-Command pwsh).Source -NoProfile -File $childPath $mockPath $badBlock (Join-Path $testRoot preference-negative-args.json) '' 2>&1|Out-String)
    if($LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq 17 -or (Test-Path $environment)) {
        throw "Native-error preference negative control failed: $negativeOutput"
    }
    $commands=@(
        @('Validate partial ScreenDC migration','xtask','upstream-parity','screen-dc'),
        @('Validate partial DesktopDup and window migration','xtask','upstream-parity','desktop-dup'),
        @('Compare portable PipeWire consumer model with exact source','xtask','upstream-model','pipewire'),
        @('Validate partial FramePool migration and original caller contexts','xtask','upstream-parity','frame-pool')
    )
    $number=0
    foreach($command in $commands) {
        foreach($sharedPath in @('',$producerPath)) {
            $number++
            $block=Join-Path $testRoot "workflow-consumer-$number.ps1"
            $argsPath=Join-Path $testRoot "consumer-args-$number.json"
            [IO.File]::WriteAllText($block,(Workflow-Block $command[0]))
            & (Get-Command pwsh).Source -NoProfile -File $childPath $mockPath $block $argsPath $sharedPath | Out-Null
            $expected=@($command[1..3])+@($(if($sharedPath){'--offline-inputs';$sharedPath}))
            $actual=@(Get-Content -LiteralPath $argsPath -Raw|ConvertFrom-Json)
            if($LASTEXITCODE -ne 0 -or ($actual|ConvertTo-Json -Compress) -cne ($expected|ConvertTo-Json -Compress)) {
                throw "Actual independent consumer workflow arguments differ: $($command[0])"
            }
        }
    }
    Write-Output 'Shared native input controls passed: exact new producer, incomplete/changed/escaped inputs, retained native failure and no historical search; native_capture=0'
}finally{
    $resolved=[IO.Path]::GetFullPath($testRoot)
    $expected=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar)+[IO.Path]::DirectorySeparatorChar
    if(!$resolved.StartsWith($expected,[StringComparison]::OrdinalIgnoreCase) -or (Split-Path -Leaf $resolved) -notlike 'mfs-shared-input-controls-*'){throw 'Mock cleanup path escaped its temporary root'}
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
