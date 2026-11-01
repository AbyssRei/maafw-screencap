# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
$ErrorActionPreference='Stop'
$PSNativeCommandUseErrorActionPreference=$false
. (Join-Path $PSScriptRoot toolchain-policy.ps1)
$policy=Get-ToolchainPolicy
$installed=Invoke-ToolchainAdvisory rustup @('run','nightly','rustc','-vV')
Write-Output "Rust toolchain policy=$policy; installed_nightly=$($installed.available)"
if($policy -eq 'latest' -or -not $installed.available) {
    & rustup update nightly --no-self-update
    if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
}
$components=(& rustup component list --toolchain nightly --installed | Out-String)
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
if($components -notmatch '(?m)^rustfmt-' -or $components -notmatch '(?m)^clippy-') {
    & rustup component add --toolchain nightly rustfmt clippy
    if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
}
& rustup run nightly rustc -vV
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
exit 0
