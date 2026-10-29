// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#pragma once
#include <wayland-client-core.h>
// Private opaque-pointer ABI. Caller retains a live proxy on its owning
// connection/channel. No ownership transfers and no event is dispatched here.
struct wl_event_queue *mfs_wayland_proxy_get_queue(const struct wl_proxy *proxy);
