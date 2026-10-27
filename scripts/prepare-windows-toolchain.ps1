# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [string]$DepsRoot = (Join-Path (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)) maafw-screencap-deps),
    [string]$RecordRoot = (Join-Path (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)) maafw-screencap-reference/local),
    [switch]$Install
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false

function Select-WindowsSdkRelease([string]$Html) {
    $stable = [regex]::Split($Html, '(?i)<h2[^>]+id="preview-releases"')[0]
    $releases = @([regex]::Matches($stable, '(?is)<tr>.*?</tr>') | ForEach-Object {
        $row = $_.Value
        if ($row -match 'Windows SDK for Windows (?:11|10)[^<()]*\((\d+\.\d+\.\d+\.\d+)\)') {
            $version = $Matches[1]
            if ($row -match '<a href="(https://go\.microsoft\.com/fwlink/\?linkid=\d+)"[^>]*>Installer</a>') {
                [pscustomobject]@{version=$version;installer=$Matches[1]}
            }
        }
    } | Sort-Object { [version]$_.version } -Descending)
    if (-not $releases.Count) { throw 'Microsoft SDK download page has no recognized formal installer' }
    return $releases[0]
}

function Download([string]$Url, [string]$Path) {
    & curl.exe --fail --location --silent --show-error --compressed --proto '=https' --connect-timeout 15 --max-time 300 -o $Path $Url
    if ($LASTEXITCODE -ne 0) { throw "Download failed: $Url" }
}
function Signed-Installer([string]$Url, [string]$Name) {
    $path = Join-Path $recordDirectory $Name
    Write-Host "Downloading and checking Microsoft signature: $Name ..."
    Download $Url $path
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|, )O=Microsoft Corporation(,|$)') {
        throw "Installer lacks a valid Microsoft Authenticode signature: $path"
    }
    $record.installers.Add([ordered]@{
        name=$Name;url=$Url;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        file_version=(Get-Item -LiteralPath $path).VersionInfo.FileVersion
        signature_status=$signature.Status.ToString();signer=$signature.SignerCertificate.Subject
    })
    return $path
}
function Installed-Vs {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) { return $null }
    $output = & $vswhere -version '[18.0,19.0)' -products '*' -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json
    if ($LASTEXITCODE -ne 0) { throw 'VS2026 discovery failed' }
    $instances = @($output | ConvertFrom-Json)
    if ($instances.Count) { return $instances[0] }
    return $null
}
function Installed-Sdks {
    return @(Get-ItemProperty 'HKLM:/SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall/*','HKLM:/SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall/*' |
        Where-Object { $_.PSObject.Properties['DisplayName'] -and $_.DisplayName -match '^Windows Software Development Kit' } |
        Select-Object DisplayName,DisplayVersion)
}
function Run-Installer([string]$Name, [string]$Program, [string[]]$Arguments) {
    Write-Host "Installing $Name ..."
    $info = [Diagnostics.ProcessStartInfo]::new($Program)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw "Cannot start $Name" }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $stdout.GetAwaiter().GetResult() | Set-Content -LiteralPath (Join-Path $recordDirectory "$Name-stdout.log") -Encoding utf8NoBOM
        $stderr.GetAwaiter().GetResult() | Set-Content -LiteralPath (Join-Path $recordDirectory "$Name-stderr.log") -Encoding utf8NoBOM
        $record.operations.Add([ordered]@{name=$Name;program=$Program;arguments=$Arguments;exit=$process.ExitCode})
        if ($process.ExitCode -eq 3010) { throw "$Name requires an OS restart; no restart is performed and validation must wait" }
        if ($process.ExitCode -ne 0) { throw "$Name failed with exit $($process.ExitCode)" }
    } finally { $process.Dispose() }
}

