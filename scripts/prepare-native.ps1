# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][string]$DepsRoot,
    [Parameter(Mandatory)][string]$RunRoot,
    [Parameter(Mandatory)][string]$UpstreamRoot,
    [Parameter(Mandatory)][string]$ConfigureTool,
    [Parameter(Mandatory)][string]$RustBatch,
    [switch]$Desktop,
    [switch]$GdiParity,
    [switch]$PrintWindowParity,
    [switch]$ScreenDcParity,
    [switch]$FramePoolContext,
    [switch]$FramePoolParity,
    [string]$OfflineManifest = '',
    [string]$CargoConfig = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false
$productRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $productRoot
$inputsRoot = Join-Path $RunRoot inputs
$configRoot = Join-Path $RunRoot windows-x64-vs2026-msbuild-release
$logsRoot = Join-Path $configRoot logs
$checks = [Collections.Generic.List[object]]::new()
$inputs = [Collections.Generic.List[object]]::new()
$pending = [Collections.Generic.List[string]]::new()
$nativeMissingEnvironment = $false
if ($PrintWindowParity -and -not $GdiParity) { throw 'PrintWindow parity requires the shared explicit GDI adapter/regression target.' }
if ($ScreenDcParity -and -not $GdiParity) { throw 'ScreenDC parity requires the shared explicit GDI adapter/regression target.' }
if ($FramePoolParity -and -not $GdiParity) { throw 'FramePool parity requires the explicit shared image adapter/regression target.' }
$framePoolSource = $FramePoolContext -or $FramePoolParity
$screenEvidence = ''
$frameContextEvidence = ''
if ($FramePoolContext -and (-not $Desktop -or $GdiParity)) { throw 'FramePool context observation requires its independent explicit desktop entry.' }
if ($ScreenDcParity) {
    $pending.Add('ScreenDC is a partial P1 component. Exact-original window helpers, scripted capture boundaries, real DC/bitmap/conversion fault checks and visible pixels remain distinct evidence. Original source-only presentation controls retain before/after outcomes. The owned topmost/transition-disabled scene records point/root visibility around capture, without changing capture/helper calls or waits and without retry. Occlusion, remaining query failures, movement/resize, actual nondefault depth and complete method/manager acceptance remain outstanding. All previous black/gray pixel, resource-baseline and harness I/O failures remain archived.')
}
if ($PrintWindowParity) {
    $pending.Add('PrintWindow is a partial P1 component. Pseudo-minimize helper/wrapper migration and 22 deterministic original-body scripts are implemented; fake scheduling does not prove real thread lifetime. Dedicated UI-owner/message-loop fixtures, joined shutdown, layered/nonlayered stop/inactive and target-loss lifecycles are separate native checks. Earlier same-thread layered failures remain archived; capture/helper code is unchanged by the fixture correction. Foreground recovery has its own exact native case: missing foreground authority remains missing_environment, never success. Complete foreground/window-state coverage and the earlier independent original-popup black-bitmap failure remain unresolved. GetDIBits deselection is a native-precondition adaptation, not P3 modernization. Fixture recovery is distinct from product cleanup success.')
}
if ($FramePoolParity) {
    $pending.Add('FramePool is a partial P1 migration, not a backend/phase exit. Four separate Rust caller contexts compare original image bytes, clone owners, native close, callback event, apartment and DPI; twenty Rust capture/window/device lifecycles retain all images through completed dispatcher shutdown. Full repeated source/fault comparisons, native failure/initialization-exception/cleanup, complete polling/cache/resize/permission/format, full pseudo-wrapper and manager matrices remain pending. The migrated pseudo wrapper shares the original helper; five isolated source image/state scenarios feed twenty Rust lifecycles. Source known-color failures remain failed and make the batch nonpassing, even when subsequent byte/state diagnostics match; a local alpha-zero black result is not capture acceptance. Safety target-row copy and scoped factory ownership differ from direct mapped conversion/global projection caching and are explicitly counted/classified; old WGC unload AV remains unresolved.')
} elseif ($FramePoolContext) {
    $pending.Add('This P1 task observes only the unmodified C++ FramePool source in four isolated caller contexts. It implements no Rust FramePool backend or SDK and does not accept Rust parity, full pixels/format/resize/permission/cache/timeout semantics or P1. Process-isolated factory/MTA state, known interior pixels and COM observations remain specific to this native environment.')
} elseif ($GdiParity) {
    $pending.Add('P1 remains incomplete. GDI fault probes and direct window-state cases do not establish actual nondefault display depth or a complete backend/phase pass. Manager/forwarding and every other backend remain outstanding. Fixture recovery is separate from recorded implementation cleanup failures.')
} else {
    $pending.Add('This P0 native task does not independently accept the complete preparation phase; associate the source/method/license and other platform records in the external exit assessment.')
}
$offline = -not [string]::IsNullOrEmpty($OfflineManifest)
$prepared = if ($offline) { Get-Content -LiteralPath $OfflineManifest -Raw | ConvertFrom-Json } else { $null }
$sourceIdentities = @{}
$replayRoot = Join-Path $DepsRoot ('replay/' + (Split-Path -Leaf $RunRoot).Substring((Split-Path -Leaf $RunRoot).Length - 12))

