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
$failed = $false
$referenceArtifacts = $null
$rustArtifacts = $null

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
        if ($Name -eq 'pipewire-fd-error-observation' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'connect_fd_early_error result=null errno=\d+ input_fd_open=[01] observed') {
            throw 'Expected the real native early-error descriptor observation'
        }
        if ($Name -eq 'pipewire-source-negotiation-pods' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'original PipeWire negotiation pods=3 saved') {
            throw 'Expected three original negotiation PODs'
        }
        if ($Name -eq 'rust-pipewire-daemon-shm' -and (Get-Content -LiteralPath $log -Raw) -notmatch 'native Rust PipeWire daemon SHM cycles=20 captures=60 retained=60 passed') {
            throw 'Expected twenty Rust daemon/SHM lifecycles and sixty retained raw RGB samples'
        }
        $status = 'passed'
    } finally {
        $checks.Add([ordered]@{name=$Name;status=$status;log=(Artifact $log)})
        Write-Host "${Name}: $status"
    }
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
        $env:MFS_P1_PIPEWIRE_LIBRARY=(& readlink -f (Join-Path $prefix lib/libpipewire-0.3.so)|Out-String).Trim()
        if($LASTEXITCODE -ne 0 -or -not(Test-Path -LiteralPath $env:MFS_P1_PIPEWIRE_LIBRARY -PathType Leaf)){throw 'Cannot identify the selected native PipeWire library'}
        . (Join-Path $PSScriptRoot prepare-pipewire-reference.ps1)
        $sourceExecutable=Prepare-PipeWireReference $UpstreamRoot
        $referenceArtifacts=Join-Path $RunRoot linux-x64/reference-u/artifacts
        New-Item -ItemType Directory -Path $referenceArtifacts -Force|Out-Null
        Json (Join-Path $referenceArtifacts geometry.json) ([ordered]@{width=28;height=24;stride=84;format='BGR';pattern='asymmetric RGB bars with row/column/channel/producer-serial xor; RGB(0,0) holds 24-bit serial';scope='Fixture serial/retention observed; original API capture time and replay status are not exposed. Repeated markers are diagnostic, not rejected.'})
        $podDirectory=Join-Path $referenceArtifacts negotiation
        $rustArtifacts=Join-Path $RunRoot linux-x64/rust-shm/artifacts
        New-Item -ItemType Directory -Path $rustArtifacts -Force|Out-Null
        $env:MFS_P1_PIPEWIRE_RUST_ARTIFACTS=$rustArtifacts
        $env:MFS_P1_PIPEWIRE_POD_ORACLE=$podDirectory
        Json (Join-Path $rustArtifacts geometry.json) ([ordered]@{width=28;height=24;stride=84;format='RGB';scope='Raw native driver probe; RawAudit preserves RGB and does not implement OpenCV conversion or source/BGR parity.'})
        try { Step pipewire-source-negotiation-pods $sourceExecutable @('--pods',$podDirectory) }
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
    $env:PIPEWIRE_DEBUG = '3'
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
    if(-not $SourceReference){
        Step rust-pipewire-capture cargo @('test','--release','--locked','--test','p0-interop','--features','prototype-linux-pipewire','--','--include-ignored','--test-threads=1','--nocapture')
    }
    Step rust-pipewire-clippy cargo @('clippy','--workspace','--all-targets','--locked','--features','prototype-linux-pipewire,linux-pipewire','--','-D','warnings')
    if($SourceReference){
        # Independent processes; a failed ownership premise must not erase the
        # separate original-source lifecycle evidence or become an overall pass.
        try { Step pipewire-fd-error-observation timeout @('--signal=TERM','--kill-after=5s','30s',$sourceExecutable,'--fd-error',$socket) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step original-pipewire-source-context timeout @('--signal=TERM','--kill-after=5s','180s',$sourceExecutable,$socket,$env:MFS_P0_PIPEWIRE_NODE_ID,$referenceArtifacts) }
        catch { $failed=$true; Write-Host $_.Exception.Message }
        try { Step rust-pipewire-daemon-shm timeout @('--signal=TERM','--kill-after=5s','180s','cargo','test','--release','--locked','--lib','--features','prototype-linux-pipewire,linux-pipewire','backends::linux::pipewire::native::tests::','--','--ignored','--test-threads=1','--nocapture') }
        catch { $failed=$true; Write-Host $_.Exception.Message }
    }
} catch {
    $failed = $true
    $log = Join-Path $logsRoot preparation-error.log
    $_.Exception.ToString() | Set-Content -LiteralPath $log -Encoding utf8
    $checks.Add([ordered]@{name='pipewire-preparation';status='failed';log=(Artifact $log)})
    Write-Host $_.Exception.ToString()
} finally {
    foreach($directory in @($referenceArtifacts,$rustArtifacts)){
        if($directory){Get-ChildItem -LiteralPath $directory -File -Recurse|ForEach-Object{Save $_.FullName}}
    }
    StopOwned source $fixture $fixtureOut $fixtureError
    StopOwned daemon $server $serverOut $serverError
    Json (Join-Path $RunRoot native-result.json) ([ordered]@{
        layout_version=1;run_id=(Split-Path -Leaf $RunRoot);phase=($SourceReference ? 'P1' : 'P0');phase_status='incomplete'
        checks=@($checks.ToArray());inputs=@($inputs.ToArray())
        pending=@('Owned native producer only: portal/desktop capture, DMA-BUF, Wlr, macOS, source/method review and complete phase gates remain outstanding.','Four portable consumer tests and four checked row/storage tests are separate from native capture.',($SourceReference ? 'Original-source context selected: complete fixed PipeWireScreencap header/cpp included unedited in one translation unit, exact extracted screenshot base, private namespace/logging overlays, asymmetric padded-RGB fixture. Twenty FD/init/three-output/inactive/close source cycles require sixty known BGR results and retained images plus FD/thread observations. Separate native Rust daemon/SHM probe requires twenty cycles/sixty raw RGB outputs with exact negotiation POD bytes, queue ownership and held CPU samples. RawAudit preserves RGB, so source/OpenCV BGR parity is not established. Both raw global initializations are balanced by test-only deinit after destruction. Timings are diagnostic; source API freshness/replay is not exposed. Portal FD, owned mapping/DMA-BUF, full format/failure/alias matrices, framework/input and complete P1 remain pending. P0 Rust native probe stays a separate default-pattern entry.' : 'Original-source context/driver probe not selected. The default P0 RGB-bar producer and native Rust stream prototype remain separate from later source/Rust parity.'),'Latest Rust dependency checks are recorded by the parent batch and remain required even if these diagnostic native probes pass.')
    })
}
if($failed -or @($checks | Where-Object status -EQ failed).Count) { exit 1 }
