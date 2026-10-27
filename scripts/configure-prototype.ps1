# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
param(
    [Parameter(Mandatory)][string]$ConfigureTool,
    [Parameter(Mandatory)][string]$RustBatch,
    [Parameter(Mandatory)][string]$ReferenceRoot,
    [Parameter(Mandatory)][ValidateSet('windows','pipewire','wlr','screen-capture-kit')][string]$Family
)
$configurationArguments = @('configure','--offline-inputs',$RustBatch,'--root',$ReferenceRoot)
$configurationArguments += @('--consumer-lock',(Join-Path (Split-Path -Parent $PSScriptRoot) Cargo.lock))
foreach ($configurationFamily in @('windows','pipewire','wlr','screen-capture-kit')) {
    $configurationArguments += @("--$configurationFamily",$(if ($configurationFamily -eq $Family) {'ON'} else {'OFF'}))
}
$configurationOutput = & $ConfigureTool @configurationArguments
if ($LASTEXITCODE -ne 0) { throw 'Explicit prototype native configuration failed' }
$configurationPaths = @($configurationOutput | Where-Object { $_.StartsWith('MFS_BUILD_CONFIGURATION=') })
if ($configurationPaths.Count -ne 1) { throw 'Configuration did not return one build input' }
$env:MFS_BUILD_CONFIGURATION = $configurationPaths[0].Substring('MFS_BUILD_CONFIGURATION='.Length)
if (-not (Test-Path -LiteralPath $env:MFS_BUILD_CONFIGURATION -PathType Leaf)) { throw 'Prepared build input is missing' }
$configurationOutput | ForEach-Object { Write-Host $_ }
