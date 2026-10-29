// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private kernel DMA fixture ABI, never an SDK or product runtime dependency.
#pragma once
#include <stdint.h>
#define MFS_DMA_EXPORT __attribute__((visibility("default")))
#ifdef __cplusplus
#define MFS_DMA_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_DMA_NOEXCEPT
#endif
// 1 udmabuf; 2 system DMA heap. Success transfers one real DMA-BUF FD; failure
// returns -errno after releasing all intermediate device/backing descriptors.
MFS_DMA_EXPORT int32_t mfs_pw_dma_create(uint32_t exporter,uint64_t bytes) MFS_DMA_NOEXCEPT;
// Synchronous independent initialized input; caller owns fd. Map, WRITE START, copy, WRITE
// END and unmap. Return 0 or first errno; never consumes fd or input pointer.
MFS_DMA_EXPORT int32_t mfs_pw_dma_write(int32_t fd,const uint8_t* input,uint64_t bytes) MFS_DMA_NOEXCEPT;
// 0 sizeof sync, 1 align, 2 flags offset, 3 ioctl, 4 READ, 5 START, 6 END.
MFS_DMA_EXPORT uint64_t mfs_pw_dma_abi(uint32_t index) MFS_DMA_NOEXCEPT;
#ifdef __cplusplus
}
#endif
