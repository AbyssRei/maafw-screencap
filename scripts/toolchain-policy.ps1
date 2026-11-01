# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later

# The workflow opts in explicitly. Native capabilities and real build/runtime
# results remain required; the age of an installed tool is not a CI failure.
function Get-ToolchainPolicy {
    $policy = $env:MFS_TOOLCHAIN_POLICY
    if (-not $policy) { return 'latest' }
    if ($policy -notin @('latest', 'preinstalled')) { throw "Unknown MFS_TOOLCHAIN_POLICY: $policy" }
    return $policy
}

function Get-WindowsToolchainReadiness([string]$Policy, $Vs, [object[]]$SdkProducts, [object[]]$SdkDirectories, $Expected) {
    if ($Policy -eq 'preinstalled') { return [pscustomobject]@{vs=[bool]$Vs;sdk=(@($SdkDirectories).Count -gt 0)} }
    if ($Policy -ne 'latest' -or -not $Expected) { throw 'Latest Windows toolchain selection requires a resolved release identity' }
    return [pscustomobject]@{
        vs=($Vs -and $Vs.installationVersion -eq $Expected.vs_version)
        sdk=(@($SdkProducts | Where-Object { $_.DisplayName.EndsWith('Windows '+$Expected.sdk.version) }).Count -gt 0)
    }
}

function Invoke-ToolchainAdvisory([string]$Program, [string[]]$Arguments) {
    try {
        $output = (& $Program @Arguments 2>&1 | Out-String).Trim()
        $code = $LASTEXITCODE
        return [pscustomobject]@{exit=$code;available=($code -eq 0);output=$output}
    } catch {
        return [pscustomobject]@{exit=-1;available=$false;output=$_.Exception.Message}
    }
}

function Test-CMakeGenerator($Capabilities, [string]$Generator) {
    if (-not $Capabilities -or -not $Capabilities.PSObject.Properties['version'] -or
        -not $Capabilities.version.PSObject.Properties['string'] -or -not $Capabilities.PSObject.Properties['generators']) { return $false }
    return $Capabilities.version.string -and @($Capabilities.generators | Where-Object name -EQ $Generator).Count -gt 0
}

function Find-InstalledCMake([string]$Generator) {
    $program = Get-Command cmake -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $program) { return $null }
    $probe = Invoke-ToolchainAdvisory $program.Source @('-E', 'capabilities')
    if (-not $probe.available) { return $null }
    try { $capabilities = $probe.output | ConvertFrom-Json } catch { return $null }
    if (-not (Test-CMakeGenerator $capabilities $Generator)) { return $null }
    return [pscustomobject]@{policy='preinstalled';program=$program.Source;version=$capabilities.version.string;sha256=(Get-FileHash -LiteralPath $program.Source).Hash.ToLowerInvariant();capabilities=$capabilities}
}
