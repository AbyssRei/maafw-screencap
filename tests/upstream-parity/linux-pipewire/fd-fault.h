// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Private native acceptance controls, loaded only into explicit fault processes.
#pragma once
#include <stdint.h>
#define MFS_PW_FD_EXPORT __attribute__((visibility("default")))
#ifdef __cplusplus
#define MFS_PW_FD_NOEXCEPT noexcept
extern "C" {
#else
#define MFS_PW_FD_NOEXCEPT
#endif
// Arms one rejection of this exact socket's native loop add_io. Calling channel
// must own the not-yet-running context; no concurrent init/close is permitted.
MFS_PW_FD_EXPORT int32_t mfs_pw_fd_fault_arm(int32_t fd) MFS_PW_FD_NOEXCEPT;
MFS_PW_FD_EXPORT uint32_t mfs_pw_fd_fault_attempts() MFS_PW_FD_NOEXCEPT;
MFS_PW_FD_EXPORT uint32_t mfs_pw_fd_fault_rejections() MFS_PW_FD_NOEXCEPT;
MFS_PW_FD_EXPORT int32_t mfs_pw_fd_fault_native_error() MFS_PW_FD_NOEXCEPT;
MFS_PW_FD_EXPORT int32_t mfs_pw_fd_fault_control_error() MFS_PW_FD_NOEXCEPT;
#ifdef __cplusplus
}
#endif
