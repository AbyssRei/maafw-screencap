// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "desktop-pending.h"
#include "gdi-probe-internal.hpp"
#include "Screencap/DesktopDupWindowScreencap.h"
#include <memory>
#include <new>
int32_t mfs_desktop_pending_create(void* hwnd, void** output) {
    if(!output)return 1;*output=nullptr;
    try {*output=new MAA_CTRL_UNIT_NS::DesktopDupWindowScreencap(static_cast<HWND>(hwnd));return 0;}
    catch(const std::bad_alloc&){return 3;}catch(...){return 2;}
}
int32_t mfs_desktop_pending_capture(void* owner,MfsParityImage* output) {
    if(!output)return 1;*output={};if(!owner)return 1;
    try {
        auto image=static_cast<MAA_CTRL_UNIT_NS::DesktopDupWindowScreencap*>(owner)->screencap();
        return image?mfs_parity::transfer_image(image->clone(),output):1;
    }catch(const std::bad_alloc&){return 3;}catch(...){return 2;}
}
int32_t mfs_desktop_pending_destroy(void** owner) {
    if(!owner)return 1;
    auto* capture=static_cast<MAA_CTRL_UNIT_NS::DesktopDupWindowScreencap*>(*owner);
    *owner=nullptr;delete capture;return 0;
}