function Write-NewJson([string]$Path, [object]$Value) {
    $bytes = [Text.Encoding]::UTF8.GetBytes(($Value | ConvertTo-Json -Depth 40) + "`n")
    $stream = [IO.File]::Open($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
    try { $stream.Write($bytes) } finally { $stream.Dispose() }
}

function Artifact([string]$Path) {
    $relative = [IO.Path]::GetRelativePath($ReferenceRoot, $Path).Replace('\', '/')
    if ($relative.StartsWith('../') -or [IO.Path]::IsPathRooted($relative)) {
        throw "Artifact is outside reference: $Path"
    }
    return [ordered]@{ path = $relative; sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
}

function Save-Input([string]$Path) { $inputs.Add((Artifact $Path)) }

function Select-SdkReleases([string]$DirectoryVersion, [object[]]$Products) {
    $directory = [version]$DirectoryVersion
    $matching = foreach ($product in $Products) {
        if (-not $product.PSObject.Properties['DisplayName'] -or
            -not $product.PSObject.Properties['DisplayVersion']) { continue }
        if ($product.DisplayName -cmatch '^Windows Software Development Kit - Windows (\d+\.\d+\.\d+\.\d+)$') {
            $release = [version]$Matches[1]
            if ($release.Major -eq $directory.Major -and $release.Minor -eq $directory.Minor -and $release.Build -eq $directory.Build) {
                [pscustomobject][ordered]@{sdk_release=$release.ToString();product_version=[string]$product.DisplayVersion}
            }
        }
    }
    return @($matching | Sort-Object sdk_release,product_version -Unique)
}

function Prepared-File([string]$Name) {
    $matches = @($prepared.inputs | Where-Object { $_.path.EndsWith("/inputs/$Name") })
    if ($matches.Count -ne 1) { throw "Prepared input $Name is missing or ambiguous" }
    $path = Join-Path $ReferenceRoot $matches[0].path
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $matches[0].sha256) {
        throw "Prepared input changed: $Name"
    }
    return $path
}

function Step([string]$Name, [string]$Program, [string[]]$Arguments) {
    Write-Host "Running $Name ..."
    $logPath = Join-Path $logsRoot "$Name.log"
    $status = 'failed'
    try {
        "program=$Program`narguments=$($Arguments | ConvertTo-Json -Compress)" | Set-Content -LiteralPath $logPath -Encoding utf8
        & $Program @Arguments >> $logPath 2>&1
        $exitCode = $LASTEXITCODE
        "exit=$exitCode" | Add-Content -LiteralPath $logPath -Encoding utf8
        if ($Name -eq 'pseudo-foreground-parity') {
            $observed = Get-Content -LiteralPath $logPath -Raw
            if ($exitCode -eq 101 -and
                $observed.Contains('MFS_MISSING_ENVIRONMENT:foreground_control') -and
                $observed -match '(?m)^test result: FAILED\. 0 passed; 1 failed; 0 ignored;' -and
                $observed -notmatch 'panicked at') {
                $status = 'missing_environment'
                $script:nativeMissingEnvironment = $true
                $pending.Add('The exact original/Rust foreground monitor case could not complete: native SetForegroundWindow returned zero while its own target was active and another process retained foreground. No activation bypass or input injection was used. The case remains unaccepted; other independent checks continue.')
                Write-Host "$Name`: missing_environment"
                return
            }
            if ($exitCode -eq 0 -and $observed -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'The exact foreground case did not execute one native test'
            }
        }
        if ($Name -match '^frame-pool-rust-(uninitialized|mta|mta-dispatcher|sta|lifecycles|pseudo)$' -and $exitCode -eq 0) {
            if ((Get-Content -LiteralPath $logPath -Raw) -notmatch '(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;') {
                throw 'The exact native FramePool case did not execute one test'
            }
        }
        if ($exitCode -eq 77 -and $Name -eq 'windows-toolchain-current') { $status = 'missing_environment' }
        if ($exitCode -ne 0) { throw "$Name exited with $exitCode; see $logPath" }
        $status = 'passed'
    }
    catch {
        $_.Exception.Message | Add-Content -LiteralPath $logPath -Encoding utf8
        throw
    }
    finally {
        $checks.Add([ordered]@{ name = $Name; status = $status; log = (Artifact $logPath) })
        Write-Host "${Name}: $status"
    }
}

function Api([string]$Endpoint) {
    $response = & gh api $Endpoint
    if ($LASTEXITCODE -ne 0) { throw "GitHub API query failed: $Endpoint" }
    return ($response | ConvertFrom-Json)
}

function Git-Text([string]$Directory, [string[]]$Arguments) {
    $response = & git -C $Directory @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Git query failed: $($Arguments -join ' ')" }
    return ($response | Out-String).Trim()
}

function Source([string]$Repository, [string]$Ref, [string]$Label) {
    if ($offline) {
        $metadata = Prepared-File "$Label-resolution.json"
        $resolution = Get-Content -LiteralPath $metadata -Raw | ConvertFrom-Json
        if ($resolution.repository -ne "https://github.com/$Repository.git" -or $resolution.requested_ref -ne $Ref) {
            throw "Prepared $Label source identity does not match its declared input"
        }
        $archive = Prepared-File "$Label-source.tar"
        Save-Input $metadata
        Save-Input $archive
        $sourcePath = Join-Path $replayRoot $Label
        New-Item -ItemType Directory -Path $sourcePath -Force | Out-Null
        Step "$Label-source-replay" tar @('-xf', $archive, '-C', $sourcePath)
        $sourceIdentities[$Label] = $resolution.commit
        return $sourcePath
    }
    $commit = Api "repos/$Repository/commits/$Ref"
    $identity = $commit.sha
    $sourcePath = Join-Path $DepsRoot "native-sources/$Label-$identity"
    New-Item -ItemType Directory -Path (Split-Path -Parent $sourcePath) -Force | Out-Null
    if (-not (Test-Path -LiteralPath $sourcePath)) {
        Step "$Label-init" git @('init', '--quiet', $sourcePath)
        Step "$Label-remote" git @('-C', $sourcePath, 'remote', 'add', 'origin', "https://github.com/$Repository.git")
        Step "$Label-fetch" git @('-C', $sourcePath, 'fetch', '--no-tags', '--depth=1', 'origin', $identity)
        Step "$Label-checkout" git @('-C', $sourcePath, '-c', 'core.autocrlf=false', 'checkout', '--quiet', '--detach', 'FETCH_HEAD')
    }
    if ((Git-Text $sourcePath @('rev-parse', 'HEAD')) -ne $identity -or
        (Git-Text $sourcePath @('status', '--porcelain', '--untracked-files=all'))) {
        throw "Cached $Label source differs from this batch's resolved commit"
    }
    $metadata = Join-Path $inputsRoot "$Label-resolution.json"
    Write-NewJson $metadata ([ordered]@{
        repository = "https://github.com/$Repository.git"; requested_ref = $Ref; commit = $identity
        tree = (Git-Text $sourcePath @('rev-parse', 'HEAD^{tree}'))
        queried_at = [DateTime]::UtcNow.ToString('o')
    })
    Save-Input $metadata
    $archive = Join-Path $inputsRoot "$Label-source.tar"
    Step "$Label-archive" git @('-C', $sourcePath, 'archive', '--format=tar', '--output', $archive, $identity)
    Save-Input $archive
    $sourceIdentities[$Label] = $identity
    return $sourcePath
}

function Boost-Source {
    $release = if ($offline) { Get-Content -LiteralPath (Prepared-File boost-release.json) -Raw | ConvertFrom-Json } else { Api 'repos/boostorg/boost/releases/latest' }
    $metadata = Join-Path $inputsRoot boost-release.json
    Write-NewJson $metadata $release
    Save-Input $metadata
    $assets = @($release.assets | Where-Object name -Match '-cmake.tar.xz$')
    if ($assets.Count -ne 1) { throw 'Boost release must provide exactly one complete CMake source archive' }
    $asset = $assets[0]
    $archive = Join-Path $DepsRoot "downloads/$($asset.name)"
    if (-not (Test-Path -LiteralPath $archive)) {
        if ($offline) { Copy-Item -LiteralPath (Prepared-File $asset.name) -Destination $archive }
        else { Step 'boost-download' curl.exe @('--fail', '--location', '--connect-timeout', '15', '--max-time', '600', '--proto', '=https', '-o', $archive, $asset.browser_download_url) }
    }
    if ('sha256:' + (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $asset.digest) {
        throw 'Boost source archive digest mismatch'
    }
    $saved = Join-Path $inputsRoot $asset.name
    Copy-Item -LiteralPath $archive -Destination $saved
    Save-Input $saved
    $sourcePath = Join-Path $replayRoot Boost
    New-Item -ItemType Directory -Path $sourcePath -Force | Out-Null
    # This documentation-only symlink in the complete release uses a spelling
    # rejected by Windows tar. The full original archive is retained above.
    Step 'boost-source-unpack' tar @('-xf', $archive, '--strip-components=1', '--exclude=*/libs/decimal/doc/modules/ROOT/examples', '-C', $sourcePath)
    return $sourcePath
}

New-Item -ItemType Directory -Path $inputsRoot, $logsRoot -Force | Out-Null
$failed = $false
try {
    if (-not $IsWindows) { throw 'This preparation task currently supports Windows x64 only' }
    $environment = Join-Path $inputsRoot host-capabilities.json
    $video = @()
    $videoError = $null
    try {
        $video = @(Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion, VideoModeDescription, CurrentHorizontalResolution, CurrentVerticalResolution)
    } catch { $videoError = $_.Exception.Message }
    Write-NewJson $environment ([ordered]@{
        os = [Environment]::OSVersion.VersionString
        architecture = [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
        process_session = [Diagnostics.Process]::GetCurrentProcess().SessionId
        user_interactive = [Environment]::UserInteractive
        desktop_requested = [bool]$Desktop; video_controllers = $video; video_query_error = $videoError
        scope = 'Environment observations; these do not prove D3D/DXGI/WinRT capture, permissions, physical GPU or multiple-monitor acceptance.'
    })
    Save-Input $environment
    $mode = Join-Path $inputsRoot dependency-mode.json
    Write-NewJson $mode ([ordered]@{ offline = $offline; prepared_manifest = $OfflineManifest; current_source_snapshot = 'inputs/product-source' })
    Save-Input $mode
    if ($offline) {
        $pending.Add('Explicit offline dependency snapshot used; no new online latest-version check is asserted for this native attempt.')
        $env:CARGO_NET_OFFLINE = 'true'
        if (-not $CargoConfig) { throw 'Offline Rust tests require an explicit archived-vendor configuration' }
    } else {
        $preparation = Join-Path $inputsRoot toolchain-preparation
        try {
            Step 'windows-toolchain-current' pwsh @('-NoProfile','-File',(Join-Path $PSScriptRoot prepare-windows-toolchain.ps1),'-DepsRoot',$DepsRoot,'-RecordRoot',$preparation)
        } finally {
            if (Test-Path -LiteralPath $preparation) {
                Get-ChildItem -LiteralPath $preparation -Recurse -File | ForEach-Object { Save-Input $_.FullName }
            }
        }
    }
    $release = if ($offline) { Get-Content -LiteralPath (Prepared-File cmake-release.json) -Raw | ConvertFrom-Json } else { Api 'repos/Kitware/CMake/releases/latest' }
    $asset = @($release.assets | Where-Object name -Match 'windows-x86_64.zip$')
    if ($asset.Count -ne 1) { throw 'CMake release does not contain exactly one Windows x64 zip' }
    $asset = $asset[0]
    $cmakeMetadata = Join-Path $inputsRoot cmake-release.json
    Write-NewJson $cmakeMetadata $release
    Save-Input $cmakeMetadata
    $downloadRoot = Join-Path $DepsRoot downloads
    New-Item -ItemType Directory -Path $downloadRoot -Force | Out-Null
    $archive = Join-Path $downloadRoot $asset.name
    if (-not (Test-Path -LiteralPath $archive)) {
        if ($offline) { Copy-Item -LiteralPath (Prepared-File $asset.name) -Destination $archive }
        else { Step 'cmake-download' curl.exe @('--fail', '--location', '--connect-timeout', '15', '--max-time', '600', '--proto', '=https', '-o', $archive, $asset.browser_download_url) }
    }
    $actual = 'sha256:' + (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $asset.digest) { throw 'CMake download digest does not match the current official release' }
    $toolRoot = Join-Path $DepsRoot tools
    $cmake = Join-Path $toolRoot (($asset.name -replace '\.zip$', '') + '/bin/cmake.exe')
    if (-not (Test-Path -LiteralPath $cmake)) {
        New-Item -ItemType Directory -Path $toolRoot -Force | Out-Null
        Expand-Archive -LiteralPath $archive -DestinationPath $toolRoot
    }
    $savedCmake = Join-Path $inputsRoot $asset.name
    Copy-Item -LiteralPath $archive -Destination $savedCmake
    Save-Input $savedCmake
    Step 'cmake-identity' $cmake @('--version')
    $cmakeVersion = (& $cmake --version | Select-Object -First 1)
    if ($LASTEXITCODE -ne 0 -or $cmakeVersion -ne "cmake version $($release.tag_name.TrimStart('v'))") {
        throw 'Cached CMake executable does not match the latest resolved release'
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $vs = & $vswhere -version '[18.0,19.0)' -products '*' -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'VS2026 with MSVC x64 tools is required; no generator fallback is allowed' }
    $vs = @($vs)[0]
    $vsMetadata = Join-Path $inputsRoot vs2026.json
    Write-NewJson $vsMetadata $vs
    Save-Input $vsMetadata
    $toolset = (Get-Content (Join-Path $vs.installationPath 'VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt') -Raw).Trim()
    $sdkRoot = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots').KitsRoot10
    $sdk = Get-ChildItem (Join-Path $sdkRoot Lib) -Directory |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' -and (Test-Path (Join-Path $_.FullName 'um/x64/kernel32.lib')) } |
        Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
    if (-not $sdk) { throw 'An installed x64 Windows SDK is required' }
    # Servicing releases replace files within the same 10.0.<build>.0 tree.
    # The directory/kernel32 identity alone cannot invalidate an OpenCV cache
    # when only headers or other SDK components change.
    $sdkProducts = @(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*')
    [object[]]$sdkReleases = @(Select-SdkReleases $sdk.Name $sdkProducts)
    if ($framePoolSource) {
        $sdkHeaders = @()
        foreach ($entry in @(@('base.h','sdk-cppwinrt-base.h'),@('Windows.Foundation.h','sdk-cppwinrt-foundation.h'))) {
            $installedHeader = Join-Path $sdkRoot "Include/$($sdk.Name)/cppwinrt/winrt/$($entry[0])"
            $savedHeader = Join-Path $inputsRoot $entry[1]
            Copy-Item -LiteralPath $installedHeader -Destination $savedHeader
            if ((Get-FileHash -LiteralPath $installedHeader -Algorithm SHA256).Hash -ne (Get-FileHash -LiteralPath $savedHeader -Algorithm SHA256).Hash) {
                throw 'Selected SDK C++/WinRT header changed during snapshot'
            }
            Save-Input $savedHeader
            $sdkHeaders += [ordered]@{ installed_path=$installedHeader; source=(Artifact $savedHeader) }
        }
        $sdkSourceRecord = Join-Path $inputsRoot sdk-cppwinrt-source.json
        Write-NewJson $sdkSourceRecord ([ordered]@{ sdk_directory=$sdk.Name; installed_releases=$sdkReleases; headers=$sdkHeaders; license='MIT'; license_source='https://github.com/microsoft/cppwinrt/blob/master/LICENSE' })
        Save-Input $sdkSourceRecord
    }

    $opencvRelease = if ($offline) { Get-Content -LiteralPath (Prepared-File opencv-release.json) -Raw | ConvertFrom-Json } else { Api 'repos/opencv/opencv/releases/latest' }
    $opencvReleasePath = Join-Path $inputsRoot opencv-release.json
    Write-NewJson $opencvReleasePath $opencvRelease
    Save-Input $opencvReleasePath
    $opencv = Source 'opencv/opencv' $opencvRelease.tag_name OpenCV
    $maaPolicy = Join-Path $inputsRoot maautils-release-policy.json
    if ($offline) {
        $oldPolicy = Get-Content -LiteralPath (Prepared-File maautils-release-policy.json) -Raw | ConvertFrom-Json
        $maaRef = $oldPolicy.selected_ref
        Write-NewJson $maaPolicy $oldPolicy
    } else {
        $maaRepository = Api 'repos/MaaXYZ/MaaUtils'
        $maaReleases = @(Api 'repos/MaaXYZ/MaaUtils/releases')
        $maaRelease = $maaReleases | Where-Object { -not $_.prerelease -and -not $_.draft } | Select-Object -First 1
        $maaRef = if ($maaRelease) { $maaRelease.tag_name } else { $maaRepository.default_branch }
        Write-NewJson $maaPolicy ([ordered]@{ formal_release = [bool]$maaRelease; selected_ref = $maaRef; releases = $maaReleases })
    }
    Save-Input $maaPolicy
    $maautils = Source 'MaaXYZ/MaaUtils' $maaRef MaaUtils

    # OpenCV enables generic ASM before detecting its platform. A runner's GNU
    # assembler can load CMake's Windows-GNU platform and contaminate an MSVC
    # build with MINGW/pthread/.a requirements. Use the same installed compiler
    # as C/C++; this does not disable separate NASM or CPU-dispatched kernels.
    $compiler = Join-Path $vs.installationPath "VC/Tools/MSVC/$toolset/bin/Hostx64/x64/cl.exe"
    $options = @("-DCMAKE_ASM_COMPILER:FILEPATH=$compiler", '-DBUILD_LIST=core,imgproc,imgcodecs', '-DBUILD_SHARED_LIBS=OFF', '-DBUILD_WITH_STATIC_CRT=OFF',
        '-DBUILD_TESTS=OFF', '-DBUILD_PERF_TESTS=OFF', '-DBUILD_EXAMPLES=OFF', '-DBUILD_opencv_apps=OFF',
        '-DWITH_PROTOBUF=OFF', '-DWITH_FLATBUFFERS=OFF', '-DWITH_IPP=ON', '-DBUILD_IPP_IW=ON')
    $opencvDownloads = Join-Path $DepsRoot downloads/opencv
    if ($offline) {
        $opencvDownloads = Join-Path $replayRoot downloads
        New-Item -ItemType Directory -Path $opencvDownloads -Force | Out-Null
        Step 'opencv-download-replay' tar @('-xf', (Prepared-File opencv-downloads.tar), '-C', $opencvDownloads)
        $options += '-DOPENCV_DOWNLOAD_TRIES_LIST='
    }
    $configuration = [ordered]@{
        opencv_commit = $sourceIdentities['OpenCV']; opencv_source = $opencv; cmake_digest = $actual
        opencv_downloads = $opencvDownloads; offline = $offline
        vs_version = $vs.installationVersion; vs_instance = $vs.instanceId; toolset = $toolset
        compiler_sha256 = (Get-FileHash -LiteralPath (Join-Path $vs.installationPath "VC/Tools/MSVC/$toolset/bin/Hostx64/x64/cl.exe") -Algorithm SHA256).Hash.ToLowerInvariant()
        compiler_file_version = (Get-Item -LiteralPath $compiler).VersionInfo.FileVersion
        sdk = $sdk.Name; generator = 'Visual Studio 18 2026'; arch = 'x64'; options = $options
        sdk_root = $sdkRoot; sdk_installed_releases = $sdkReleases
        sdk_kernel32_sha256 = (Get-FileHash -LiteralPath (Join-Path $sdk.FullName 'um/x64/kernel32.lib') -Algorithm SHA256).Hash.ToLowerInvariant()
        cmake_binary_sha256 = (Get-FileHash -LiteralPath $cmake -Algorithm SHA256).Hash.ToLowerInvariant()
        cpu_policy = 'OpenCV upstream default CPU baseline, dispatch, SIMD and applicable HAL/third-party optimizations retained'
        module_policy = 'No DNN/OCR/inference module is selected. Disable unused Protobuf/Flatbuffers discovery; OpenCV 5 otherwise exports an unbuilt libprotobuf target in this static subset.'
        assembler_policy = 'Generic ASM uses the selected MSVC compiler to prevent GNU platform contamination. Separate NASM and optimized CPU kernels remain enabled.'
    }
    $configPath = Join-Path $inputsRoot native-configuration.json
    Write-NewJson $configPath $configuration
    Save-Input $configPath
    $key = (Get-FileHash -LiteralPath $configPath -Algorithm SHA256).Hash.ToLowerInvariant()
    # MSBuild's .NET tracking still encounters MAX_PATH even on long-path-aware
    # hosts. Shorten physical directories, checking the full cache identity.
    $nativeRoot = Join-Path $DepsRoot "native/$($key.Substring(0, 20))"
    New-Item -ItemType Directory -Path $nativeRoot -Force | Out-Null
    $cacheIdentity = Join-Path $nativeRoot configuration.json
    if (Test-Path -LiteralPath $cacheIdentity) {
        if ((Get-FileHash -LiteralPath $cacheIdentity -Algorithm SHA256).Hash.ToLowerInvariant() -ne $key) {
            throw 'Native cache key prefix collision or modified configuration'
        }
    } else { Copy-Item -LiteralPath $configPath -Destination $cacheIdentity }
    $opencvBuild = Join-Path $nativeRoot b
    $opencvInstall = Join-Path $nativeRoot i
    $generator = @('-G', 'Visual Studio 18 2026', '-A', 'x64', "-DCMAKE_GENERATOR_INSTANCE=$($vs.installationPath)", "-DCMAKE_SYSTEM_VERSION=$($sdk.Name)")
    $opencvConfigureArguments = @('-S', $opencv, '-B', $opencvBuild, "-DCMAKE_INSTALL_PREFIX=$opencvInstall", "-DOPENCV_DOWNLOAD_PATH=$opencvDownloads") + $generator + $options
    Step 'opencv-configure' $cmake $opencvConfigureArguments
    # An untyped -D compiler path can reset FILEPATH to UNINITIALIZED only on
    # cache reuse. Exercise the second configure before validating its identity.
    Step 'opencv-reconfigure' $cmake $opencvConfigureArguments
    $opencvCache = Join-Path $inputsRoot OpenCV-CMakeCache.txt
    Copy-Item -LiteralPath (Join-Path $opencvBuild CMakeCache.txt) -Destination $opencvCache
    Save-Input $opencvCache
    $cvconfig = Join-Path $inputsRoot opencv-cvconfig.h
    Copy-Item -LiteralPath (Join-Path $opencvBuild cvconfig.h) -Destination $cvconfig
    Save-Input $cvconfig
    $cvconfigText = Get-Content -LiteralPath $cvconfig -Raw
    $cacheText = Get-Content -LiteralPath $opencvCache -Raw
    $selectedAssembler = ($cacheText -split "`n" | Where-Object { $_ -match '^CMAKE_ASM_COMPILER:(STRING|FILEPATH)=' }) -replace '^CMAKE_ASM_COMPILER:(STRING|FILEPATH)=', ''
    if (-not $selectedAssembler -or [IO.Path]::GetFullPath($selectedAssembler.Trim()) -ne [IO.Path]::GetFullPath($compiler)) {
        throw 'OpenCV generic ASM compiler differs from the selected MSVC toolchain'
    }
    if ($cvconfigText -notmatch '(?m)^#define HAVE_IPP\s*$' -or
        $cacheText -notmatch '(?m)^CV_DISABLE_OPTIMIZATION:BOOL=OFF\s*$' -or
        $cacheText -notmatch '(?m)^CPU_DISPATCH:STRING=\S+') {
        throw 'The selected Windows OpenCV build did not retain required IPP/CPU optimizations'
    }
    Step 'opencv-build-install' $cmake @('--build', $opencvBuild, '--config', 'Release', '--target', 'INSTALL', '--parallel', '8')
    if (Test-Path -LiteralPath $opencvDownloads) {
        $downloadArchive = Join-Path $inputsRoot opencv-downloads.tar
        Step 'opencv-download-archive' tar @('-cf', $downloadArchive, '-C', $opencvDownloads, '.')
        Save-Input $downloadArchive
    }
    $artifacts = Join-Path $configRoot reference-u/artifacts
    New-Item -ItemType Directory -Path $artifacts -Force | Out-Null
    $adapterBuild = Join-Path $nativeRoot "a-$([DateTime]::UtcNow.Ticks)"
    Step 'adapter-configure' $cmake (@('-S', (Join-Path $productRoot tests/upstream-parity/native), '-B', $adapterBuild,
        "-DOpenCV_DIR=$opencvInstall", '-DMFS_BUILD_REFERENCE=OFF') + $generator)
    Step 'adapter-build' $cmake @('--build', $adapterBuild, '--config', 'Release', '--target', 'mfs_p0_adapter', '--parallel', '8')
    $adapterLibrary = Join-Path $artifacts mfs_p0_adapter.dll
    Copy-Item -LiteralPath (Join-Path $adapterBuild Release/mfs_p0_adapter.dll) -Destination $adapterLibrary
    Save-Input $adapterLibrary
    foreach ($name in @('CMakeCache.txt', 'toolchain-Release.txt')) {
        $saved = Join-Path $inputsRoot "adapter-$name"
        Copy-Item -LiteralPath (Join-Path $adapterBuild $name) -Destination $saved
        Save-Input $saved
    }
    $env:MFS_P0_NATIVE_LIBRARY = $adapterLibrary
    . (Join-Path $PSScriptRoot configure-prototype.ps1) -ConfigureTool $ConfigureTool -RustBatch $RustBatch -ReferenceRoot $ReferenceRoot -Family windows
    Save-Input $env:MFS_BUILD_CONFIGURATION
    $rustArguments = @('test', '--release', '--locked', '--test', 'p0-native', '--features', 'prototype-windows-com')
    if ($offline) { $rustArguments += @('--offline', '--config', $CargoConfig) }
    Step 'rust-native-ffi' cargo ($rustArguments + @('--', '--ignored', '--test-threads=1', '--nocapture'))

    $boost = Boost-Source
    $build = Join-Path $nativeRoot "r-$([DateTime]::UtcNow.Ticks)"
    [string[]]$parityOptions = @(if ($GdiParity) { '-DMFS_BUILD_GDI_PARITY=ON' })
    if ($PrintWindowParity) { $parityOptions += '-DMFS_BUILD_PRINT_PARITY=ON' }
    if ($ScreenDcParity) { $parityOptions += '-DMFS_BUILD_SCREEN_PARITY=ON' }
    if ($framePoolSource) { $parityOptions += '-DMFS_BUILD_FRAMEPOOL_REFERENCE=ON' }
    if ($FramePoolParity) { $parityOptions += '-DMFS_BUILD_FRAMEPOOL_PARITY=ON' }
    Step 'reference-configure' $cmake (@('-S', (Join-Path $productRoot tests/upstream-parity/native), '-B', $build,
        "-DOpenCV_DIR=$opencvInstall", "-DMFS_UPSTREAM_ROOT=$UpstreamRoot", "-DMFS_MAAUTILS_ROOT=$maautils", "-DMFS_BOOST_ROOT=$boost") + $generator + $parityOptions)
    $referenceCache = Get-Content -LiteralPath (Join-Path $build CMakeCache.txt) -Raw
    foreach ($selection in @(@('MFS_BUILD_GDI_PARITY',[bool]$GdiParity),@('MFS_BUILD_PRINT_PARITY',[bool]$PrintWindowParity),@('MFS_BUILD_SCREEN_PARITY',[bool]$ScreenDcParity),@('MFS_BUILD_FRAMEPOOL_REFERENCE',[bool]$framePoolSource),@('MFS_BUILD_FRAMEPOOL_PARITY',[bool]$FramePoolParity))) {
        $expectedValue = if ($selection[1]) { 'ON' } else { 'OFF' }
        $pattern = '(?m)^' + [regex]::Escape($selection[0]) + ':BOOL=' + $expectedValue + '\r?$'
        if (-not [regex]::IsMatch($referenceCache,$pattern)) { throw "CMake did not select $($selection[0])=$expectedValue" }
    }
    Step 'reference-build' $cmake @('--build', $build, '--config', 'Release', '--target', 'mfs_reference_u', '--parallel', '8')
    if ($GdiParity) {
        Step 'gdi-parity-build' $cmake @('--build', $build, '--config', 'Release', '--target', 'mfs_gdi_parity', '--parallel', '8')
    }
    if ($framePoolSource) {
        Step 'frame-pool-context-build' $cmake @('--build', $build, '--config', 'Release', '--target', 'mfs_framepool_reference', '--parallel', '8')
    }
    foreach ($name in @('CMakeCache.txt', 'toolchain-Release.txt')) {
        $saved = Join-Path $inputsRoot "reference-$name"
        Copy-Item -LiteralPath (Join-Path $build $name) -Destination $saved
        Save-Input $saved
    }
    foreach ($name in @('mfs_reference_utils.dll', 'mfs_reference_u.exe')) {
        $saved = Join-Path $artifacts $name
        Copy-Item -LiteralPath (Join-Path $build "Release/$name") -Destination $saved
        Save-Input $saved
    }
    if ($framePoolSource) {
        $programPath = Join-Path $artifacts mfs_framepool_reference.exe
        Copy-Item -LiteralPath (Join-Path $build Release/mfs_framepool_reference.exe) -Destination $programPath
        Save-Input $programPath
        $frameContextEvidence = Join-Path $artifacts frame-pool-context
        New-Item -ItemType Directory -Path $frameContextEvidence | Out-Null
        $contextFailures = [Collections.Generic.List[string]]::new()
        foreach ($modeName in @('uninitialized','mta','mta-dispatcher','sta')) {
            $modeRoot = Join-Path $frameContextEvidence $modeName
            New-Item -ItemType Directory -Path $modeRoot | Out-Null
            try { Step "frame-pool-source-$modeName" $programPath @($modeName, $modeRoot) }
            catch { $contextFailures.Add($_.Exception.Message) }
        }
        if ($contextFailures.Count -gt 0) { throw ($contextFailures -join "`n") }
        if ($FramePoolParity) {
            $pseudoEvidence = Join-Path $frameContextEvidence 'pseudo'
            $pseudoSourceFailures = 0
            foreach ($scenario in @('pseudo-visible','pseudo-stop','pseudo-inactive','pseudo-layered-stop','pseudo-layered-inactive')) {
                $modeEvidence = Join-Path $pseudoEvidence $scenario
                New-Item -ItemType Directory -Path $modeEvidence -Force | Out-Null
                try { Step "frame-pool-source-$scenario" $programPath @('mta',$modeEvidence,$scenario) }
                catch {
                    # Keep this exact source/color failure in the final result.
                    # Continue independent regressions and byte/state diagnostics;
                    # they cannot turn this source pixel failure into acceptance.
                    $failed = $true
                    $pseudoSourceFailures++
                    Write-Host $_.Exception.Message
                }
            }
            if ($pseudoSourceFailures -gt 0) {
                Write-Host 'Original wrapper known-pixel failures retained; later matching bytes/states are diagnostic only.'
            }
        }
    }
    if ($GdiParity) {
        $parityLibrary = Join-Path $artifacts mfs_gdi_parity.dll
        Copy-Item -LiteralPath (Join-Path $build Release/mfs_gdi_parity.dll) -Destination $parityLibrary
        Save-Input $parityLibrary
        $env:MFS_GDI_PARITY_LIBRARY = $parityLibrary
        $capabilities = @('windows-gdi', 'prototype-windows-com')
        if ($PrintWindowParity) { $capabilities += 'windows-print-window' }
        if ($ScreenDcParity) { $capabilities += 'windows-screen-dc' }
        if ($FramePoolParity) { $capabilities += 'windows-frame-pool' }
        $parityFeatures = $capabilities -join ','
        if ($ScreenDcParity) {
            $screenEvidence = Join-Path $artifacts screen-evidence
            New-Item -ItemType Directory -Path $screenEvidence | Out-Null
            $env:MFS_SCREEN_EVIDENCE = $screenEvidence
            $geometryEvidence = Join-Path $artifacts corrected-fork-geometry
            New-Item -ItemType Directory -Path $geometryEvidence | Out-Null
            $env:MFS_F_GEOMETRY_ARTIFACTS = $geometryEvidence
        }
        $parityArguments = @('test', '--release', '--locked', '--lib', '--features', $parityFeatures)
        $clippyArguments = @('clippy', '--workspace', '--all-targets', '--locked', '--features', $parityFeatures)
        if ($offline) {
            $parityArguments += @('--offline', '--config', $CargoConfig)
            $clippyArguments += @('--offline', '--config', $CargoConfig)
        }
        # Shared GDI readback tests moved to their owning module; select all
        # compiled Windows backend modules so GDI-only runs retain those checks.
        $parityFilter = 'backends::windows::'
        if ($ScreenDcParity) {
            Step 'screen-window-helper-original-parity' cargo ($parityArguments + @('--', 'platform::windows::window_api::', '--include-ignored', '--test-threads=1', '--nocapture'))
        }
        if ($PrintWindowParity -or $FramePoolParity) {
            Step 'pseudo-helper-original-parity' cargo ($parityArguments + @('--', 'platform::windows::pseudo_minimize::', '--include-ignored', '--test-threads=1', '--nocapture'))
        }
        $foregroundCase = 'backends::windows::print_window::tests::parity::pseudo_minimize::restores_original_attributes_after_real_foreground_reactivation'
        [string[]]$caseSelection = @($parityFilter, '--include-ignored', '--test-threads=1', '--nocapture')
        $frameCase = 'backends::windows::frame_pool::tests::captures_original_framepool_pixels_and_retains_result_and_cache_clone_owners'
        $frameLifecycleCase = 'backends::windows::frame_pool::tests::retains_pixels_across_twenty_complete_framepool_lifecycles'
        $framePseudoCase = 'backends::windows::frame_pool_pseudo::tests::preserves_original_pseudo_wrapper_pixels_and_stop_inactive_states'
        if ($FramePoolParity) { $caseSelection += @('--skip', $frameCase, '--skip', $frameLifecycleCase, '--skip', $framePseudoCase) }
        if ($PrintWindowParity) { $caseSelection += @('--skip', $foregroundCase) }
        Step 'gdi-original-parity' cargo ($parityArguments + @('--') + $caseSelection)
        Step 'gdi-migration-clippy' cargo ($clippyArguments + @('--', '-D', 'warnings'))
        if ($FramePoolParity) {
            Step 'frame-pool-factory-native' cargo ($parityArguments + @('--', 'platform::windows::wgc::factory::tests::', '--include-ignored', '--test-threads=1', '--nocapture'))
            foreach ($modeName in @('uninitialized','mta','mta-dispatcher','sta')) {
                $env:MFS_FRAMEPOOL_HOST_MODE = $modeName
                $env:MFS_FRAMEPOOL_REFERENCE_PIXELS = Join-Path $frameContextEvidence "$modeName/source.bgr"
                Step "frame-pool-rust-$modeName" cargo ($parityArguments + @('--', $frameCase, '--exact', '--include-ignored', '--test-threads=1', '--nocapture'))
            }
            $env:MFS_FRAMEPOOL_REFERENCE_PIXELS = Join-Path $frameContextEvidence 'mta-dispatcher/source.bgr'
            Step 'frame-pool-rust-lifecycles' cargo ($parityArguments + @('--', $frameLifecycleCase, '--exact', '--include-ignored', '--test-threads=1', '--nocapture'))
            $env:MFS_FRAMEPOOL_PSEUDO_REFERENCE_ROOT = $pseudoEvidence
            Step 'frame-pool-rust-pseudo' cargo ($parityArguments + @('--', $framePseudoCase, '--exact', '--include-ignored', '--test-threads=1', '--nocapture'))
        }
        if ($PrintWindowParity) {
            # Separate invocation preserves the required case and its native
            # environment result without obscuring other pixel/ownership checks.
            Step 'pseudo-foreground-parity' cargo ($parityArguments + @('--', $foregroundCase, '--exact', '--include-ignored', '--test-threads=1', '--nocapture'))
        }
    }
    $licenses = Join-Path $artifacts licenses
    New-Item -ItemType Directory -Path $licenses -Force | Out-Null
    foreach ($item in @(
        @((Join-Path $productRoot LICENSE), 'maafw-screencap-LICENSE.txt'),
        @((Join-Path $UpstreamRoot LICENSE.md), 'MaaFramework-LICENSE.md'),
        @((Join-Path $maautils LICENSE.md), 'MaaUtils-LICENSE.md'),
        @((Join-Path $opencv LICENSE), 'OpenCV-LICENSE.txt'),
        @((Join-Path $opencv COPYRIGHT), 'OpenCV-COPYRIGHT.txt'),
        @((Join-Path $boost LICENSE_1_0.txt), 'Boost-LICENSE.txt')
    )) {
        Copy-Item -LiteralPath $item[0] -Destination (Join-Path $licenses $item[1])
    }
    Copy-Item -LiteralPath (Join-Path $opencvInstall etc/licenses) -Destination (Join-Path $licenses OpenCV) -Recurse
    if ($framePoolSource) {
        Copy-Item -LiteralPath (Join-Path $productRoot licenses/cppwinrt-MIT.txt) -Destination (Join-Path $licenses cppwinrt-MIT.txt)
    }
    Get-ChildItem -LiteralPath $licenses -Recurse -File | ForEach-Object { Save-Input $_.FullName }
    if ($Desktop) {
        $pixels = Join-Path $artifacts controlled-window.bgr
        Step 'reference-gdi-capture' (Join-Path $artifacts mfs_reference_u.exe) @($pixels)
        Save-Input $pixels
    } else { $pending.Add('The native desktop reference executable was built but capture was not requested.') }
}
catch {
    $failed = $true
    $message = $_.Exception.ToString()
    Write-Host $message
    $failureLog = Join-Path $logsRoot preparation-error.log
    $message | Set-Content -LiteralPath $failureLog -Encoding utf8
    $checks.Add([ordered]@{ name = 'native-preparation'; status = 'failed'; log = (Artifact $failureLog) })
}
finally {
    if ($screenEvidence -and (Test-Path -LiteralPath $screenEvidence)) {
        Get-ChildItem -LiteralPath $screenEvidence -File | ForEach-Object { Save-Input $_.FullName }
    }
    if ($frameContextEvidence -and (Test-Path -LiteralPath $frameContextEvidence)) {
        Get-ChildItem -LiteralPath $frameContextEvidence -Recurse -File | ForEach-Object { Save-Input $_.FullName }
    }
    Write-NewJson (Join-Path $RunRoot native-result.json) ([ordered]@{
        layout_version = 1; run_id = (Split-Path -Leaf $RunRoot); phase = $(if ($GdiParity -or $FramePoolContext) { 'P1' } else { 'P0' }); phase_status = 'incomplete'
        checks = @($checks.ToArray()); pending = @($pending.ToArray()); inputs = @($inputs.ToArray())
    })
}
if ($failed -or $nativeMissingEnvironment) { exit 1 }
