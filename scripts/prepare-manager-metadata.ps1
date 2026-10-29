# Copyright (C) 2026 AbyssRei
# SPDX-License-Identifier: LGPL-3.0-or-later
# Explicit model artifacts only. The owning batch has prepared latest meojson
# and selected its native compiler; Step/Save and recording belong to that batch.
function Save-ManagerMetadataReference([string]$BuildDirectory,[string]$RecordDirectory,[string]$OutputDirectory) {
    $program=Join-Path $BuildDirectory ($IsWindows ? 'Release/mfs_manager_metadata_model.exe' : 'mfs_manager_metadata_model')
    $savedMetadata=Join-Path $RecordDirectory (Split-Path -Leaf $program)
    Copy-Item -LiteralPath $program -Destination $savedMetadata;Save $savedMetadata
    foreach($relative in @('win32-info-source.inc','win32-info-members.inc','macos-info-source.inc','macos-info-members.inc')) {
        $savedMetadata=Join-Path $RecordDirectory $relative
        Copy-Item -LiteralPath (Join-Path $BuildDirectory $relative) -Destination $savedMetadata;Save $savedMetadata
    }
    $env:MFS_P1_MANAGER_METADATA_MODEL=$program
    $env:MFS_P1_MANAGER_METADATA_ARTIFACTS=$OutputDirectory
}