if (-not $IsWindows -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64') { throw 'Native Windows x64 is required' }
$recordDirectory = Join-Path ([IO.Path]::GetFullPath($RecordRoot)) ('windows-toolchain-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffffffZ'))
New-Item -ItemType Directory -Path $recordDirectory | Out-Null
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $recordDirectory preparation-source.ps1)
$record = [ordered]@{status='incomplete';script_sha256=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();install_requested=[bool]$Install;installers=[Collections.Generic.List[object]]::new();operations=[Collections.Generic.List[object]]::new()}
$exitCode = 1
try {
    $channelPath = Join-Path $recordDirectory vs18-channel.json
    Download 'https://aka.ms/vs/18/stable/channel' $channelPath
    $channel = Get-Content -LiteralPath $channelPath -Raw | ConvertFrom-Json
    $products = @($channel.channelItems | Where-Object { $_.id -eq 'Microsoft.VisualStudio.Product.BuildTools' -and $_.type -eq 'ChannelProduct' })
    if ($products.Count -ne 1 -or $products[0].version -notmatch '^18\.\d+\.\d+\.\d+$') { throw 'VS2026 stable channel has no unique Build Tools identity' }
    $vsVersion = $products[0].version
    $sdkPage = Join-Path $recordDirectory windows-sdk-downloads.html
    Download 'https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads' $sdkPage
    $sdk = Select-WindowsSdkRelease (Get-Content -LiteralPath $sdkPage -Raw)
    $record.expected = [ordered]@{vs_version=$vsVersion;sdk=$sdk}
    $record.before = [ordered]@{vs=(Installed-Vs);sdks=(Installed-Sdks)}
    $vsReady = $record.before.vs -and $record.before.vs.installationVersion -eq $vsVersion
    $sdkReady = @($record.before.sdks | Where-Object { $_.DisplayName.EndsWith('Windows '+$sdk.version) }).Count -gt 0
    $vsInstaller = if (-not $vsReady) { Signed-Installer 'https://aka.ms/vs/stable/vs_buildtools.exe' vs_BuildTools.exe } else { $null }
    $sdkInstaller = if (-not $sdkReady) { Signed-Installer $sdk.installer winsdksetup.exe } else { $null }
    if ($vsInstaller -and (Get-Item -LiteralPath $vsInstaller).VersionInfo.FileVersion -ne $vsVersion) { throw 'Build Tools bootstrapper differs from the selected VS2026 stable channel' }
    if ($sdkInstaller) {
        $expected = [version]$sdk.version
        $actual = [version](Get-Item -LiteralPath $sdkInstaller).VersionInfo.FileVersion
        if ($actual.Major -ne $expected.Major -or $actual.Minor -notin @(0,1) -or $actual.Build -ne $expected.Build -or $actual.Revision -ne $expected.Revision) { throw 'SDK bootstrapper differs from the selected formal SDK build/revision' }
    }
    $installPath = Join-Path ([IO.Path]::GetFullPath($DepsRoot)) 'tools/vs2026-buildtools'
    $record.vs_install_path = $installPath
    if ($Install -and (-not $vsReady -or -not $sdkReady)) {
        $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
        if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Installation requires an elevated PowerShell; read-only preparation is preserved and no UAC is requested' }
        if ($vsInstaller) {
            # Only modify our dedicated Build Tools instance. Other IDEs are
            # left to their owner, even when that IDE is older than the channel.
            $verb = if (Test-Path -LiteralPath $installPath) { 'update' } else { $null }
            [string[]]$arguments = @($verb | Where-Object { $_ })+@('--quiet','--wait','--norestart','--installPath',$installPath)
            if (-not $verb) { $arguments += @('--add','Microsoft.VisualStudio.Workload.VCTools','--add','Microsoft.VisualStudio.Component.VC.Tools.x86.x64') }
            Run-Installer vs-buildtools $vsInstaller $arguments
        }
        if ($sdkInstaller) { Run-Installer windows-sdk $sdkInstaller @('/quiet','/norestart','/ceip','off','/features','OptionId.DesktopCPPx64','OptionId.DesktopCPPx86','OptionId.UWPCPP','/log',(Join-Path $recordDirectory windows-sdk-install.log)) }
    }
    $record.after = [ordered]@{vs=(Installed-Vs);sdks=(Installed-Sdks)}
    $vsReady = $record.after.vs -and $record.after.vs.installationVersion -eq $vsVersion
    $sdkReady = @($record.after.sdks | Where-Object { $_.DisplayName.EndsWith('Windows '+$sdk.version) }).Count -gt 0
    $record.status = if ($vsReady -and $sdkReady) { 'prepared_environment' } else { 'missing_environment' }
    $exitCode = if ($vsReady -and $sdkReady) { 0 } else { 77 }
    Write-Host "VS2026 expected=$vsVersion ready=$vsReady; SDK expected=$($sdk.version) ready=$sdkReady"
} catch {
    $record.error = $_.Exception.ToString()
    Write-Host $record.error
} finally {
    $record.scope = 'System dependency preparation only; installer/registry success does not prove compiler, SDK, linking, runtime or capture acceptance. SDK installation is machine-wide. Build Tools use a dedicated deps path.'
    $record | ConvertTo-Json -Depth 25 | Set-Content -LiteralPath (Join-Path $recordDirectory result.json) -Encoding utf8NoBOM
    Write-Host "Windows toolchain preparation record: $recordDirectory"
}
exit $exitCode
