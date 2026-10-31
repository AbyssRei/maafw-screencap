# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Exercises the actual orchestration function with child-command witnesses.
# No native capture or desktop acceptance is performed here.
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$source=Join-Path $PSScriptRoot ../../scripts/prepare-native.ps1
$tokens=$null;$errors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile($source,[ref]$tokens,[ref]$errors)
if($errors.Count){throw 'Native orchestration does not parse'}
$functions=@($ast.FindAll({param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Invoke-FrameCheck'},$true))
if($functions.Count -ne 1){throw 'Expected one actual independent FramePool function'}
. ([scriptblock]::Create($functions[0].Extent.Text))
$script:events=[Collections.Generic.List[string]]::new()
function Step([string]$Name,[string]$Program,[string[]]$Arguments){
    if($Program -ne 'owned-child' -or $Arguments.Count -ne 2 -or $Arguments[0] -ne '--unchanged' -or $Arguments[1] -ne 'fixture-path'){throw 'Child command or arguments changed'}
    $script:events.Add($Name)
    if($Name -eq 'original-failure'){throw 'controlled child failure retained'}
}
$script:failed=$false
Invoke-FrameCheck success owned-child @('--unchanged','fixture-path')
if($script:failed -or $script:events.Count -ne 1){throw 'Successful independent case changed batch failure'}
Invoke-FrameCheck original-failure owned-child @('--unchanged','fixture-path')
if(-not $script:failed -or $script:events.Count -ne 2){throw 'Failed child was not retained'}
Invoke-FrameCheck later-matrix owned-child @('--unchanged','fixture-path')
if(-not $script:failed -or $script:events.Count -ne 3 -or $script:events[2] -ne 'later-matrix'){throw 'Independent continuation cleared failure or did not execute'}
Write-Host 'FramePool independent cases=3 original_failure_retained=1 later_matrix_executed=1 arguments_unchanged=1 native_capture=0 passed'
