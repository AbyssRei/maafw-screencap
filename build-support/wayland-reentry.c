// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#include "wayland-reentry.h"
// wayland-sys currently omits this native getter. Keep the gap to this one
// header-checked call; connection, protocol and roundtrip APIs use bindings.
struct wl_event_queue *mfs_wayland_proxy_get_queue(const struct wl_proxy *proxy)
{
    return wl_proxy_get_queue(proxy);
}
