// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include "gdi-parity.h"

typedef struct MfsPrintFixture {
    void* owner;
    void* hwnd;
    void* popup;
    int32_t width;
    int32_t height;
    int32_t popup_width;
    int32_t popup_height;
} MfsPrintFixture;
// Acceptance fixture only: disable DWM transitions before first visibility.
// Existing PrintWindow modes 0/1/2 retain their original setup. This flag is
// restricted to the no-popup top-level ScreenDC scene, never product capture.
#define MFS_RENDER_DISABLE_TRANSITIONS 0x100u
// Independent visible-scene fixture premise; does not grant foreground rights.
#define MFS_RENDER_TOPMOST 0x200u

#ifdef __cplusplus
extern "C" {
#endif
MFS_PARITY_EXPORT int32_t mfs_print_parity_reference(void* hwnd, MfsParityImage* output);
// Fixture UI owner only. Change its next draw's palette without drawing or
// altering capture. Zero preserves the original default scene.
MFS_PARITY_EXPORT int32_t mfs_print_parity_scene(void* owner, uint32_t alternative);
// Image owners originate in this module; destination is exclusively borrowed.
MFS_PARITY_EXPORT int32_t mfs_print_parity_overlay(void* destination, void* source,
    int32_t source_x, int32_t source_y, int32_t destination_x, int32_t destination_y,
    int32_t width, int32_t height);
// mode: 0 no popup, 1 visible last-active owned popup, 2 hidden owned popup.
// Even a failed create may publish a partial owner: it must be closed exactly
// once on the caller thread while retaining this module, including on failure.
MFS_PARITY_EXPORT int32_t mfs_print_parity_fixture(int32_t width, int32_t height,
    int32_t popup_width, int32_t popup_height, int32_t offset_x, int32_t offset_y,
    uint32_t mode, MfsPrintFixture* output);
// Same target dimensions, with hwnd now a real child at (7,5) inside a larger
// root. The optional popup remains owned by the root; offsets are child-relative.
// Opaque owner/close and all partial-failure responsibilities are unchanged.
MFS_PARITY_EXPORT int32_t mfs_print_parity_child_fixture(int32_t width, int32_t height,
    int32_t popup_width, int32_t popup_height, int32_t offset_x, int32_t offset_y,
    uint32_t mode, MfsPrintFixture* output);
// Failure leaves a partial owner alive. Do not free its context/unload code.
MFS_PARITY_EXPORT int32_t mfs_print_parity_close(void** owner);
// First 11 observation slots retain their existing meaning. Capacity >=12
// additionally returns whether this fixture disabled transitions before show;
// capacity >=13 adds whether its owned root requested the topmost scene flag.
MFS_PARITY_EXPORT int32_t mfs_print_parity_stats(void* owner, uint64_t* output, size_t capacity);
// The nine layout slots are unchanged; capacity >=10 adds the transitions flag,
// and capacity >=11 adds the independent topmost-scene flag.
MFS_PARITY_EXPORT int32_t mfs_print_parity_layout(uint64_t* output, size_t capacity);
// 0: nonempty, 1: empty, 2: invalid dimensions/offset. Six int32 outputs are
// source x/y, destination x/y, width/height. This uses the source cv::Rect path.
MFS_PARITY_EXPORT int32_t mfs_print_parity_intersection(int32_t canvas_width, int32_t canvas_height,
    int32_t popup_width, int32_t popup_height, int64_t offset_x, int64_t offset_y,
    int32_t* output, size_t capacity);
// Diagnostic source interposition, distinct from pristine reference capture.
// mode 0 observes source reads; mode 1 unselects only around its first read.
// 36 u64 values: read count, overflow, mode, DC count, then two 16-value reads.
MFS_PARITY_EXPORT int32_t mfs_print_probe_reference(void* hwnd, uint32_t mode,
    MfsParityImage* image, uint64_t* observations, size_t capacity);
// Conversion-only source interposition. fail_at is 1 (main) or 2 (popup).
// An unreached fault returns 4 and destroys the unexpected successful image.
// Twelve u64 values: call count, GDI baseline, main/popup conversion GDI counts,
// GDI count after capture/unwind, exported image owners, unexpected-call flag,
// BGR allocations/releases/bytes/freed bytes, allocator error/outstanding flags.
// Reject overlap/reentry. Fault modes use a per-output module-owned allocator
// and never export its Mat. If outstanding bit 2 is reported, retain the module
// for any late release; do not claim successful cleanup. Input
// BGRA/default allocator allocations are outside these BGR-output counters.
MFS_PARITY_EXPORT int32_t mfs_print_conversion_reference(void* hwnd, uint32_t fail_at,
    MfsParityImage* image, uint64_t* observations, size_t capacity);
#ifdef __cplusplus
}
#endif
