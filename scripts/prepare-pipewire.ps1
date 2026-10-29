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
        if($code -eq 77){
            $status='missing_environment'
            Get-Content -LiteralPath $log -Tail 10 | ForEach-Object {Write-Host $_}
            throw "$Name requires a missing native environment; see $log"
        }
        if ($code -ne 0) {
            Get-Content -LiteralPath $log -Tail 50 | ForEach-Object { Write-Host $_ }
            throw "$Name exited with $code; see $log"
        }
        if ($Name -eq 'rust-pipewire-consumer-control' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 4 passed; 0 failed; 1 ignored;') {
            throw 'Expected four portable consumer tests; the external source oracle remains explicitly ignored on this Linux entry'
        }
        if ($Name -eq 'rust-pipewire-row-ownership' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 4 passed; 0 failed; 0 ignored;') {
            throw 'Expected four checked row/storage tests, separate from native stream capture'
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
        if ($Name -eq 'pipewire-fd-error-observation' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'connect_fd_early_error result=null errno=\d+ input_fd_open=[01] observed') {
            throw 'Expected the real native early-error descriptor observation'
        }
        if ($Name -eq 'pipewire-fd-add-io-error-observation' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'connect_fd_add_io_error result=null errno=\d+ input_fd_open=[01] after_teardown_fd_open=[01] intercepted_calls=1 observed') {
            throw 'Expected the actual native late connect_fd error with one explicit add_io interception'
        }
        if ($Name -eq 'native-joined-task-observation' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'native joined-task observation cycles=10000 residual_enumerations=\d+ observed') {
            throw 'Expected the separate native pthread-join/proc visibility observation'
        }
        if ($Name -eq 'native-task-diagnostic-fixtures' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'native task diagnostic empty_stream_failure=1 empty_file=1 bytes_file=1 missing_file=1 real_output_failure=1 passed') {
            throw 'Expected empty/readable/missing task inputs and a real diagnostic write-error control'
        }
        if ($Name -eq 'pipewire-source-negotiation-pods' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire negotiation pods=3 saved') {
            throw 'Expected three original negotiation PODs'
        }
        if ($Name -eq 'pipewire-source-format-fixtures' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire format fixtures=28 formats=7 choice_none=14 linear_modifier=14 saved') {
            throw 'Expected twenty-eight original native POD/parser/conversion fixtures'
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
        if($Name -eq 'linux-config-rust-parity') {
            $observed=Get-Content -LiteralPath $log -Raw
            if($observed -notmatch 'Rust Linux config cases=946 accepted=426 rejected=520 exact_uuid_metadata=1 input_actions=0 passed' -or $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'Expected all original/Rust configuration acceptance, UUID and metadata comparisons in one actual test'
            }
        }
        if($Name -eq 'linux-config-manager-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 8 passed; 0 failed; 1 ignored;') {
            throw 'Expected eight manager controls, with the explicit original-source model still ignored on this default filter'
        }
        if ($Name -eq 'rust-pipewire-manager-controls' -and (Get-Content -LiteralPath $log -Raw) -notmatch '(?m)^test result: ok\. 2 passed; 0 failed; 0 ignored;') {
            throw 'Expected the two native None/unimplemented-method and unsafe-FD boundary controls'
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
    if ($release.upcoming_release) { throw 'PipeWire latest release is not yet released' }
    $commit = $release.commit.id
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
        $managerTest=$null
        try {
            $env:MFS_P1_LINUX_MANAGER_MODEL_ARTIFACTS=Join-Path $referenceArtifacts manager-model
            $managerTest=Capture-TestProgram linux-manager-model-rust-build
            Step linux-manager-source-model timeout @('--signal=TERM','--kill-after=5s','180s',$managerTest,'backends::linux::manager::tests::matches_fixed_source_manager_reconnect_forwarding_and_uuid','--exact','--ignored','--test-threads=1','--nocapture')
        } catch { $failed=$true; Write-Host $_.Exception.Message }
        if($managerTest){
            try { Step linux-config-rust-parity timeout @('--signal=TERM','--kill-after=5s','90s',$managerTest,'backends::linux::manager::configuration::tests::matches_original_configuration_creation_uuid_and_metadata_bytes','--exact','--ignored','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            finally {if(Test-Path -LiteralPath $env:MFS_P1_LINUX_CONFIG_RUST_ARTIFACTS){Get-ChildItem -LiteralPath $env:MFS_P1_LINUX_CONFIG_RUST_ARTIFACTS -Recurse -File|ForEach-Object{Save $_.FullName}}}
            try { Step linux-config-manager-controls timeout @('--signal=TERM','--kill-after=5s','30s',$managerTest,'backends::linux::manager::tests::','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
            try { Step rust-pipewire-manager-controls timeout @('--signal=TERM','--kill-after=5s','30s',$managerTest,'backends::linux::manager::native::tests::','--test-threads=1','--nocapture') }
            catch { $failed=$true; Write-Host $_.Exception.Message }
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
        # End the ordinary producer/route premise before starting a separate
        # kernel DMA producer. Product stream flags and routing policy stay U.
        StopOwned route-supervisor $routeSupervisor $routeOut $routeError
        $routeSupervisor=$null
        StopOwned source $fixture $fixtureOut $fixtureError
        $fixture=$null
        . (Join-Path $PSScriptRoot validate-pipewire-dma-stream.ps1)
        try { Invoke-PipeWireDmaStream }
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
