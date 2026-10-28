# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Load just the pure selector, without executing native preparation/downloads.
$source = Join-Path $PSScriptRoot ../../scripts/prepare-native.ps1
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($source,[ref]$tokens,[ref]$errors)
if ($errors.Count) { throw ($errors | Out-String) }
$functions = @($ast.FindAll({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Select-SdkReleases'},$false))
if ($functions.Count -ne 1) { throw 'Expected one SDK release selector' }
. ([scriptblock]::Create($functions[0].Extent.Text))

function Product([string]$Version) {
    [pscustomobject]@{DisplayName="Windows Software Development Kit - Windows $Version";DisplayVersion=($Version -replace '^10\.0\.','10.1.')}
}
function Identity([object[]]$Products) {
    # Hold the directory and library bytes constant to reproduce a headers-only
    # servicing update; only installed SDK release evidence may change the key.
    [object[]]$releases = @(Select-SdkReleases '10.0.28000.0' $Products)
    [ordered]@{sdk='10.0.28000.0';sdk_kernel32_sha256=('0'*64);sdk_installed_releases=$releases} | ConvertTo-Json -Depth 5 -Compress
}

$old = Product '10.0.28000.2705'
$updated = Product '10.0.28000.2957'
$unrelated = Product '10.0.26100.9457'
$oldIdentity = Identity @($old)
if ($oldIdentity -eq (Identity @($updated))) { throw 'SDK servicing update failed to invalidate the cache identity' }
if ($oldIdentity -ne (Identity @($unrelated,$old,$old))) { throw 'Unrelated/duplicate SDK products changed the selected SDK identity' }
$both = Identity @($old,$updated)
if ($both -ne (Identity @($updated,$old))) { throw 'Registry enumeration order changed SDK identity' }
if ((@(Select-SdkReleases '10.0.28000.0' @($unrelated,[pscustomobject]@{}))).Count -ne 0) { throw 'Unrelated/incomplete product was accepted' }
Write-Output 'SDK cache identity: servicing invalidation, family isolation, duplicates and ordering passed'
