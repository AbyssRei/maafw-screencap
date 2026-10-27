# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later

# Shared by the two native Linux probes. Loading this file performs no work;
# initialization requires a native x64 Linux process and the caller's evidence
# functions (Step and Save). No system compiler or global SDK is replaced.
function Select-LinuxLlvmRelease($Release,[bool]$ZstdAvailable) {
    if($Release.draft -or $Release.prerelease -or $Release.tag_name -notmatch '^llvmorg-(\d+\.\d+\.\d+)$') {
        throw 'LLVM preparation requires a formal maintainer release'
    }
    $version=$Matches[1]
    $extension=if($ZstdAvailable){'.tar.zst'}else{'.tar.xz'}
    $name="LLVM-$version-Linux-X64$extension"
    $assets=@($Release.assets|Where-Object name -EQ $name)
    if($assets.Count -ne 1 -or $assets[0].digest -notmatch '^sha256:[0-9a-f]{64}$') {
        throw 'Formal LLVM release lacks one integrity-addressed native Linux x64 package'
    }
    if($assets[0].browser_download_url -notlike 'https://github.com/llvm/llvm-project/releases/download/*'){throw 'LLVM asset URL is outside the maintainer release origin'}
    return [pscustomobject]@{version=$version;asset=$assets[0]}
}

