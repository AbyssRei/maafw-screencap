# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later

. (Join-Path $PSScriptRoot toolchain-policy.ps1)

# Loading this file only defines functions. Local latest selection uses Apple's
# formal version/build. CI may use the runner's installed, identified Xcode.
function Select-AppleXcodeRelease([xml]$Feed) {
    $releases = @($Feed.rss.channel.item | ForEach-Object {
        if ($_.title -cmatch '^Xcode (\d+(?:\.\d+){0,2}) \(([A-Za-z0-9]+)\)$') {
            [pscustomobject]@{version=$Matches[1];build=$Matches[2];url=[string]$_.link;title=[string]$_.title}
        }
    } | Sort-Object { [version]($(if ($_.version.Contains('.')) { $_.version } else { $_.version+'.0' })) } -Descending)
    if (-not $releases.Count) { throw 'Apple feed contains no recognized formal Xcode release' }
    $release = $releases[0]
    if ($release.url -notlike 'https://developer.apple.com/news/releases/*') { throw 'Xcode release is outside the Apple release origin' }
    return $release
}

function Select-InstalledAppleToolchain([object[]]$Installed, $Release, [string]$Policy, [string]$Preferred) {
    $usable = @($Installed | Where-Object { $_.exit -eq 0 -and $_.version -and $_.build })
    if ($Policy -eq 'latest') {
        if (-not $Release) { throw 'Local latest Xcode selection requires the formal release identity' }
        $version = [version]($(if ($Release.version.Contains('.')) { $Release.version } else { $Release.version+'.0' }))
        return $usable | Where-Object { [version]$_.version -eq $version -and $_.build -ceq $Release.build } | Select-Object -First 1
    }
    if ($Policy -ne 'preinstalled') { throw "Unknown Xcode toolchain policy: $Policy" }
    # Preserve a usable runner selection, including its exact build identity.
    # A release-feed mismatch is recorded, never relabelled as a formal build.
    $preferredChoice = $usable | Where-Object developer_dir -EQ $Preferred | Select-Object -First 1
    if ($preferredChoice) { return $preferredChoice }
    return $usable | Sort-Object { [version]$_.version } -Descending | Select-Object -First 1
}

function Initialize-AppleToolchain([string]$InputRoot) {
    if (-not $IsMacOS) { throw 'Xcode selection requires native macOS' }
    $policy = Get-ToolchainPolicy
    $feedPath = Join-Path $InputRoot apple-releases.rss
    $release = $null
    $lookup = $null
    if ($policy -eq 'preinstalled') {
        $lookup = Invoke-ToolchainAdvisory curl @('--fail','--location','--silent','--show-error','--connect-timeout','5','--max-time','15','https://developer.apple.com/news/releases/rss/releases.rss')
        $lookup.output | Set-Content -LiteralPath $feedPath -Encoding utf8
        if ($lookup.available) {
            try { $release = Select-AppleXcodeRelease ([xml]$lookup.output) }
            catch { $lookup = [pscustomobject]@{exit=$lookup.exit;available=$false;output=$_.Exception.Message} }
        }
    } else {
        Step apple-releases curl @('--fail','--location','--silent','--show-error','--connect-timeout','15','--max-time','60','-o',$feedPath,'https://developer.apple.com/news/releases/rss/releases.rss')
        $release = Select-AppleXcodeRelease ([xml](Get-Content -LiteralPath $feedPath -Raw))
    }
    $inputs.Add((Artifact $feedPath))
    $observations = [Collections.Generic.List[object]]::new()
    $originalDeveloperDir = $env:DEVELOPER_DIR
    $preferred = $originalDeveloperDir
    if ($policy -eq 'preinstalled' -and -not $preferred) {
        $preferred = (& /usr/bin/xcode-select --print-path 2>&1 | Out-String).Trim()
        if ($LASTEXITCODE -ne 0) { $preferred = '' }
    }
    # Probe each installed bundle using a process-local override. Do not invoke
    # sudo/xcode-select or modify the user's machine-wide default.
    try {
        foreach ($bundle in @(Get-ChildItem -LiteralPath /Applications -Directory -Filter 'Xcode*.app' | Sort-Object FullName)) {
            $developer = Join-Path $bundle.FullName Contents/Developer
            if (-not (Test-Path -LiteralPath $developer -PathType Container)) { continue }
            $env:DEVELOPER_DIR = $developer
            $output = (& /usr/bin/xcodebuild -version 2>&1 | Out-String).Trim()
            $code = $LASTEXITCODE
            $matched = $code -eq 0 -and $output -cmatch '^Xcode ([0-9.]+)\s+Build version ([A-Za-z0-9]+)$'
            $version = if ($matched) { $Matches[1] } else { $null }
            $build = if ($matched) { $Matches[2] } else { $null }
            $installedVersion = if ($matched -and -not $version.Contains('.')) { $version+'.0' } else { $version }
            $observations.Add([pscustomobject]@{developer_dir=$developer;exit=$code;identity=$output;version=$installedVersion;build=$build})
        }
    } finally { $env:DEVELOPER_DIR = $originalDeveloperDir }
    $choice = Select-InstalledAppleToolchain @($observations.ToArray()) $release $policy $preferred
    $selected = if ($choice) { $choice.developer_dir } else { $null }
    $releaseVersion = if ($release) { if ($release.version.Contains('.')) { $release.version } else { $release.version+'.0' } } else { $null }
    $observationPath = Join-Path $InputRoot xcode-selection.json
    Json $observationPath ([ordered]@{
        layout_version=1;release=(Artifact $feedPath);expected=$release
        installed=@($observations.ToArray());selected_developer_dir=$selected;policy=$policy
        selected=$choice;release_lookup=$lookup
        matches_latest_formal=($choice -and $release -and [version]$choice.version -eq [version]$releaseVersion -and $choice.build -ceq $release.build)
        scope='Actual Xcode version/build selection. CI preinstalled policy does not require a match to latest formal; this does not relabel an RC or unknown build. SDK use, linkage, runtime capture and callback ownership require native checks.'
    })
    $inputs.Add((Artifact $observationPath))
    $log = Join-Path $logsRoot xcode-selection.log
    "policy=$policy`nexpected=$($release | ConvertTo-Json -Compress)`nselected=$selected" | Set-Content -LiteralPath $log -Encoding utf8
    $status = if ($selected) { 'passed' } else { 'missing_environment' }
    $checks.Add([ordered]@{name='xcode-selection';status=$status;log=(Artifact $log)})
    if (-not $selected) {
        if ($policy -eq 'preinstalled') { throw 'No usable installed Xcode was identified on this native runner' }
        throw "Latest formal $($release.title) is not installed; preserve this missing-environment result and provision a matching native runner"
    }
    $env:DEVELOPER_DIR = $selected
    $sdk = (& /usr/bin/xcrun --sdk macosx --show-sdk-path | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $sdk -PathType Container)) { throw 'Selected Xcode has no usable macOS SDK' }
    $env:SDKROOT = $sdk
}
