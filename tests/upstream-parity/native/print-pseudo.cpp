// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "print-pseudo.h"
#include "gdi-probe-internal.hpp"
#include "Screencap/PrintWindowWithPseudoMinimizeScreencap.h"
#include <memory>
using Original = MAA_CTRL_UNIT_NS::PrintWindowWithPseudoMinimizeScreencap;
using OriginalHelper = MAA_CTRL_UNIT_NS::PseudoMinimizeHelper;

int32_t mfs_print_pseudo_open(void* hwnd, void** owner)
{
    if (!owner) { return 1; }
    *owner = nullptr;
    try { *owner = new Original(static_cast<HWND>(hwnd)); return 0; }
    catch (const std::bad_alloc&) { return 2; }
    catch (...) { return 3; }
}
int32_t mfs_print_pseudo_capture(void* owner, MfsParityImage* image)
{
    if (!image) { return 1; }
    *image = {};
    if (!owner) { return 1; }
    try {
        auto pixels = static_cast<Original*>(owner)->screencap();
        if (!pixels) { return 4; }
        return mfs_parity::transfer_image(std::move(*pixels), image);
    }
    catch (const std::bad_alloc&) { return 2; }
    catch (...) { return 3; }
}
int32_t mfs_print_pseudo_inactive(void* owner)
{
    if (!owner) { return 1; }
    try { static_cast<Original*>(owner)->inactive(); return 0; }
    catch (...) { return 3; }
}
int32_t mfs_print_pseudo_close(void** owner)
{
    if (!owner) { return 1; }
    // The upstream destructor synchronously stops/joins the monitor. No retry
    // or timeout: completion precedes owner invalidation and module release.
    delete static_cast<Original*>(*owner);
    *owner = nullptr;
    return 0;
}
int32_t mfs_pseudo_helper_open(void* hwnd, void** owner)
{
    if (!owner) { return 1; }
    *owner = nullptr;
    try {
        auto helper = std::make_unique<OriginalHelper>(static_cast<HWND>(hwnd));
        helper->start();
        *owner = helper.release();
        return 0;
    }
    catch (const std::bad_alloc&) { return 2; }
    catch (...) { return 3; }
}
int32_t mfs_pseudo_helper_ensure(void* owner)
{
    if (!owner) { return 1; }
    try { static_cast<OriginalHelper*>(owner)->ensure_not_minimized(); return 0; }
    catch (...) { return 3; }
}
int32_t mfs_pseudo_helper_status(void* owner, uint32_t* pseudo)
{
    if (!pseudo) { return 1; }
    *pseudo = 0;
    if (!owner) { return 1; }
    *pseudo = static_cast<OriginalHelper*>(owner)->is_pseudo_minimized() ? 1u : 0u;
    return 0;
}
int32_t mfs_pseudo_helper_close(void** owner)
{
    if (!owner) { return 1; }
    delete static_cast<OriginalHelper*>(*owner);
    *owner = nullptr;
    return 0;
}