function Initialize-LinuxLlvm([string]$ReferenceRoot,[string]$DepsRoot,[string]$RunRoot,[switch]$BindingGenerator) {
    if(-not $IsLinux -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64') {
        throw 'LLVM native preparation requires Linux x64'
    }
    Write-Host 'Resolving the latest formal LLVM compiler and libclang ...'
    $inputRoot=Join-Path $RunRoot inputs
    $releaseText=& gh api repos/llvm/llvm-project/releases/latest
    if($LASTEXITCODE -ne 0){throw 'LLVM release lookup failed; cached versions cannot replace the online check'}
    $releasePath=Join-Path $inputRoot llvm-release.json
    $releaseStream=[IO.File]::Open($releasePath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write)
    try{$releaseStream.Write([Text.Encoding]::UTF8.GetBytes(($releaseText|Out-String)))}finally{$releaseStream.Dispose()}
    Save $releasePath
    $selection=Select-LinuxLlvmRelease ($releaseText|ConvertFrom-Json) ([bool](Get-Command zstd -ErrorAction SilentlyContinue))
    $asset=$selection.asset
    $digest=$asset.digest.Substring('sha256:'.Length)
    $downloads=Join-Path $DepsRoot downloads
    New-Item -ItemType Directory -Path $downloads -Force|Out-Null
    $archive=Join-Path $downloads $asset.name
    if(-not(Test-Path -LiteralPath $archive)) {
        Step llvm-download curl @('--fail','--location','--proto','=https','--connect-timeout','15','--max-time','900','-o',$archive,$asset.browser_download_url)
    }
    if((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $digest) {
        throw 'LLVM package digest differs from the current maintainer release'
    }
    # Reuse an immutable reference copy from this native job when the second
    # backend resolves the same release, retaining an explicit artifact link.
    $savedArchive=$null
    $index=Get-Content -LiteralPath (Join-Path $ReferenceRoot manifest.json) -Raw|ConvertFrom-Json
    foreach($entry in $index.runs) {
        $run=Get-Content -LiteralPath (Join-Path $ReferenceRoot $entry.path) -Raw|ConvertFrom-Json
        foreach($artifact in $run.inputs) {
            if($artifact.path.EndsWith('/'+$asset.name) -and $artifact.sha256 -eq $digest) {
                $candidate=Join-Path $ReferenceRoot $artifact.path
                if((Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant() -ne $digest){throw 'Archived LLVM input changed'}
                $savedArchive=$candidate
                break
            }
        }
        if($savedArchive){break}
    }
    if(-not $savedArchive) {
        $savedArchive=Join-Path $inputRoot $asset.name
        Copy-Item -LiteralPath $archive -Destination $savedArchive
    }
    Save $savedArchive
    $selectionProfile='c-native-bindgen-v1'
    $toolRoot=Join-Path $DepsRoot ('tools/llvm-'+$selection.version+'-'+$digest.Substring(0,16)+'-'+$selectionProfile)
    $identityPath=Join-Path $toolRoot mfs-tool-identity.json
    if(-not(Test-Path -LiteralPath $toolRoot)) {
        $stage=$toolRoot+'-stage-'+[Guid]::NewGuid().ToString('N')
        $depsAbsolute=[IO.Path]::GetFullPath($DepsRoot).TrimEnd('/')+'/'
        foreach($path in @($toolRoot,$stage)) {
            if(-not [IO.Path]::GetFullPath($path).StartsWith($depsAbsolute,[StringComparison]::Ordinal)) {throw 'LLVM installation escaped the dependency cache'}
        }
        New-Item -ItemType Directory -Path $stage|Out-Null
        # Official LLVM zstd packages use a long window exceeding zstd's
        # default 128 MiB decoder limit. Stream through an explicit bounded
        # decoder for both inventory and extraction; do not expand a full tar.
        # Keep even one argument as an array: PowerShell otherwise unwraps
        # the conditional output and splats a scalar string as characters.
        [string[]]$decoder=if($asset.name.EndsWith('.tar.zst')){@('--use-compress-program=zstd --memory=2048MB')}else{@()}
        $entries=@(& tar @decoder -tf $archive)
        if($LASTEXITCODE -ne 0){throw 'Cannot inspect the official LLVM package'}
        $allMembers=Join-Path $inputRoot llvm-archive-members.txt
        $entries|Set-Content -LiteralPath $allMembers -Encoding utf8NoBOM
        Save $allMembers
        foreach($entry in $entries) {
            if($entry.StartsWith('/') -or $entry -match '(^|/)\.\.(/|$)' -or $entry.Contains([char]0)) {throw 'Unsafe LLVM archive member'}
        }
        $clean=@($entries|ForEach-Object{$_ -replace '^(\./)+',''})
        $roots=@($clean|ForEach-Object{($_ -split '/')[0]}|Sort-Object -Unique)
        $strip=if($roots.Count -eq 1 -and $roots[0] -notin @('bin','lib','include','share')){1}else{0}
        $selected=[Collections.Generic.List[string]]::new()
        foreach($entry in $entries) {
            if($entry.EndsWith('/')){continue}
            $relative=$entry -replace '^(\./)+',''
            if($strip){$relative=($relative -split '/',2)[1]}
            if($relative -match '^bin/(clang(\+\+)?(-\d+)?|llvm-config(-\d+)?|lld(-\d+)?|ld\.lld(-\d+)?|llvm-ar(-\d+)?|llvm-ranlib(-\d+)?)$' -or
               $relative -match '^lib/(libLLVM[^/]*|libclang[^/]*)\.so' -or
               $relative.StartsWith('lib/clang/') -or
               $relative -match '^lib/([^/]+/)?lib(c\+\+|c\+\+abi|unwind)\.so' -or
               $relative -match '(^|/)(LICENSE|COPYRIGHT|NOTICE)[^/]*$') {$selected.Add($entry)}
        }
        if($selected.Count -eq 0){throw 'LLVM package has no supported compiler layout'}
        $members=Join-Path $inputRoot llvm-package-members.txt
        $selected|Set-Content -LiteralPath $members -Encoding utf8NoBOM
        Save $members
        Step llvm-unpack tar ($decoder+@('-xf',$archive,'-C',$stage,'--no-same-owner','--no-same-permissions','--no-recursion',"--strip-components=$strip",'--verbatim-files-from','-T',$members))
        foreach($name in @('clang','clang++','llvm-config','ld.lld','llvm-ar','llvm-ranlib')) {
            if(-not(Test-Path -LiteralPath (Join-Path $stage "bin/$name") -PathType Leaf)){throw "LLVM package misses $name"}
        }
        $files=@(Get-ChildItem -LiteralPath $stage -Recurse -File|Sort-Object FullName|ForEach-Object{
            [ordered]@{path=[IO.Path]::GetRelativePath($stage,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
        })
        $identity=[ordered]@{version=$selection.version;asset_digest=$asset.digest;selection_profile=$selectionProfile;files=$files}
        [IO.File]::WriteAllText((Join-Path $stage mfs-tool-identity.json),($identity|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $stage -Destination $toolRoot
    }
    $identity=Get-Content -LiteralPath $identityPath -Raw|ConvertFrom-Json
    if($identity.asset_digest -ne $asset.digest -or $identity.version -ne $selection.version -or $identity.selection_profile -ne $selectionProfile){throw 'LLVM cache identity differs from its release/profile'}
    foreach($file in $identity.files) {
        if($file.path.StartsWith('/') -or $file.path -match '(^|/)\.\.(/|$)'){throw 'LLVM cache manifest escaped its installation'}
        $path=Join-Path $toolRoot $file.path
        if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256){throw 'LLVM compiler/resource cache has changed'}
    }
    $savedIdentity=Join-Path $inputRoot llvm-tool-identity.json
    Copy-Item -LiteralPath $identityPath -Destination $savedIdentity
    Save $savedIdentity
    $libclangPath=$null
    if($BindingGenerator) {
        $libclang=@(Get-ChildItem -LiteralPath (Join-Path $toolRoot lib) -Recurse -File|Where-Object{$_.Name -match '^libclang(?:-[0-9]+)?\.so' -and $_.Name -notmatch 'cpp'}|Sort-Object FullName|Select-Object -First 1)
        if($libclang.Count -ne 1){throw 'Formal LLVM package has no libclang C API library'}
        $libclangPath=$libclang[0].FullName
        $env:LIBCLANG_PATH=Split-Path -Parent $libclangPath
    }
    $env:CC=Join-Path $toolRoot bin/clang
    $env:CXX=Join-Path $toolRoot bin/clang++
    $env:AR=Join-Path $toolRoot bin/llvm-ar
    $env:RANLIB=Join-Path $toolRoot bin/llvm-ranlib
    $env:LLVM_CONFIG_PATH=Join-Path $toolRoot bin/llvm-config
    $env:PATH=(Join-Path $toolRoot bin)+':'+$env:PATH
    $env:LDFLAGS=($env:LDFLAGS+' -fuse-ld=lld').Trim()
    $env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$env:CC
    [string[]]$llvmRustFlags=if($env:CARGO_ENCODED_RUSTFLAGS){@($env:CARGO_ENCODED_RUSTFLAGS.Split([char]31))}elseif($env:RUSTFLAGS){@($env:RUSTFLAGS -split '\s+'|Where-Object{$_})}else{@()}
    $llvmRustFlags+=@('-C',"linker=$($env:CC)",'-C','link-arg=-fuse-ld=lld')
    $env:CARGO_ENCODED_RUSTFLAGS=[string]::Join([char]31,[string[]]$llvmRustFlags)
    Step llvm-compiler-identity $env:CC @('--version')
    Step llvm-linker-identity (Join-Path $toolRoot bin/ld.lld) @('--version')
    $version=(& $env:LLVM_CONFIG_PATH --version|Out-String).Trim()
    if($LASTEXITCODE -ne 0 -or $version -ne $selection.version){throw 'Prepared LLVM reports a different formal version'}
    $configuration=Join-Path $inputRoot llvm-toolchain.json
    Json $configuration ([ordered]@{layout_version=1;version=$version;release=(Artifact $releasePath);package=(Artifact $savedArchive);tool_identity=(Artifact $savedIdentity);asset_digest=$asset.digest;selection_profile=$selectionProfile;installation=$toolRoot;binding_generator=[bool]$BindingGenerator;libclang=$libclangPath;cc=$env:CC;cxx=$env:CXX;linker=(Join-Path $toolRoot bin/ld.lld);ldflags=$env:LDFLAGS;rustflags=$llvmRustFlags;scope='Native Linux C compiler/linker and explicitly requested binding generator; host OS/SDK and other platforms require their own checks.'})
    Save $configuration
}
