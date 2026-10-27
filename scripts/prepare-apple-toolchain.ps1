# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later

# Loading this file only defines functions. Selection uses Apple's formal
# release identity, including the build number, rather than an app filename.
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

function Initialize-AppleToolchain([string]$InputRoot) {
    if (-not $IsMacOS) { throw 'Xcode selection requires native macOS' }
    $feedPath = Join-Path $InputRoot apple-releases.rss
    Step apple-releases curl @('--fail','--location','--silent','--show-error','--connect-timeout','15','--max-time','60','-o',$feedPath,'https://developer.apple.com/news/releases/rss/releases.rss')
    $inputs.Add((Artifact $feedPath))
    $release = Select-AppleXcodeRelease ([xml](Get-Content -LiteralPath $feedPath -Raw))
    $observations = [Collections.Generic.List[object]]::new()
    $selected = $null
    $originalDeveloperDir = $env:DEVELOPER_DIR
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
            $observations.Add([ordered]@{developer_dir=$developer;exit=$code;identity=$output})
            $expectedVersion = if ($release.version.Contains('.')) { $release.version } else { $release.version+'.0' }
            $installedVersion = if ($version -and $version.Contains('.')) { $version } else { "$version.0" }
            if ($matched -and [version]$installedVersion -eq [version]$expectedVersion -and $build -ceq $release.build -and -not $selected) {
                $selected = $developer
            }
        }
    } finally { $env:DEVELOPER_DIR = $originalDeveloperDir }
    $observationPath = Join-Path $InputRoot xcode-selection.json
    Json $observationPath ([ordered]@{
        layout_version=1;release=(Artifact $feedPath);expected=$release
        installed=@($observations.ToArray());selected_developer_dir=$selected
        scope='Formal Xcode version/build selection only. SDK use, linkage, runtime capture and callback ownership require the native checks.'
    })
    $inputs.Add((Artifact $observationPath))
    $log = Join-Path $logsRoot xcode-selection.log
    "expected=$($release.title)`nselected=$selected" | Set-Content -LiteralPath $log -Encoding utf8
    $status = if ($selected) { 'passed' } else { 'missing_environment' }
    $checks.Add([ordered]@{name='xcode-selection';status=$status;log=(Artifact $log)})
    if (-not $selected) { throw "Latest formal $($release.title) is not installed; preserve this missing-environment result and provision a matching native runner" }
    $env:DEVELOPER_DIR = $selected
    $sdk = (& /usr/bin/xcrun --sdk macosx --show-sdk-path | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $sdk -PathType Container)) { throw 'Selected Xcode has no usable macOS SDK' }
    $env:SDKROOT = $sdk
}
